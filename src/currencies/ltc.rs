use colored::*;
use core::{fmt, fmt::Display};
use serde::Serialize;
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv};
use bdk_wallet::bitcoin::hashes::Hash;
use bdk_wallet::bitcoin::secp256k1::Secp256k1;
use bdk_wallet::bitcoin::{NetworkKind, PrivateKey};
use bip39::Mnemonic;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::configuration::*;
use crate::currencies::ltc_chain::{self, LtcHistoryItem, LtcNetwork};

const DEFAULT_LTC_MAINNET_PATH: &str = "m/84'/2'/0'/0/0";
const DEFAULT_LTC_TESTNET_PATH: &str = "m/84'/1'/0'/0/0";

pub fn generate_ltc_hd_wallet() -> Option<LitecoinWallet> {
    match LitecoinWallet::new() {
        Ok(wallet) => Some(wallet),
        Err(_) => {
            crate::configuration::logging::error("litecoin wallet generation failed");
            None
        }
    }
}

pub fn generate_from_mnemonic(mnemonic: &str, passphrase: &str) -> Option<LitecoinWallet> {
    match LitecoinWallet::from_mnemonic(mnemonic, passphrase) {
        Ok(wallet) => Some(wallet),
        Err(_) => {
            crate::configuration::logging::error("litecoin wallet from mnemonic failed");
            None
        }
    }
}

pub fn generate_from_private_key(wif: &str) -> Option<LitecoinWallet> {
    match LitecoinWallet::from_private_key(wif) {
        Ok(wallet) => Some(wallet),
        Err(_) => {
            crate::configuration::logging::error("litecoin wallet from WIF failed");
            None
        }
    }
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct LitecoinWallet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mnemonic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    pub balance: Arc<Mutex<String>>,
    pub history: Arc<Mutex<Vec<LtcHistoryItem>>>,
}

impl LitecoinWallet {
    pub fn new() -> Result<Self, block_error::Error> {
        let mut entropy = vec![0u8; 16];
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy(&entropy)
            .map_err(|e| block_error::Error::new(format!("Mnemonic generation failed: {e:?}")))?;
        Self::from_mnemonic(&mnemonic.to_string(), "")
    }

    pub fn from_mnemonic(mnemonic: &str, passphrase: &str) -> Result<Self, block_error::Error> {
        Self::from_mnemonic_on(mnemonic, passphrase, LtcNetwork::Mainnet)
    }

    pub fn from_mnemonic_on(mnemonic: &str, passphrase: &str, network: LtcNetwork) -> Result<Self, block_error::Error> {
        let parsed = Mnemonic::parse_normalized(mnemonic)
            .map_err(|e| block_error::Error::new(format!("Invalid mnemonic: {e:?}")))?;
        let seed = parsed.to_seed(passphrase);
        let path_str = match network {
            LtcNetwork::Mainnet => DEFAULT_LTC_MAINNET_PATH,
            LtcNetwork::Testnet => DEFAULT_LTC_TESTNET_PATH,
        };
        let secp = Secp256k1::new();
        // NetworkKind::Main here only satisfies Xpriv's type requirement; it has no bearing on
        // the derived key bytes, which are pure BIP32 math. Litecoin-specific encoding (address,
        // WIF) happens separately, below and in ltc_chain.rs — never through this value.
        let master = Xpriv::new_master(NetworkKind::Main, &seed)
            .map_err(|e| block_error::Error::new(format!("BIP32 master key failed: {e:?}")))?;
        let path = DerivationPath::from_str(path_str)
            .map_err(|e| block_error::Error::new(format!("Invalid derivation path: {e:?}")))?;
        let derived = master
            .derive_priv(&secp, &path)
            .map_err(|e| block_error::Error::new(format!("BIP32 derive failed: {e:?}")))?;

        let mut wallet = wallet_from_secret(derived.private_key.secret_bytes(), network)?;
        wallet.mnemonic = Some(mnemonic.to_string());
        wallet.path = Some(path_str.to_string());
        if !passphrase.is_empty() {
            wallet.password = Some(passphrase.to_string());
        }
        Ok(wallet)
    }

    pub fn from_private_key(wif: &str) -> Result<Self, block_error::Error> {
        let (secret, network) = ltc_chain::decode_wif(wif)?;
        wallet_from_secret(secret, network)
    }

    pub fn set_wallet_name(&mut self, name: String) {
        self.wallet_name = Some(name);
    }

    pub fn wipe_secrets(&mut self) {
        crate::configuration::secrets::wipe_optional_string(&mut self.mnemonic);
        crate::configuration::secrets::wipe_optional_string(&mut self.password);
        crate::configuration::secrets::wipe_optional_string(&mut self.private_key);
        crate::configuration::secrets::wipe_optional_string(&mut self.public_key);
    }

}

/// Every clone of a wallet carries its own copy of the mnemonic and private key in freshly
/// allocated `String`s. `lock_store` only reaches the copy the app holds, so without this any
/// other clone — the snapshot a send screen is built from, a temporary passed by value — would
/// hand its plaintext back to the allocator intact, and from there potentially to swap.
impl Drop for LitecoinWallet {
    fn drop(&mut self) {
        self.wipe_secrets();
    }
}

impl LitecoinWallet {
    pub fn generate_qr_address(&self) -> Result<gdk4::Texture, block_error::Error> {
        let address = match &self.address {
            Some(addr) => addr,
            None => "",
        };
        if address.is_empty() {
            return Err(block_error::Error::new("no receive address for QR".to_string()));
        }
        let qrcode = QRBuilder::new(address.to_string())
            .build()
            .map_err(|e| block_error::Error::new(format!("QR encode failed: {e:?}")))?;
        let img = ImageBuilder::default()
            .shape(Shape::RoundedSquare)
            .fit_width(300)
            .to_pixmap(&qrcode);
        let encoded_png = match img.encode_png() {
            Ok(png) => png,
            Err(e) => return Err(block_error::Error::IOError(e.into())),
        };
        let texture = gdk4::Texture::from_bytes(&glib::Bytes::from(&encoded_png))?;
        Ok(texture)
    }
}

/// Builds a wallet from a raw 32-byte secp256k1 secret key. Network-agnostic key math
/// (`bitcoin::PrivateKey`/`PublicKey`/hashing) is reused from the `bitcoin` crate; the address
/// text and WIF encoding are hand-rolled in `ltc_chain.rs` since that crate has no Litecoin
/// network variant.
fn wallet_from_secret(secret: [u8; 32], network: LtcNetwork) -> Result<LitecoinWallet, block_error::Error> {
    let secp = Secp256k1::new();
    let secret_key = bdk_wallet::bitcoin::secp256k1::SecretKey::from_slice(&secret)
        .map_err(|e| block_error::Error::new(format!("invalid secret key: {e:?}")))?;
    let private_key = PrivateKey::new(secret_key, bdk_wallet::bitcoin::Network::Bitcoin);
    let public_key = private_key.public_key(&secp);
    let pubkey_hash = public_key
        .wpubkey_hash()
        .map_err(|_| block_error::Error::new("litecoin key is not compressed".to_string()))?;
    let address = ltc_chain::encode_address(&pubkey_hash.to_byte_array(), network)?;
    let wif = ltc_chain::encode_wif(&secret, network);

    // Field by field: the zeroizing `Drop` on this type rules out struct-update syntax.
    let mut wallet = LitecoinWallet::default();
    wallet.private_key = Some(wif);
    wallet.public_key = Some(hex::encode(public_key.to_bytes()));
    wallet.address = Some(address);
    wallet.network = Some(ltc_chain::network_name(network).to_string());
    wallet.balance = Arc::new(Mutex::new(String::from("Uninitialized")));
    wallet.history = Arc::new(Mutex::new(Vec::new()));
    Ok(wallet)
}

/// Double-SHA256, used by the hand-rolled WIF base58check encoding in `ltc_chain.rs`.
pub(crate) fn double_sha256(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    second.into()
}

#[cfg_attr(tarpaulin, skip)]
impl Display for LitecoinWallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = [
            match &self.wallet_name {
                Some(wallet_name) => format!("      {}          {}\n", "Wallet Name".cyan().bold(), wallet_name),
                _ => "".to_owned(),
            },
            match &self.path {
                Some(path) => format!("      {}                 {}\n", "Path".cyan().bold(), path),
                _ => "".to_owned(),
            },
            match &self.address {
                Some(address) => format!("      {}              {}\n", "Address".cyan().bold(), address),
                _ => "".to_owned(),
            },
            match &self.network {
                Some(network) => format!("      {}              {}\n", "Network".cyan().bold(), network),
                _ => "".to_owned(),
            },
        ]
        .concat();
        let output = output[..output.len() - 1].to_owned();
        write!(f, "\n{}", output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn generate_from_known_mnemonic_is_bech32_ltc() {
        let wallet = generate_from_mnemonic(ABANDON, "").unwrap();
        let address = wallet.address.clone().unwrap();
        assert!(address.starts_with("ltc1q"), "{address}");
        assert_eq!(wallet.path.as_deref(), Some(DEFAULT_LTC_MAINNET_PATH));
        assert!(wallet.private_key.as_deref().unwrap().starts_with('T') || wallet.private_key.as_deref().unwrap().starts_with('6'));
    }

    #[test]
    fn testnet_mnemonic_derives_tltc_address() {
        let wallet = LitecoinWallet::from_mnemonic_on(ABANDON, "", LtcNetwork::Testnet).unwrap();
        let address = wallet.address.clone().unwrap();
        assert!(address.starts_with("tltc1q"), "{address}");
        assert_eq!(wallet.path.as_deref(), Some(DEFAULT_LTC_TESTNET_PATH));
    }

    #[test]
    fn mnemonic_restore_matches() {
        let created = generate_ltc_hd_wallet().unwrap();
        let phrase = created.mnemonic.clone().unwrap();
        let restored = LitecoinWallet::from_mnemonic(&phrase, "").unwrap();
        assert_eq!(created.address, restored.address);
        assert_eq!(created.private_key, restored.private_key);
    }

    #[test]
    fn generate_from_wif_roundtrips_address() {
        let generated = generate_from_mnemonic(ABANDON, "").unwrap();
        let wif = generated.private_key.clone().unwrap();
        let wallet = generate_from_private_key(&wif).unwrap();
        assert_eq!(wallet.address, generated.address);
        assert!(wallet.public_key.is_some());
    }

    #[test]
    fn wipe_secrets_clears_key_material_and_keeps_address() {
        let mut wallet = LitecoinWallet::from_mnemonic(ABANDON, "").unwrap();
        let address = wallet.address.clone();
        assert!(wallet.mnemonic.is_some());
        assert!(wallet.private_key.is_some());
        wallet.wipe_secrets();
        assert!(wallet.mnemonic.is_none());
        assert!(wallet.private_key.is_none());
        assert!(wallet.password.is_none());
        assert!(wallet.public_key.is_none());
        assert_eq!(wallet.address, address);
    }

    #[test]
    fn display_omits_mnemonic_and_private_key() {
        let wallet = LitecoinWallet::from_mnemonic(ABANDON, "").unwrap();
        let key = wallet.private_key.clone().unwrap();
        let rendered = format!("{wallet}");
        assert!(!rendered.contains("abandon"));
        assert!(!rendered.contains(&key));
        assert!(rendered.contains("ltc1q"));
    }
}
