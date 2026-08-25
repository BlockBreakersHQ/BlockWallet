use colored::*;
use core::{fmt, fmt::Display};
use serde::Serialize;
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use bip39::Mnemonic;
use ed25519_dalek::{Signer, SigningKey};
use rand::RngCore;

use crate::configuration::*;
use crate::currencies::sol_chain::SolHistoryItem;

const DEFAULT_SOL_PATH: &str = "m/44'/501'/0'/0'";

pub fn generate_sol_basic_wallet() -> Option<SolanaWallet> {
    match SolanaWallet::new() {
        Ok(sol_wallet) => return Some(sol_wallet),
        Err(_) => {
            crate::configuration::logging::error("solana wallet generation failed");
            return None
        }
    };
}

pub fn generate_sol_hd_wallet() -> Option<SolanaWallet> {
    match SolanaWallet::new_hd(24, DEFAULT_SOL_PATH) {
        Ok(sol_wallet) => return Some(sol_wallet),
        Err(_) => {
            crate::configuration::logging::error("solana HD wallet generation failed");
            return None
        }
    };
}

pub fn generate_from_mnemonic(mnemonic: &str, mut path: &str) -> Option<SolanaWallet> {
    if path.is_empty() {
        path = DEFAULT_SOL_PATH;
    }

    match SolanaWallet::from_mnemonic(mnemonic, path, "") {
        Ok(sol_wallet) => return Some(sol_wallet),
        Err(_) => {
            crate::configuration::logging::error("solana wallet from mnemonic failed");
            return None
        }
    };
}

pub fn generate_from_private_key(private_key: &str) -> Option<SolanaWallet> {
    match SolanaWallet::from_private_key(private_key) {
        Ok(sol_wallet) => return Some(sol_wallet),
        Err(_) => {
            crate::configuration::logging::error("solana wallet from private key failed");
            return None
        }
    }
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct SolanaWallet {
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
    pub spl_balances: Arc<Mutex<HashMap<String, f64>>>,
    pub history: Arc<Mutex<Vec<SolHistoryItem>>>,
}

impl SolanaWallet {
    pub fn new() -> Result<Self, block_error::Error> {
        Self::new_hd(12, DEFAULT_SOL_PATH)
    }

    pub fn new_hd(word_count: u8, path: &str) -> Result<Self, block_error::Error> {
        let entropy_len = match word_count {
            12 => 16,
            15 => 20,
            18 => 24,
            21 => 28,
            24 => 32,
            _ => return Err(block_error::Error::new(format!("Invalid word count provided: {:?}. Valid options are 12, 15, 18, 21, 24", word_count)))
        };
        let mut entropy = vec![0u8; entropy_len];
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy(&entropy)
            .map_err(|e| block_error::Error::new(format!("Mnemonic generation failed: {:?}", e)))?;
        Self::from_mnemonic(&mnemonic.to_string(), path, "")
    }

    pub fn from_mnemonic(mnemonic: &str, path: &str, passphrase: &str) -> Result<Self, block_error::Error> {
        let parsed = Mnemonic::parse_normalized(mnemonic)
            .map_err(|e| block_error::Error::new(format!("Invalid mnemonic: {:?}", e)))?;
        let indexes = crate::currencies::sol_chain::parse_derivation_indexes(path)?;
        let seed = parsed.to_seed(passphrase);
        let secret = slip10_ed25519::derive_ed25519_private_key(&seed, &indexes);

        let mut wallet = wallet_from_secret(secret, Some(mnemonic.to_string()), Some(path.to_string()));
        if !passphrase.is_empty() {
            wallet.password = Some(passphrase.to_string());
        }
        Ok(wallet)
    }

    pub fn from_private_key(private_key: &str) -> Result<Self, block_error::Error> {
        let trimmed = private_key.trim();
        let bytes = bs58::decode(trimmed)
            .into_vec()
            .map_err(|e| block_error::Error::new(format!("Invalid Solana private key: {:?}", e)))?;
        let secret: [u8; 32] = match bytes.len() {
            32 => bytes.try_into().unwrap(),
            // Solana CLI keypair export is the 64-byte secret||public concatenation; the seed is the first 32 bytes.
            64 => bytes[..32].try_into().unwrap(),
            other => return Err(block_error::Error::new(format!(
                "Invalid Solana private key length: {other} bytes (expected 32 or 64)"
            ))),
        };
        Ok(wallet_from_secret(secret, None, None))
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
impl Drop for SolanaWallet {
    fn drop(&mut self) {
        self.wipe_secrets();
    }
}

impl SolanaWallet {
    pub fn generate_qr_address(&self) -> Result<gdk4::Texture, block_error::Error> {
        let address = match &self.address {
            Some(addr) => addr,
            None => ""
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
            Err(e)  => return Err(block_error::Error::IOError(e.into()))
        };

        let texture = gdk4::Texture::from_bytes(&glib::Bytes::from(&encoded_png))?;
        Ok(texture)
    }
}

fn wallet_from_secret(secret: [u8; 32], mnemonic: Option<String>, path: Option<String>) -> SolanaWallet {
    let signing_key = SigningKey::from_bytes(&secret);
    let address = bs58::encode(signing_key.verifying_key().to_bytes()).into_string();

    // Field by field: the zeroizing `Drop` on this type rules out struct-update syntax.
    let mut wallet = SolanaWallet::default();
    wallet.mnemonic = mnemonic;
    wallet.private_key = Some(bs58::encode(secret).into_string());
    wallet.public_key = Some(address.clone());
    wallet.address = Some(address);
    wallet.path = path;
    wallet
}

pub(crate) fn signing_key_from_base58(private_key: &str) -> Result<SigningKey, block_error::Error> {
    let bytes = bs58::decode(private_key.trim())
        .into_vec()
        .map_err(|e| block_error::Error::new(format!("Invalid Solana private key: {:?}", e)))?;
    let secret: [u8; 32] = match bytes.len() {
        32 => bytes.try_into().unwrap(),
        64 => bytes[..32].try_into().unwrap(),
        other => return Err(block_error::Error::new(format!(
            "Invalid Solana private key length: {other} bytes (expected 32 or 64)"
        ))),
    };
    Ok(SigningKey::from_bytes(&secret))
}

pub(crate) fn sign_message(signing_key: &SigningKey, message: &[u8]) -> [u8; 64] {
    signing_key.sign(message).to_bytes()
}

#[cfg_attr(tarpaulin, skip)]
impl Display for SolanaWallet {
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

    #[test]
    fn generate_from_known_mnemonic() {
        let wallet = generate_from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "",
        )
        .unwrap();
        let address = wallet.address.clone().unwrap();
        // Base58 Solana addresses are 32-44 chars; never 0x-prefixed hex.
        assert!(address.len() >= 32 && address.len() <= 44);
        assert!(!address.starts_with("0x"));
        assert!(wallet.private_key.is_some());
        assert!(wallet.mnemonic.is_some());
    }

    #[test]
    fn same_mnemonic_derives_same_address_deterministically() {
        let a = generate_from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "",
        )
        .unwrap();
        let b = generate_from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "",
        )
        .unwrap();
        assert_eq!(a.address, b.address);
    }

    #[test]
    fn different_mnemonic_derives_different_address() {
        let a = generate_from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "",
        )
        .unwrap();
        let b = generate_sol_hd_wallet().unwrap();
        assert_ne!(a.address, b.address);
    }

    #[test]
    fn generate_from_private_key_roundtrips_address() {
        let generated = generate_sol_hd_wallet().unwrap();
        let key = generated.private_key.clone().unwrap();
        let wallet = generate_from_private_key(&key).unwrap();
        assert_eq!(wallet.address, generated.address);
        assert!(wallet.public_key.is_some());
    }

    #[test]
    fn wipe_secrets_clears_key_material_and_keeps_address() {
        let mut wallet = SolanaWallet::from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            DEFAULT_SOL_PATH,
            "",
        )
        .unwrap();
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
        let wallet = SolanaWallet::from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            DEFAULT_SOL_PATH,
            "",
        )
        .unwrap();
        let key = wallet.private_key.clone().unwrap();
        let rendered = format!("{wallet}");
        assert!(!rendered.contains("abandon"));
        assert!(!rendered.contains(&key));
    }
}
