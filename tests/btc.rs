use block_wallet::currencies::btc::{
    generate_from_mnemonic, generate_from_private_key, BitcoinWallet,
};
use block_wallet::currencies::btc_chain;

#[test]
fn generate_from_known_mnemonic_is_bip84() {
    let wallet = generate_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "",
    )
    .unwrap();
    assert!(wallet.mnemonic.is_some());
    let address = wallet.address.unwrap();
    assert!(
        address.starts_with("bc1q"),
        "BIP84 address should be native segwit: {address}"
    );
    assert_eq!(wallet.path.as_deref(), Some("m/84'/0'/0'"));
}

#[test]
fn generate_from_wif_roundtrips_address() {
    let generated = BitcoinWallet::new().unwrap();
    let wif = generated.private_key.clone().unwrap();
    let wallet = generate_from_private_key(&wif).unwrap();
    assert_eq!(wallet.address, generated.address);
    assert!(wallet.public_key.is_some());
}

#[test]
fn mnemonic_restore_matches_generated_wallet() {
    let generated = BitcoinWallet::new().unwrap();
    let restored = generate_from_mnemonic(generated.mnemonic.as_ref().unwrap(), "").unwrap();
    assert_eq!(restored.address, generated.address);
    assert!(restored.address.unwrap().starts_with("bc1q"));
}

#[test]
fn send_path_validates_address_and_amount_without_network() {
    use bdk_wallet::bitcoin::Network;
    assert!(btc_chain::validate_address(
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
        Network::Bitcoin
    )
    .is_ok());
    assert_eq!(btc_chain::btc_to_sats("0.001").unwrap(), 100_000);
    let backend = btc_chain::parse_backend("", Network::Testnet);
    match backend {
        btc_chain::BtcBackend::Esplora(url) => assert!(url.contains("testnet")),
        _ => panic!("default testnet backend should be esplora"),
    }
}
