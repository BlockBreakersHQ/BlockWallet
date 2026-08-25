use block_wallet::ApplicationSettings;

#[test]
fn generate_btc_wallet_sets_name() {
    let wallet = ApplicationSettings::generate_btc_wallet(String::from("test_name")).unwrap();
    assert_eq!(wallet.wallet_name.clone().unwrap(), "test_name");
}

#[test]
fn generate_eth_wallet_sets_name() {
    let wallet = ApplicationSettings::generate_eth_wallet(String::from("test_name")).unwrap();
    assert_eq!(wallet.wallet_name.clone().unwrap(), "test_name");
}

#[test]
fn find_config_path_parent_exists() {
    let config_path = ApplicationSettings::find_config_path().unwrap();
    assert_eq!(config_path.file_name().unwrap(), "Config.dic");
    assert!(config_path.parent().unwrap().exists());
    let mut exe_dir = std::env::current_exe().unwrap();
    exe_dir.pop();
    assert_ne!(config_path.parent().unwrap(), exe_dir.as_path());
}

#[test]
fn find_error_path_parent_exists() {
    let error_path = ApplicationSettings::find_error_path().unwrap();
    assert_eq!(error_path.file_name().unwrap(), "blockwallet.log");
    assert!(error_path.parent().unwrap().exists());
}
