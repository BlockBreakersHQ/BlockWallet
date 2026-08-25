use block_wallet::currencies::ltc::{generate_from_mnemonic, generate_from_private_key, generate_ltc_hd_wallet};
use block_wallet::currencies::ltc_chain::{decode_address, encode_wif, ltc_to_sats, parse_network, LtcNetwork};

#[test]
fn generate_from_known_mnemonic() {
    let wallet = generate_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "",
    )
    .unwrap();
    let address = wallet.address.unwrap();
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
    let address = wallet.address.unwrap();
    let decoded = decode_address(&address).unwrap();
    assert_eq!(decoded.network, LtcNetwork::Mainnet);
    assert!(decode_address("not-a-litecoin-address").is_err());
    let wif = encode_wif(&[7u8; 32], LtcNetwork::Mainnet);
    assert!(!wif.is_empty());
}
