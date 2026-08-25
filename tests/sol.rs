use block_wallet::currencies::sol::{generate_from_mnemonic, generate_from_private_key, generate_sol_hd_wallet};
use block_wallet::currencies::sol_chain::{
    bundled_tokens, parse_network, parse_token_amount, validate_address, SolNetwork,
};

#[test]
fn generate_from_known_mnemonic() {
    let wallet = generate_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "",
    )
    .unwrap();
    let address = wallet.address.unwrap();
    assert!(!address.starts_with("0x"));
    assert!(address.len() >= 32 && address.len() <= 44);
    assert!(wallet.private_key.is_some());
    assert!(wallet.mnemonic.is_some());
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
fn sol_chain_validates_and_parses_without_rpc() {
    assert_eq!(parse_network("devnet"), SolNetwork::Devnet);
    assert!(validate_address("11111111111111111111111111111111").is_ok());
    assert!(validate_address("not-a-valid-address!!!").is_err());
    assert_eq!(parse_token_amount("1.25", 6).unwrap(), 1_250_000u64);
    assert!(bundled_tokens(SolNetwork::Mainnet)
        .iter()
        .any(|token| token.symbol == "SOL" && token.native));
    assert!(bundled_tokens(SolNetwork::Mainnet)
        .iter()
        .any(|token| token.symbol == "USDC" && !token.native));
}
