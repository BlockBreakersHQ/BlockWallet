use crate::ApplicationSettings;

#[cfg(test)]
mod btc_tests {
    use crate::tests::configuration::application_settings::*;

    #[test]
    fn test_generate_btc_wallet() {
        let wallet = ApplicationSettings::generate_btc_wallet(String::from("test_name"));
        assert_eq!(wallet.wallet_name.unwrap(), "test_name");
    }

    #[test]
    fn test_generate_eth_wallet() {
        let wallet = ApplicationSettings::generate_eth_wallet(String::from("test_name"));
        assert_eq!(wallet.wallet_name.unwrap(), "test_name");
    }

    #[test]
    fn test_find_config_path() {
        let mut config_path = ApplicationSettings::find_config_path().unwrap();
        config_path.pop();
        assert!(config_path.exists());
    }

    #[test]
    fn test_find_error_path() {
        let mut error_path = ApplicationSettings::find_error_path().unwrap();
        error_path.pop();
        assert!(error_path.exists());
    }
}