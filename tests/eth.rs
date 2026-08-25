use block_wallet::currencies::eth::{
    generate_eth_hd_wallet, generate_from_mnemonic, generate_from_private_key,
};
use block_wallet::currencies::eth_chain::{
    bundled_tokens, parse_network, parse_token_amount, validate_address, EthNetwork,
};
use alloy::primitives::U256;

#[test]
fn generate_from_known_mnemonic() {
    let wallet = generate_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "",
    )
    .unwrap();
    let address = wallet.address.clone().unwrap();
    assert!(address.starts_with("0x"));
    assert_eq!(address.len(), 42);
    assert!(wallet.private_key.is_some());
    assert!(wallet.mnemonic.is_some());
}

#[test]
fn generate_from_private_key_roundtrips_address() {
    let generated = generate_eth_hd_wallet().unwrap();
    let key = generated.private_key.clone().unwrap();
    let wallet = generate_from_private_key(&key).unwrap();
    assert_eq!(wallet.address, generated.address);
    assert!(wallet.public_key.is_some());
}

#[test]
fn eth_chain_validates_and_parses_without_rpc() {
    assert_eq!(parse_network("sepolia"), EthNetwork::Sepolia);
    assert!(validate_address("0x9858Eff28F61CF0aDe1AC00482789d2EF5e6d47E").is_ok());
    assert!(validate_address("vitalik.eth").is_err());
    assert_eq!(
        parse_token_amount("1.25", 6).unwrap(),
        U256::from(1_250_000u64)
    );
    assert!(bundled_tokens(EthNetwork::Mainnet)
        .iter()
        .any(|token| token.symbol == "USDC" && !token.native));
}
