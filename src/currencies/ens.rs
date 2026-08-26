//! ENS name resolution, so a recipient can be typed as `vitalik.eth`.
//!
//! Two contract calls against the ENS registry, which lives on Ethereum mainnet:
//!
//! 1. `resolver(bytes32 node)` on the registry, to find which contract answers for the name;
//! 2. `addr(bytes32 node)` on that resolver, to get the address.
//!
//! # Why this always uses mainnet
//!
//! ENS records live on mainnet regardless of where the funds are going. A name resolved while
//! the wallet is pointed at Base or Arbitrum must still be looked up on mainnet, or the
//! registry simply is not there and every name appears not to exist. The resolved address is
//! then usable on any EVM chain, since addresses are chain-agnostic.
//!
//! Sepolia is the exception: it has its own ENS deployment at the same address, used for
//! testing, so a testnet wallet resolves against Sepolia rather than mainnet.
//!
//! # What is deliberately not supported
//!
//! Wildcard/offchain resolution (ENSIP-10 / CCIP-read) is not implemented. Those names return
//! an `OffchainLookup` revert that has to be followed to an HTTP gateway, and silently sending
//! funds based on an answer fetched from an arbitrary URL named by a contract is not something
//! this wallet should do without a lot more thought. Such names report as unresolvable.

use alloy::primitives::{keccak256, Address, B256};

use crate::configuration::block_error;
use crate::currencies::eth_chain::EthNetwork;

/// The ENS registry. Same address on mainnet and on Sepolia's test deployment.
pub const ENS_REGISTRY: &str = "0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e";

/// True when the input looks like a name to resolve rather than a raw address.
///
/// Deliberately broad: ENS supports more than `.eth`, and anything containing a dot that is
/// not a hex address is worth attempting. A wrong guess costs one failed lookup and a clear
/// message, whereas being too narrow silently treats a valid name as a malformed address.
pub fn looks_like_name(input: &str) -> bool {
    let text = input.trim();
    !text.is_empty()
        && !text.starts_with("0x")
        && !text.starts_with("0X")
        && text.contains('.')
        && !text.ends_with('.')
        && !text.starts_with('.')
}

/// EIP-137 namehash.
///
/// Recursive hash from the root: `namehash([]) = 0x00…00`, and
/// `namehash([label, …rest]) = keccak256(namehash(rest) ++ keccak256(label))`.
pub fn namehash(name: &str) -> B256 {
    let mut node = [0u8; 32];
    let name = name.trim().trim_matches('.');
    if name.is_empty() {
        return B256::from(node);
    }
    // Labels are combined right to left, so the top-level domain is folded in first.
    for label in name.rsplit('.') {
        let label_hash = keccak256(label.as_bytes());
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&node);
        buf[32..].copy_from_slice(label_hash.as_slice());
        node = keccak256(buf).into();
    }
    B256::from(node)
}

/// Normalise a name before hashing.
///
/// A deliberately conservative subset of UTS-46: lower-casing and rejecting anything that is
/// not plain ASCII. Full normalisation involves unicode mapping and confusable handling, and
/// getting it subtly wrong would hash to a different name than the user believes they typed,
/// which for a payment destination is the worst possible failure. Refusing non-ASCII is
/// honest; guessing is not.
pub fn normalize(name: &str) -> Result<String, block_error::Error> {
    let trimmed = name.trim();
    if !trimmed.is_ascii() {
        return Err(block_error::Error::new(
            "only plain ASCII ENS names are supported; this one could resolve to something other than it appears"
                .to_string(),
        ));
    }
    if trimmed.split('.').any(|label| label.is_empty()) {
        return Err(block_error::Error::new("that name has an empty label".to_string()));
    }
    Ok(trimmed.to_ascii_lowercase())
}

/// Which chain the registry should be queried on for a given wallet network.
pub fn registry_network(network: EthNetwork) -> EthNetwork {
    match network {
        // Sepolia has its own ENS deployment for testing.
        EthNetwork::Sepolia => EthNetwork::Sepolia,
        // Everything else resolves against mainnet, including the L2s.
        _ => EthNetwork::Mainnet,
    }
}

/// `resolver(bytes32)` calldata.
pub fn encode_resolver_call(node: B256) -> Vec<u8> {
    const SELECTOR_RESOLVER: [u8; 4] = [0x01, 0x78, 0xb8, 0xbf];
    let mut data = Vec::with_capacity(36);
    data.extend_from_slice(&SELECTOR_RESOLVER);
    data.extend_from_slice(node.as_slice());
    data
}

/// `addr(bytes32)` calldata.
pub fn encode_addr_call(node: B256) -> Vec<u8> {
    const SELECTOR_ADDR: [u8; 4] = [0x3b, 0x3b, 0x57, 0xde];
    let mut data = Vec::with_capacity(36);
    data.extend_from_slice(&SELECTOR_ADDR);
    data.extend_from_slice(node.as_slice());
    data
}

/// Read an address out of a 32-byte ABI word.
pub fn decode_address_word(raw: &[u8]) -> Option<Address> {
    if raw.len() < 32 {
        return None;
    }
    // Right-aligned in the word; the leading 12 bytes are zero padding.
    let address = Address::from_slice(&raw[12..32]);
    if address == Address::ZERO {
        // The registry returns the zero address for "no resolver" and "no record", both of
        // which mean unresolvable rather than "send to 0x0".
        None
    } else {
        Some(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn namehash_matches_the_eip137_vectors() {
        // The published EIP-137 test vectors. These are the whole correctness argument for
        // resolution: a wrong namehash resolves a different name than the user typed.
        assert_eq!(namehash(""), B256::ZERO);
        assert_eq!(
            namehash("eth"),
            B256::from_str("0x93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae").unwrap()
        );
        assert_eq!(
            namehash("foo.eth"),
            B256::from_str("0xde9b09fd7c5f901e23a3f19fecc54828e9c848539801e86591bd9801b019f84f").unwrap()
        );
    }

    #[test]
    fn namehash_ignores_surrounding_dots_and_whitespace() {
        assert_eq!(namehash("foo.eth"), namehash("  foo.eth  "));
        assert_eq!(namehash("foo.eth"), namehash("foo.eth."));
    }

    #[test]
    fn names_are_told_apart_from_addresses() {
        assert!(looks_like_name("vitalik.eth"));
        assert!(looks_like_name("sub.domain.eth"));
        assert!(looks_like_name("example.xyz"));
        // Raw addresses must never be treated as names.
        assert!(!looks_like_name("0x9858EfFD232B4033E47d90003D41EC34EcaEda94"));
        assert!(!looks_like_name(""));
        assert!(!looks_like_name("   "));
        assert!(!looks_like_name("nodot"));
        assert!(!looks_like_name(".eth"));
        assert!(!looks_like_name("trailing."));
    }

    #[test]
    fn normalisation_lowercases_and_refuses_non_ascii() {
        assert_eq!(normalize("VITALIK.ETH").unwrap(), "vitalik.eth");
        assert_eq!(normalize("  Foo.Eth ").unwrap(), "foo.eth");
        // Homograph risk: refused rather than guessed at.
        assert!(normalize("vitalіk.eth").is_err());
        assert!(normalize("münchen.eth").is_err());
        assert!(normalize("a..eth").is_err());
    }

    #[test]
    fn selectors_are_the_real_keccak_hashes() {
        // Derived rather than trusted, same reasoning as the THORChain router selector.
        let resolver = alloy::primitives::keccak256(b"resolver(bytes32)");
        assert_eq!(&resolver[0..4], &[0x01, 0x78, 0xb8, 0xbf]);
        let addr = alloy::primitives::keccak256(b"addr(bytes32)");
        assert_eq!(&addr[0..4], &[0x3b, 0x3b, 0x57, 0xde]);
    }

    #[test]
    fn calldata_is_selector_plus_the_node() {
        let node = namehash("foo.eth");
        let data = encode_resolver_call(node);
        assert_eq!(data.len(), 36);
        assert_eq!(&data[0..4], &[0x01, 0x78, 0xb8, 0xbf]);
        assert_eq!(&data[4..36], node.as_slice());
    }

    #[test]
    fn a_zero_answer_reads_as_unresolvable_rather_than_the_zero_address() {
        // Sending to 0x0 burns funds, so this distinction matters more than usual.
        let mut word = [0u8; 32];
        assert!(decode_address_word(&word).is_none());
        word[31] = 1;
        assert!(decode_address_word(&word).is_some());
        assert!(decode_address_word(&[0u8; 4]).is_none());
    }

    #[test]
    fn l2_networks_resolve_against_mainnet() {
        // The registry only exists on mainnet, so an L2 wallet must not look for it locally.
        assert_eq!(registry_network(EthNetwork::Base), EthNetwork::Mainnet);
        assert_eq!(registry_network(EthNetwork::ArbitrumOne), EthNetwork::Mainnet);
        assert_eq!(registry_network(EthNetwork::Mainnet), EthNetwork::Mainnet);
        // Sepolia has its own deployment.
        assert_eq!(registry_network(EthNetwork::Sepolia), EthNetwork::Sepolia);
    }
}
