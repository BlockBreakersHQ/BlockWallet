use block_wallet::configuration::application_settings::ApplicationSettings;
use block_wallet::currencies::tokens::Tokens;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[test]
fn restore_derives_btc_bip84_and_eth_bip44_from_one_mnemonic() {
    let mut settings = ApplicationSettings::new(Tokens::new());
    settings.restore_from_mnemonic(ABANDON, "").unwrap();

    assert_eq!(settings.mnemonic.as_deref(), Some(ABANDON));
    assert_eq!(settings.btc_wallets.len(), 1);
    assert_eq!(settings.eth_wallets.len(), 1);
    assert_eq!(settings.btc_wallets[0].mnemonic, settings.eth_wallets[0].mnemonic);
    assert!(settings.btc_wallets[0].address.as_ref().unwrap().starts_with("bc1q"));
    let eth = settings.eth_wallets[0].address.as_ref().unwrap();
    assert!(eth.starts_with("0x"));
    assert_eq!(eth.len(), 42);

    let mut again = ApplicationSettings::new(Tokens::new());
    again.restore_from_mnemonic(ABANDON, "").unwrap();
    assert_eq!(again.btc_wallets[0].address, settings.btc_wallets[0].address);
    assert_eq!(again.eth_wallets[0].address, settings.eth_wallets[0].address);
    assert_eq!(
        settings.btc_wallets[0].address.as_deref(),
        Some("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu")
    );
    assert_eq!(
        settings.eth_wallets[0].address.as_deref(),
        Some("0x9858EfFD232B4033E47d90003D41EC34EcaEda94")
    );
}

#[test]
fn invalid_mnemonic_is_rejected() {
    let mut settings = ApplicationSettings::new(Tokens::new());
    assert!(settings.restore_from_mnemonic("not a mnemonic", "").is_err());
    assert!(settings.mnemonic.is_none());
    assert!(settings.btc_wallets.is_empty());
    assert!(settings.eth_wallets.is_empty());
}
