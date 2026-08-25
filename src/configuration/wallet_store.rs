use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::configuration::block_error;

pub const SCHEMA_VERSION: u32 = 1;
const FILE_VERSION: u32 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const M_KIB: u32 = 19_456;
const T_COST: u32 = 2;
const P_COST: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KdfParams {
    pub algo: String,
    pub salt: String,
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreFile {
    pub version: u32,
    pub kdf: KdfParams,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayloadV1 {
    pub schema: u32,
    pub mnemonic: Option<String>,
    #[serde(default)]
    pub passphrase: Option<String>,
    pub settings: StoreSettings,
    pub btc: Vec<BtcRecord>,
    pub eth: Vec<EthRecord>,
    #[serde(default)]
    pub sol: Vec<SolRecord>,
    #[serde(default)]
    pub ltc: Vec<LtcRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreSettings {
    pub starred: Vec<String>,
    pub infura_key: String,
    pub etherscan_key: String,
    pub btc_node: String,
    pub eth_node: String,
    #[serde(default)]
    pub sol_node: String,
    #[serde(default)]
    pub ltc_node: String,
    #[serde(default)]
    pub btc_network: String,
    #[serde(default)]
    pub eth_network: String,
    #[serde(default)]
    pub sol_network: String,
    #[serde(default)]
    pub ltc_network: String,
    #[serde(default)]
    pub custom_tokens: Vec<CustomTokenRecord>,
    #[serde(default)]
    pub lock_timeout_secs: u32,
    #[serde(default)]
    pub show_prices: bool,
    #[serde(default)]
    pub fiat: String,
    #[serde(default)]
    pub btc_units: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomTokenRecord {
    pub symbol: String,
    pub name: String,
    pub address: String,
    pub decimals: i32,
    /// "btc" | "eth" | "sol" | "ltc". Empty on records written before Solana support existed;
    /// those all predate anything but ERC-20 custom tokens, so they default to "eth".
    #[serde(default)]
    pub chain: String,
}

impl CustomTokenRecord {
    pub fn chain_or_default(&self) -> String {
        if self.chain.is_empty() {
            "eth".to_string()
        } else {
            self.chain.clone()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BtcRecord {
    pub name: String,
    pub mnemonic: Option<String>,
    pub passphrase: Option<String>,
    pub private_key_wif: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LtcRecord {
    pub name: String,
    pub mnemonic: Option<String>,
    pub passphrase: Option<String>,
    pub private_key_wif: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EthRecord {
    pub name: String,
    pub mnemonic: Option<String>,
    pub path: Option<String>,
    pub private_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SolRecord {
    pub name: String,
    pub mnemonic: Option<String>,
    pub path: Option<String>,
    pub private_key: Option<String>,
}

#[derive(Clone)]
pub struct StoreSession {
    pub path: PathBuf,
    key: Vec<u8>,
    salt: Vec<u8>,
    m_kib: u32,
    t: u32,
    p: u32,
}

impl std::fmt::Debug for StoreSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreSession")
            .field("path", &self.path)
            .field("kdf", &format!("argon2id m={} t={} p={}", self.m_kib, self.t, self.p))
            .finish_non_exhaustive()
    }
}

impl Default for PayloadV1 {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            mnemonic: None,
            passphrase: None,
            settings: StoreSettings {
                starred: Vec::new(),
                infura_key: String::new(),
                etherscan_key: String::new(),
                btc_node: String::new(),
                eth_node: String::new(),
                sol_node: String::new(),
                ltc_node: String::new(),
                btc_network: String::new(),
                eth_network: String::new(),
                sol_network: String::new(),
                ltc_network: String::new(),
                custom_tokens: Vec::new(),
                lock_timeout_secs: 0,
                show_prices: false,
                fiat: String::new(),
                btc_units: String::new(),
            },
            btc: Vec::new(),
            eth: Vec::new(),
            sol: Vec::new(),
            ltc: Vec::new(),
        }
    }
}

impl PayloadV1 {
    pub fn new() -> Self {
        Self::default()
    }
}

fn derive_key(password: &str, salt: &[u8], m_kib: u32, t: u32, p: u32) -> Result<Vec<u8>, block_error::Error> {
    let params = Params::new(m_kib, t, p, Some(KEY_LEN)).map_err(|e| {
        block_error::Error::new(format!("argon2 params: {e}"))
    })?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = vec![0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| block_error::Error::new(format!("argon2id failed: {e}")))?;
    Ok(key)
}

fn encrypt(key: &[u8], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), block_error::Error> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| block_error::Error::new(format!("cipher key: {e}")))?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| block_error::Error::new("encryption failed".to_string()))?;
    Ok((nonce_bytes.to_vec(), ciphertext))
}

fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, block_error::Error> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| block_error::Error::new(format!("cipher key: {e}")))?;
    if nonce.len() != NONCE_LEN {
        return Err(block_error::Error::new("invalid nonce".to_string()));
    }
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| block_error::Error::new("decryption failed".to_string()))
}

fn write_file(path: &Path, session: &StoreSession, payload: &PayloadV1) -> Result<(), block_error::Error> {
    let plaintext = serde_json::to_vec(payload)?;
    let (nonce, ciphertext) = encrypt(&session.key, &plaintext)?;
    let file = StoreFile {
        version: FILE_VERSION,
        kdf: KdfParams {
            algo: "argon2id".to_string(),
            salt: hex::encode(&session.salt),
            m_kib: session.m_kib,
            t: session.t,
            p: session.p,
        },
        nonce: hex::encode(nonce),
        ciphertext: hex::encode(ciphertext),
    };
    let json = serde_json::to_vec_pretty(&file)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json)?;
    Ok(())
}

impl StoreSession {
    pub fn create(path: &Path, password: &str, payload: &PayloadV1) -> Result<Self, block_error::Error> {
        if password.is_empty() {
            return Err(block_error::Error::new("password must not be empty".to_string()));
        }
        let mut salt = vec![0u8; SALT_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        let key = derive_key(password, &salt, M_KIB, T_COST, P_COST)?;
        let session = Self {
            path: path.to_path_buf(),
            key,
            salt,
            m_kib: M_KIB,
            t: T_COST,
            p: P_COST,
        };
        write_file(path, &session, payload)?;
        Ok(session)
    }

    pub fn unlock(path: &Path, password: &str) -> Result<(PayloadV1, Self), block_error::Error> {
        let bytes = fs::read(path)?;
        let file: StoreFile = serde_json::from_slice(&bytes).map_err(|_| {
            block_error::Error::new("wallet store is not schema v1".to_string())
        })?;
        if file.version != FILE_VERSION {
            return Err(block_error::Error::new(format!(
                "unsupported store version {}",
                file.version
            )));
        }
        if file.kdf.algo != "argon2id" {
            return Err(block_error::Error::new("unsupported kdf".to_string()));
        }
        let salt = hex::decode(&file.kdf.salt)
            .map_err(|_| block_error::Error::new("invalid kdf salt".to_string()))?;
        let nonce = hex::decode(&file.nonce)
            .map_err(|_| block_error::Error::new("invalid nonce".to_string()))?;
        let ciphertext = hex::decode(&file.ciphertext)
            .map_err(|_| block_error::Error::new("invalid ciphertext".to_string()))?;
        let key = derive_key(password, &salt, file.kdf.m_kib, file.kdf.t, file.kdf.p)?;
        let plaintext = decrypt(&key, &nonce, &ciphertext)?;
        let payload: PayloadV1 = serde_json::from_slice(&plaintext).map_err(|_| {
            block_error::Error::new("invalid wallet payload".to_string())
        })?;
        if payload.schema != SCHEMA_VERSION {
            return Err(block_error::Error::new(format!(
                "unsupported payload schema {}",
                payload.schema
            )));
        }
        let session = Self {
            path: path.to_path_buf(),
            key,
            salt,
            m_kib: file.kdf.m_kib,
            t: file.kdf.t,
            p: file.kdf.p,
        };
        Ok((payload, session))
    }

    pub fn save(&self, payload: &PayloadV1) -> Result<(), block_error::Error> {
        write_file(&self.path, self, payload)
    }

    pub fn wipe(&mut self) {
        crate::configuration::secrets::wipe_vec(&mut self.key);
        crate::configuration::secrets::wipe_vec(&mut self.salt);
        self.m_kib = 0;
        self.t = 0;
        self.p = 0;
    }
}

impl Drop for StoreSession {
    fn drop(&mut self) {
        self.wipe();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn temp_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("blockwallet-store-{}.json", rand::thread_rng().next_u64()));
        path
    }

    fn sample_payload() -> PayloadV1 {
        PayloadV1 {
            schema: SCHEMA_VERSION,
            mnemonic: Some("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()),
            passphrase: None,
            settings: StoreSettings {
                starred: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string(), "LTC".to_string()],
                infura_key: String::new(),
                etherscan_key: String::new(),
                btc_node: "ssl://localhost:50001".to_string(),
                eth_node: "https://example.invalid".to_string(),
                sol_node: "https://sol.example.invalid".to_string(),
                ltc_node: "https://ltc.example.invalid".to_string(),
                btc_network: "bitcoin".to_string(),
                eth_network: "sepolia".to_string(),
                sol_network: "devnet".to_string(),
                ltc_network: "testnet".to_string(),
                custom_tokens: Vec::new(),
                lock_timeout_secs: 120,
                show_prices: false,
                fiat: "usd".to_string(),
                btc_units: "btc".to_string(),
            },
            btc: vec![BtcRecord {
                name: "btc_wallet".to_string(),
                mnemonic: Some("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()),
                passphrase: None,
                private_key_wif: None,
            }],
            eth: vec![EthRecord {
                name: "eth_wallet".to_string(),
                mnemonic: Some("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()),
                path: Some("m/44'/60'/0'/0/0".to_string()),
                private_key: None,
            }],
            sol: vec![SolRecord {
                name: "sol_wallet".to_string(),
                mnemonic: Some("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()),
                path: Some("m/44'/501'/0'/0'".to_string()),
                private_key: None,
            }],
            ltc: vec![LtcRecord {
                name: "ltc_wallet".to_string(),
                mnemonic: Some("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()),
                passphrase: None,
                private_key_wif: None,
            }],
        }
    }

    #[test]
    fn session_debug_omits_key_bytes() {
        let path = temp_path();
        let session = StoreSession::create(&path, "pw", &PayloadV1::new()).unwrap();
        let rendered = format!("{session:?}");
        assert!(rendered.contains("StoreSession"));
        assert!(!rendered.contains(&hex::encode(&session.key)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn create_unlock_roundtrip() {
        let path = temp_path();
        let payload = sample_payload();
        let session = StoreSession::create(&path, "correct horse", &payload).unwrap();
        let (loaded, _) = StoreSession::unlock(&path, "correct horse").unwrap();
        assert_eq!(loaded, payload);
        let _ = session;
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn wrong_password_fails() {
        let path = temp_path();
        let payload = sample_payload();
        StoreSession::create(&path, "correct horse", &payload).unwrap();
        let err = StoreSession::unlock(&path, "wrong").unwrap_err();
        let _ = fs::remove_file(&path);
        match err {
            block_error::Error::New(message) => assert!(message.contains("decrypt") || message.contains("ERROR")),
            _ => panic!("expected decrypt error, got {err:?}"),
        }
    }

    #[test]
    fn empty_password_rejected() {
        let path = temp_path();
        let err = StoreSession::create(&path, "", &PayloadV1::new()).unwrap_err();
        match err {
            block_error::Error::New(message) => assert!(message.contains("password")),
            _ => panic!("expected password error"),
        }
    }

    #[test]
    fn save_rewrites_ciphertext_with_same_key() {
        let path = temp_path();
        let mut payload = sample_payload();
        let session = StoreSession::create(&path, "pw", &payload).unwrap();
        payload.settings.eth_node = "https://updated.invalid".to_string();
        session.save(&payload).unwrap();
        let (loaded, _) = StoreSession::unlock(&path, "pw").unwrap();
        assert_eq!(loaded.settings.eth_node, "https://updated.invalid");
        let _ = fs::remove_file(&path);
    }
}
