use wagyu_ethereum::*;
use wagyu_model::Mnemonic;
use wagyu_model::mnemonic::MnemonicCount;
use wagyu_model::MnemonicExtended;
use wagyu_model::ExtendedPrivateKey;
use wagyu_model::ExtendedPublicKey;
use wagyu_model::PublicKey;
use wagyu_model::PrivateKey;
use colored::*;
use core::{fmt, fmt::Display, str::FromStr};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Serialize};
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;

use crate::configuration::*;
use crate::block_error;

pub fn generate_eth_hd_wallet() -> Option<EthereumWallet> {
    match EthereumWallet::new_hd::<wagyu_ethereum::network::Mainnet, wagyu_ethereum::wordlist::English, _>(
        &mut StdRng::from_entropy(),
        24,
        None,
        "m/44'/60'/0'/0'/0",
    ) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    };
}

pub fn generate_from_mnemonic(mnemonic: &str, mut path: &str) -> Option<EthereumWallet> {
    if path.is_empty() {
        path = "m/44'/60'/0'/0'/0";
    }

    match EthereumWallet::from_mnemonic::<wagyu_ethereum::network::Mainnet, wagyu_ethereum::wordlist::English>(
        mnemonic,
        None,
        path
    ) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    };
}

pub fn generate_from_private_key(private_key: &str) -> Option<EthereumWallet> {
    match EthereumWallet::from_private_key(private_key) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    }
}

pub fn generate_from_extended_private_key(extended_private_key: &str, path: &str) -> Option<EthereumWallet> {
    let path_option;
    if path.is_empty() {
        path_option = Some(String::from("m/44'/60'/0'/0'/0"));
    }
    else {
        path_option = Some(String::from(path));
    }

    match EthereumWallet::from_extended_private_key::<wagyu_ethereum::network::Mainnet>(extended_private_key, &path_option) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    }
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct EthereumWallet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mnemonic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<String>,
}

impl EthereumWallet {
    pub fn new<R: Rng>(rng: &mut R) -> Result<Self, block_error::Error> {
        let private_key = EthereumPrivateKey::new(rng)?;
        let public_key = private_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            ..Default::default()
        })
    }

    pub fn new_hd<N: EthereumNetwork, W: EthereumWordlist, R: Rng>(
        rng: &mut R,
        word_count: u8,
        password: Option<&str>,
        path: &str,
    ) -> Result<Self, block_error::Error> {
        let mnemonic = EthereumMnemonic::<N, W>::new_with_count(rng, word_count)?;
        let master_extended_private_key = mnemonic.to_extended_private_key(password)?;
        let derivation_path = EthereumDerivationPath::from_str(path)?;
        let extended_private_key = master_extended_private_key.derive(&derivation_path)?;
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        //let balance = EthereumWallet::get_balance(address.to_string());
        //block_on(balance);
        Ok(Self {
            path: Some(path.to_string()),
            password: password.map(String::from),
            mnemonic: Some(mnemonic.to_string()),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            //balance: Some(balance.to_string()),
            ..Default::default()
        })
    }

    pub fn from_mnemonic<N: EthereumNetwork, W: EthereumWordlist>(
        mnemonic: &str,
        password: Option<&str>,
        path: &str,
    ) -> Result<Self, block_error::Error> {
        let mnemonic = EthereumMnemonic::<N, W>::from_phrase(&mnemonic)?;
        let master_extended_private_key = mnemonic.to_extended_private_key(password)?;
        let derivation_path = EthereumDerivationPath::from_str(path)?;
        let extended_private_key = master_extended_private_key.derive(&derivation_path)?;
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            path: Some(path.to_string()),
            password: password.map(String::from),
            mnemonic: Some(mnemonic.to_string()),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            ..Default::default()
        })
    }

    pub fn from_extended_private_key<N: EthereumNetwork>(
        extended_private_key: &str,
        path: &Option<String>
    ) -> Result<Self, block_error::Error> {
        let mut extended_private_key = EthereumExtendedPrivateKey::<N>::from_str(extended_private_key)?;
        if let Some(derivation_path) = path {
            let derivation_path = EthereumDerivationPath::from_str(&derivation_path)?;
            extended_private_key = extended_private_key.derive(&derivation_path)?;
        }
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            path: path.clone(),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            ..Default::default()
        })
    }

    pub fn from_private_key(private_key: &str) -> Result<Self, block_error::Error> {
        let private_key = EthereumPrivateKey::from_str(private_key)?;
        let public_key = private_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            ..Default::default()
        })
    }

    pub async fn get_balance(address: String) -> Result<Self, block_error::Error> {
        println!("In get balance func");
        let transport = match web3::transports::Http::new("https://mainnet.infura.io/v3/dcc1c768f6a148f6a0364143bc66e692:8545") {
            Ok(t) => t,
            Err(e) => return Err(block_error::Error::Web3Error(e))
        };
        println!("Here 1");
        let web3 = web3::Web3::new(transport);
        let mut accounts = match web3.eth().accounts().await{
            Ok(acc) => acc,
            Err(e) => return Err(block_error::Error::Web3Error(e))
        };
        println!("Here 2");
        accounts.push(address.parse().unwrap());
        for account in accounts {
            let balance = web3.eth().balance(account, None).await;
            println!("Balance of {:?}: {}", account, balance.unwrap());
        }
        println!("Here 3");
        Ok(Self{ balance: Some(String::from("this is a balance")), ..Default::default()})
    }

    pub fn set_wallet_name(&mut self, name: String) {
            self.wallet_name = Some(name);
    }

    pub fn generate_qr_address(&self) -> Result<gdk4::Texture, block_error::Error> {
        let address = match &self.address {
            Some(addr) => addr,
            None => ""
        };

        let qrcode = QRBuilder::new(address.to_string())
            .build()
            .unwrap();

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

#[cfg_attr(tarpaulin, skip)]
impl Display for EthereumWallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = [
            match &self.wallet_name {
                Some(path) => format!("      {}          {}\n", "Wallet Name".cyan().bold(), path),
                _ => "".to_owned(),
            },
            match &self.path {
                Some(path) => format!("      {}                 {}\n", "Path".cyan().bold(), path),
                _ => "".to_owned(),
            },
            match &self.password {
                Some(password) => format!("      {}             {}\n", "Password".cyan().bold(), password),
                _ => "".to_owned(),
            },
            match &self.mnemonic {
                Some(mnemonic) => format!("      {}             {}\n", "Mnemonic".cyan().bold(), mnemonic),
                _ => "".to_owned(),
            },
            match &self.extended_private_key {
                Some(extended_private_key) => format!(
                    "      {} {}\n",
                    "Extended Private Key".cyan().bold(),
                    extended_private_key
                ),
                _ => "".to_owned(),
            },
            match &self.extended_public_key {
                Some(extended_public_key) => format!(
                    "      {}  {}\n",
                    "Extended Public Key".cyan().bold(),
                    extended_public_key
                ),
                _ => "".to_owned(),
            },
            match &self.private_key {
                Some(private_key) => format!("      {}          {}\n", "Private Key".cyan().bold(), private_key),
                _ => "".to_owned(),
            },
            match &self.public_key {
                Some(public_key) => format!("      {}           {}\n", "Public Key".cyan().bold(), public_key),
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
            match &self.transaction_hex {
                Some(transaction_hex) => {
                    format!("      {}      {}\n", "Transaction Hex".cyan().bold(), transaction_hex)
                }
                _ => "".to_owned(),
            },
        ]
        .concat();

        let output = output[..output.len() - 1].to_owned();
        write!(f, "\n{}", output)
    }
}