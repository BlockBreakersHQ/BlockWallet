use block_wallet::currencies::ltc::{generate_from_mnemonic, generate_from_private_key, generate_ltc_hd_wallet};
use block_wallet::currencies::ltc_chain::{decode_address, encode_wif, ltc_to_sats, parse_network, LtcNetwork};

#[test]
fn generate_from_known_mnemonic() {
    let wallet = generate_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "",
    )
    .unwrap();
    let address = wallet.address.clone().unwrap();
    assert!(address.starts_with("ltc1q"), "{address}");
    assert!(wallet.private_key.is_some());
    assert!(wallet.mnemonic.is_some());
}

#[test]
fn generate_from_private_key_roundtrips_address() {
    let generated = generate_ltc_hd_wallet().unwrap();
    let key = generated.private_key.clone().unwrap();
    let wallet = generate_from_private_key(&key).unwrap();
    assert_eq!(wallet.address, generated.address);
    assert!(wallet.public_key.is_some());
}

#[test]
fn ltc_chain_validates_and_parses_without_rpc() {
    assert_eq!(parse_network("testnet"), LtcNetwork::Testnet);
    assert_eq!(parse_network(""), LtcNetwork::Mainnet);
    assert_eq!(ltc_to_sats("1.25").unwrap(), 125_000_000);
    // Round-trip a real address through the public API rather than hand-typing a bech32
    // string, since a wrong/edited checksum would just fail to decode.
    let wallet = generate_ltc_hd_wallet().unwrap();
    let address = wallet.address.clone().unwrap();
    let decoded = decode_address(&address).unwrap();
    assert_eq!(decoded.network, LtcNetwork::Mainnet);
    assert!(decode_address("not-a-litecoin-address").is_err());
    let wif = encode_wif(&[7u8; 32], LtcNetwork::Mainnet);
    assert!(!wif.is_empty());
}

/// Switching to test networks must re-derive the Litecoin account, not just relabel it.
///
/// Regression guard: `apply_ltc_network` used to set only the network string, leaving a
/// mainnet `ltc1q…` address in place while every network-dependent decision flipped —
/// the send view hides the "spends real litecoin" acknowledgement on testnet, and the
/// receive screen would have shown a mainnet address labelled testnet.
#[test]
fn switching_networks_rederives_the_litecoin_address() {
    use block_wallet::configuration::initialization;
    use block_wallet::ApplicationSettings;

    let mut settings = ApplicationSettings::new(initialization::load_tokens());
    let phrase =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    settings.mnemonic = Some(phrase.to_string());
    settings.seed_passphrase = Some(String::new());
    settings.ltc_wallets = vec![
        block_wallet::configuration::seed::litecoin_from_seed(phrase, "", "Litecoin").unwrap(),
    ];

    let mainnet = settings.ltc_wallets[0].address.clone().unwrap();
    assert!(mainnet.starts_with("ltc1q"), "{mainnet}");

    settings.apply_ltc_network("testnet");
    let testnet = settings.ltc_wallets[0].address.clone().unwrap();
    assert!(
        testnet.starts_with("tltc1q"),
        "testnet address should use the tltc HRP, got {testnet}"
    );
    assert_ne!(mainnet, testnet);

    settings.apply_ltc_network("litecoin");
    assert_eq!(settings.ltc_wallets[0].address.clone().unwrap(), mainnet);
}
