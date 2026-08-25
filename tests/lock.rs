use block_wallet::configuration::application_settings::ApplicationSettings;
use block_wallet::currencies::tokens::Tokens;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[test]
fn lock_wipes_keys_from_settings() {
    let mut settings = ApplicationSettings::new(Tokens::new());
    settings.restore_from_mnemonic(ABANDON, "").unwrap();
    assert!(settings.mnemonic.is_some());
    assert!(settings.btc_wallets[0].private_key.is_some());
    settings.lock_store();
    assert!(!settings.is_unlocked());
    assert!(settings.mnemonic.is_none());
    assert!(settings.btc_wallets.is_empty());
    assert!(settings.eth_wallets.is_empty());
}
