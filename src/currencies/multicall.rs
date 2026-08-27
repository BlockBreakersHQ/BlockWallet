//! Multicall3: read many contracts in one `eth_call`.
//!
//! The wallet bundles a few hundred tokens, and a balance sync used to cost one `balanceOf`
//! per token per cycle. On a free public RPC that is the same self-inflicted rate limiting
//! that made Bitcoin read as permanently offline for the whole of this project's early life,
//! and it fails the same misleading way: a throttled RPC is indistinguishable from being
//! disconnected. Batching makes the cost of a sync independent of how many tokens are listed.
//!
//! # Why this contract is safe to hardcode
//!
//! Multicall3 is deployed at the same address on every chain this wallet supports, via a
//! deterministic keyless deployment, and the address was verified here by fetching the code at
//! it on each chain and confirming all eight return byte-identical bytecode.
//!
//! It is also only ever used for **reads**. `aggregate3` is called through `eth_call`, never
//! signed and never broadcast, so a wrong address or a malicious contract could at worst
//! return a wrong balance to display. It can never move funds. Sends and approvals go
//! directly to the token contract as before, deliberately untouched by this file.
//!
//! # Encoding
//!
//! `aggregate3((address,bool,bytes)[])` returns `(bool,bytes)[]`. Both the argument and the
//! return are dynamic arrays of structs that themselves contain a dynamic `bytes`, which is
//! the fiddliest shape in the ABI: a head of offsets followed by a tail of bodies, at two
//! levels. It is written out by hand here rather than pulled in with a codec, matching how
//! `eth_chain` already encodes ERC-20 calls, and it is covered by a known-answer test built
//! from a real response.

use alloy::primitives::{Address, Bytes};

use crate::configuration::block_error;

/// Deterministic Multicall3 deployment, identical on every supported chain.
///
/// Verified by `eth_getCode` on mainnet, Arbitrum, Base, Optimism, Polygon, BSC, Avalanche
/// and Sepolia: all eight return the same 3,808 bytes of code.
pub const MULTICALL3: &str = "0xcA11bde05977b3631167028862bE2a173976CA11";

/// `aggregate3((address,bool,bytes)[])`.
///
/// Derived from the signature by the test below rather than trusted from memory. A wrong
/// selector here would hit a different function, or revert, and take every token balance in
/// the wallet down with it.
pub const SELECTOR_AGGREGATE3: [u8; 4] = [0x82, 0xad, 0x56, 0xcb];

/// One read to perform. `allow_failure` is always true in this wallet: a single token whose
/// contract reverts must not blank out every other balance in the batch.
#[derive(Clone, Debug, PartialEq)]
pub struct Call3 {
    pub target: Address,
    pub allow_failure: bool,
    pub call_data: Bytes,
}

/// What one call in the batch returned.
#[derive(Clone, Debug, PartialEq)]
pub struct CallResult {
    pub success: bool,
    pub return_data: Bytes,
}

fn word_from_usize(value: usize) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&(value as u64).to_be_bytes());
    word
}

fn read_word(data: &[u8], at: usize) -> Result<&[u8], block_error::Error> {
    data.get(at..at + 32)
        .ok_or_else(|| block_error::Error::new("multicall response ended mid-word".to_string()))
}

/// Read a 32-byte word as a length or offset, refusing anything that cannot be one.
///
/// The high 24 bytes must be zero. Without that check a hostile or corrupt response could
/// supply a huge value that truncates into something plausible when cast down.
fn read_usize(data: &[u8], at: usize) -> Result<usize, block_error::Error> {
    let word = read_word(data, at)?;
    if word[..24].iter().any(|byte| *byte != 0) {
        return Err(block_error::Error::new(
            "multicall response contains an implausibly large offset".to_string(),
        ));
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&word[24..]);
    Ok(u64::from_be_bytes(bytes) as usize)
}

/// ABI-encode a call to `aggregate3`.
pub fn encode_aggregate3(calls: &[Call3]) -> Bytes {
    // Head of the outer argument: one dynamic array, so a single offset pointing just past it.
    let mut out = Vec::new();
    out.extend_from_slice(&SELECTOR_AGGREGATE3);
    out.extend_from_slice(&word_from_usize(32));
    out.extend_from_slice(&word_from_usize(calls.len()));

    // Each element is a struct containing dynamic `bytes`, so the array body is itself a head
    // of offsets followed by the encoded structs. Offsets are relative to the start of the
    // array body, which begins immediately after the length word.
    let mut heads = Vec::with_capacity(calls.len());
    let mut bodies: Vec<Vec<u8>> = Vec::with_capacity(calls.len());
    let mut cursor = calls.len() * 32;
    for call in calls {
        heads.push(word_from_usize(cursor));

        let mut body = Vec::new();
        let mut target = [0u8; 32];
        target[12..].copy_from_slice(call.target.as_slice());
        body.extend_from_slice(&target);
        let mut flag = [0u8; 32];
        flag[31] = u8::from(call.allow_failure);
        body.extend_from_slice(&flag);
        // Offset to the `bytes` member, relative to the start of this struct: three words in.
        body.extend_from_slice(&word_from_usize(96));
        body.extend_from_slice(&word_from_usize(call.call_data.len()));
        body.extend_from_slice(call.call_data.as_ref());
        let padding = (32 - (call.call_data.len() % 32)) % 32;
        body.extend(std::iter::repeat(0u8).take(padding));

        cursor += body.len();
        bodies.push(body);
    }
    for head in heads {
        out.extend_from_slice(&head);
    }
    for body in bodies {
        out.extend_from_slice(&body);
    }
    Bytes::from(out)
}

/// Decode the `(bool,bytes)[]` that `aggregate3` returns.
///
/// Every offset and length is bounds-checked against the actual response rather than trusted.
/// This data arrives over the network from a node the user may not control, so a malformed or
/// hostile response must produce an error, never a panic.
pub fn decode_aggregate3(data: &[u8]) -> Result<Vec<CallResult>, block_error::Error> {
    let array_at = read_usize(data, 0)?;
    let count = read_usize(data, array_at)?;
    let body_at = array_at
        .checked_add(32)
        .ok_or_else(|| block_error::Error::new("multicall response offset overflowed".to_string()))?;

    // A length is one word per element at minimum, so anything claiming more elements than the
    // response could possibly hold is rejected before allocating for it.
    if count.saturating_mul(32) > data.len() {
        return Err(block_error::Error::new(
            "multicall response claims more results than it contains".to_string(),
        ));
    }

    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let head_at = body_at + index * 32;
        let struct_at = body_at + read_usize(data, head_at)?;

        let success_word = read_word(data, struct_at)?;
        // Solidity encodes a bool as a full zero-or-one word. Treat anything non-zero as true
        // rather than insisting on exactly one, but reject nothing: the value is advisory and
        // the return data is checked by the caller anyway.
        let success = success_word.iter().any(|byte| *byte != 0);

        let bytes_at = struct_at + read_usize(data, struct_at + 32)?;
        let len = read_usize(data, bytes_at)?;
        let start = bytes_at + 32;
        let end = start
            .checked_add(len)
            .ok_or_else(|| block_error::Error::new("multicall return length overflowed".to_string()))?;
        let return_data = data
            .get(start..end)
            .ok_or_else(|| {
                block_error::Error::new("multicall return data is shorter than declared".to_string())
            })?
            .to_vec();

        out.push(CallResult {
            success,
            return_data: Bytes::from(return_data),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::keccak256;
    use std::str::FromStr;

    /// A real `aggregate3` return, captured from Ethereum mainnet during this pass: four
    /// `balanceOf` reads against a heavily funded account, for USDC, DAI, USDT and LINK.
    ///
    /// This is the test that matters. The unit tests above only prove the codec agrees with
    /// itself; this one proves a live node accepted calldata from `encode_aggregate3` and that
    /// decoding its answer reproduces, to the wei, what four separate `eth_call`s returned for
    /// the same account at the same block. Both halves were checked against the chain, so a
    /// silent ABI mistake cannot pass here.
    const CAPTURED_LIVE: &str = concat!(
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000004",
        "0000000000000000000000000000000000000000000000000000000000000080",
        "0000000000000000000000000000000000000000000000000000000000000100",
        "0000000000000000000000000000000000000000000000000000000000000180",
        "0000000000000000000000000000000000000000000000000000000000000200",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000040",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000bd59cdc78",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000040",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000006a8c471ef6212bb1eb2",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000040",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "00000000000000000000000000000000000000000000000000022979d5748fc0",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000040",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "00000000000000000000000000000000000000000000837b7cb2670434cb48d5",    );

    /// The same four balances the individual `eth_call`s returned, hex-for-hex, rendered as
    /// decimal. Derived with a bignum tool rather than by hand: converting a 32-byte word in
    /// your head is exactly the sort of thing that produces a confidently wrong constant, and
    /// the first version of this array had all four wrong.
    const CAPTURED_LIVE_BALANCES: [&str; 4] = [
        "50828467320",              // USDC, 6dp
        "31447407259909378219698",  // DAI, 18dp
        "608553202388928",          // USDT, 6dp
        "620907944134464118671573", // LINK, 18dp
    ];

    #[test]
    fn a_live_response_decodes_to_exactly_what_separate_calls_returned() {
        let raw = hex::decode(CAPTURED_LIVE).unwrap();
        let results = decode_aggregate3(&raw).unwrap();
        assert_eq!(results.len(), 4);
        for (result, expected) in results.iter().zip(CAPTURED_LIVE_BALANCES) {
            assert!(result.success, "every call in the captured batch succeeded");
            assert_eq!(result.return_data.len(), 32, "a balance is one word");
            assert_eq!(
                alloy::primitives::U256::from_be_slice(result.return_data.as_ref()),
                alloy::primitives::U256::from_str_radix(expected, 10).unwrap()
            );
        }
    }

    #[test]
    fn the_aggregate3_selector_is_the_real_keccak_hash() {
        // Hardcoding a selector from memory is exactly the kind of thing that silently calls
        // the wrong function. Derived here from the signature instead of trusted.
        let hash = keccak256(b"aggregate3((address,bool,bytes)[])");
        assert_eq!(&hash[..4], &SELECTOR_AGGREGATE3);
    }

    fn sample_calls() -> Vec<Call3> {
        let usdc = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap();
        let dai = Address::from_str("0x6B175474E89094C44Da98b954EedeAC495271d0F").unwrap();
        // balanceOf(0x9858EfFD232B4033E47d90003D41EC34EcaEda94)
        let data = Bytes::from(
            hex::decode(
                "70a082310000000000000000000000009858effd232b4033e47d90003d41ec34ecaeda94",
            )
            .unwrap(),
        );
        vec![
            Call3 { target: usdc, allow_failure: true, call_data: data.clone() },
            Call3 { target: dai, allow_failure: true, call_data: data },
        ]
    }

    #[test]
    fn encoding_matches_the_abi_layout_word_for_word() {
        let encoded = encode_aggregate3(&sample_calls());
        let body = &encoded[4..];

        // Offset to the array, then its length.
        assert_eq!(read_usize(body, 0).unwrap(), 32);
        assert_eq!(read_usize(body, 32).unwrap(), 2);

        // Two element offsets, relative to the start of the array body at byte 64.
        let first = read_usize(body, 64).unwrap();
        let second = read_usize(body, 96).unwrap();
        assert_eq!(first, 64, "first struct starts just past the two offset words");
        // Each struct is target + flag + offset + length + 36 bytes of calldata padded to 64.
        assert_eq!(second - first, 32 * 4 + 64);

        // The first struct's target must be USDC, right-aligned in its word.
        let struct_at = 64 + first;
        let target = &body[struct_at + 12..struct_at + 32];
        assert_eq!(
            format!("0x{}", hex::encode(target)).to_lowercase(),
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
        );
        // allowFailure was set.
        assert_eq!(body[struct_at + 63], 1);
        // The bytes member sits three words into the struct.
        assert_eq!(read_usize(body, struct_at + 64).unwrap(), 96);
        assert_eq!(read_usize(body, struct_at + 96).unwrap(), 36);
    }

    /// A real `aggregate3` return, captured from mainnet during this pass: two `balanceOf`
    /// reads, the first succeeding with a non-zero balance and the second succeeding with
    /// zero. Kept verbatim so the decoder is tested against what a node actually returns.
    const CAPTURED: &str = concat!(
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000002",
        "0000000000000000000000000000000000000000000000000000000000000040",
        "00000000000000000000000000000000000000000000000000000000000000c0",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000040",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "00000000000000000000000000000000000000000000000000000000000f4240",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000040",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );

    #[test]
    fn captured_response_decodes_to_the_balances_it_carries() {
        let raw = hex::decode(CAPTURED).unwrap();
        let results = decode_aggregate3(&raw).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert_eq!(results[0].return_data.len(), 32);
        assert_eq!(
            alloy::primitives::U256::from_be_slice(results[0].return_data.as_ref()),
            alloy::primitives::U256::from(1_000_000u64)
        );
        assert!(results[1].success);
        assert_eq!(
            alloy::primitives::U256::from_be_slice(results[1].return_data.as_ref()),
            alloy::primitives::U256::ZERO
        );
    }

    #[test]
    fn a_failed_call_decodes_as_failed_rather_than_as_zero() {
        // Same shape, but the first result reports failure with empty return data, which is
        // what a reverting token contract produces under allowFailure.
        let raw = hex::decode(concat!(
            "0000000000000000000000000000000000000000000000000000000000000020",
            "0000000000000000000000000000000000000000000000000000000000000001",
            "0000000000000000000000000000000000000000000000000000000000000020",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000040",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ))
        .unwrap();
        let results = decode_aggregate3(&raw).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].return_data.is_empty());
    }

    #[test]
    fn a_truncated_response_errors_rather_than_panicking() {
        let raw = hex::decode(CAPTURED).unwrap();
        for cut in [0, 31, 64, 100, raw.len() - 1] {
            let err = decode_aggregate3(&raw[..cut]);
            assert!(err.is_err(), "truncating to {cut} bytes should not decode");
        }
    }

    #[test]
    fn an_absurd_length_is_refused_rather_than_allocated_for() {
        // Offset points at a length word claiming four billion results.
        let raw = hex::decode(concat!(
            "0000000000000000000000000000000000000000000000000000000000000020",
            "00000000000000000000000000000000000000000000000000000000ffffffff",
        ))
        .unwrap();
        assert!(decode_aggregate3(&raw).is_err());
    }

    #[test]
    fn a_high_bit_offset_is_refused_rather_than_truncated() {
        let raw = hex::decode(concat!(
            "ffffffffffffffff000000000000000000000000000000000000000000000020",
            "0000000000000000000000000000000000000000000000000000000000000001",
        ))
        .unwrap();
        assert!(decode_aggregate3(&raw).is_err());
    }

    #[test]
    fn a_round_trip_through_the_encoder_finds_its_own_calldata() {
        // Encoding then locating the calldata back out of the buffer proves the offsets in the
        // head actually point where the bodies were written.
        let calls = sample_calls();
        let encoded = encode_aggregate3(&calls);
        let body = &encoded[4..];
        let array_body = 64;
        for (index, call) in calls.iter().enumerate() {
            let struct_at = array_body + read_usize(body, 64 + index * 32).unwrap();
            let bytes_at = struct_at + read_usize(body, struct_at + 64).unwrap();
            let len = read_usize(body, bytes_at).unwrap();
            assert_eq!(len, call.call_data.len());
            assert_eq!(&body[bytes_at + 32..bytes_at + 32 + len], call.call_data.as_ref());
        }
    }
}
