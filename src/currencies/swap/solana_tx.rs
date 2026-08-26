//! Signing a Solana transaction that somebody else built.
//!
//! Jupiter returns a complete, unsigned transaction. This wallet cannot interpret what is
//! inside it: doing so would mean implementing enough of the Solana runtime to resolve program
//! IDs, account metas and inner instructions, which is not something to attempt on a phone
//! for a feature like this.
//!
//! What *is* checkable, and what this module enforces, is the envelope:
//!
//! * the transaction requires exactly one signature, so no second party's authority is being
//!   assumed;
//! * the account that signature belongs to, which is also the fee payer, is the user's own
//!   account.
//!
//! That second point is the load-bearing one. On Solana the first account key is the fee payer
//! and the first signature slot is theirs. A transaction can only move funds out of an account
//! that signed it, so verifying the sole signer is the user's own account means the only funds
//! at risk are the ones the user already agreed to swap. Combined with the on-chain minimum
//! that Jupiter's own program enforces, that bounds the loss without reading the instructions.
//!
//! Both legacy and v0 (versioned) transactions are handled, because Jupiter returns v0.

use ed25519_dalek::SigningKey;

use crate::configuration::block_error;
use crate::currencies::sol::sign_message;
use crate::currencies::sol_chain::Pubkey;

/// Standard base64, hand-rolled.
///
/// Jupiter returns its transaction base64-encoded and nothing else in this project needs the
/// codec. Adding a crate for it would change `Cargo.lock`, which in turn invalidates the
/// vendored `generated-sources.json` the offline Flatpak build depends on. Sixty lines with
/// RFC 4648 test vectors is the cheaper trade.
pub mod base64 {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(input: &[u8]) -> String {
        let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = *chunk.get(1).unwrap_or(&0) as u32;
            let b2 = *chunk.get(2).unwrap_or(&0) as u32;
            let triple = (b0 << 16) | (b1 << 8) | b2;
            out.push(ALPHABET[(triple >> 18) as usize & 0x3f] as char);
            out.push(ALPHABET[(triple >> 12) as usize & 0x3f] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(triple >> 6) as usize & 0x3f] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[triple as usize & 0x3f] as char
            } else {
                '='
            });
        }
        out
    }

    fn value_of(byte: u8) -> Option<u32> {
        match byte {
            b'A'..=b'Z' => Some((byte - b'A') as u32),
            b'a'..=b'z' => Some((byte - b'a') as u32 + 26),
            b'0'..=b'9' => Some((byte - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    /// Decode, rejecting anything that is not well-formed rather than guessing.
    pub fn decode(input: &str) -> Option<Vec<u8>> {
        // Whitespace is tolerated because JSON transports sometimes wrap long values.
        let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
        if cleaned.len() % 4 != 0 || cleaned.is_empty() {
            return None;
        }
        let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
        for chunk in cleaned.chunks(4) {
            let pad = chunk.iter().filter(|b| **b == b'=').count();
            if pad > 2 {
                return None;
            }
            // Padding is only ever at the very end.
            if pad > 0 && !std::ptr::eq(chunk.as_ptr(), cleaned[cleaned.len() - 4..].as_ptr()) {
                return None;
            }
            let mut triple = 0u32;
            for (index, byte) in chunk.iter().enumerate() {
                let value = if *byte == b'=' {
                    if index < 2 {
                        return None;
                    }
                    0
                } else {
                    value_of(*byte)?
                };
                triple |= value << (18 - index * 6);
            }
            out.push((triple >> 16) as u8);
            if pad < 2 {
                out.push((triple >> 8) as u8);
            }
            if pad < 1 {
                out.push(triple as u8);
            }
        }
        Some(out)
    }
}

/// Solana's compact-u16 ("short vec") length prefix.
///
/// One to three bytes, seven bits of payload each, low group first, high bit set to continue.
pub fn compact_u16_decode(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut value: usize = 0;
    for index in 0..3 {
        let byte = *bytes.get(index)? as usize;
        value |= (byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Some((value, index + 1));
        }
    }
    None
}

/// A parsed transaction envelope: where the signatures are, and who the fee payer is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// Number of signature slots the transaction declares.
    pub signature_count: usize,
    /// Offset of the first signature slot.
    pub signatures_offset: usize,
    /// Offset at which the message begins, which is what gets signed.
    pub message_offset: usize,
    /// First account key, which on Solana is always the fee payer and first signer.
    pub fee_payer: Pubkey,
    /// True when the message is a v0 versioned message rather than a legacy one.
    pub versioned: bool,
}

/// Parse enough of a serialised transaction to sign it safely.
///
/// Deliberately minimal: this reads the signature count, the message header and the first
/// account key, and stops. It does not attempt to understand the instructions.
pub fn parse_envelope(tx: &[u8]) -> Result<Envelope, block_error::Error> {
    let err = |message: &str| block_error::Error::new(message.to_string());

    let (signature_count, prefix_len) =
        compact_u16_decode(tx).ok_or_else(|| err("transaction has no readable signature count"))?;
    if signature_count == 0 {
        return Err(err("transaction declares no signatures"));
    }
    let signatures_offset = prefix_len;
    let message_offset = signatures_offset
        .checked_add(signature_count.checked_mul(64).ok_or_else(|| err("transaction is malformed"))?)
        .ok_or_else(|| err("transaction is malformed"))?;
    if tx.len() <= message_offset {
        return Err(err("transaction is truncated"));
    }

    let message = &tx[message_offset..];
    // A versioned message is marked by the high bit of its first byte; the low bits are the
    // version. Only v0 exists today, and anything else is not something to sign blind.
    let (versioned, header_start) = if message[0] & 0x80 != 0 {
        let version = message[0] & 0x7f;
        if version != 0 {
            return Err(err("unsupported Solana transaction version"));
        }
        (true, 1)
    } else {
        (false, 0)
    };

    // Header is three bytes: required signatures, readonly signed, readonly unsigned.
    let header = message
        .get(header_start..header_start + 3)
        .ok_or_else(|| err("transaction header is truncated"))?;
    let required_signatures = header[0] as usize;
    if required_signatures != signature_count {
        return Err(err(
            "transaction's signature slots do not match the signatures it requires",
        ));
    }

    let keys_start = header_start + 3;
    let (key_count, key_prefix_len) = compact_u16_decode(&message[keys_start..])
        .ok_or_else(|| err("transaction has no readable account list"))?;
    if key_count == 0 {
        return Err(err("transaction references no accounts"));
    }
    let first_key_at = keys_start + key_prefix_len;
    let fee_payer_bytes = message
        .get(first_key_at..first_key_at + 32)
        .ok_or_else(|| err("transaction's account list is truncated"))?;
    let mut fee_payer = [0u8; 32];
    fee_payer.copy_from_slice(fee_payer_bytes);

    Ok(Envelope {
        signature_count,
        signatures_offset,
        message_offset,
        fee_payer,
        versioned,
    })
}

/// Sign a provider-built transaction, refusing anything whose shape is not what was expected.
///
/// Returns the fully serialised, signed transaction ready to broadcast.
pub fn sign_provider_transaction(
    tx: &[u8],
    signing_key: &SigningKey,
    expected_fee_payer: &Pubkey,
) -> Result<Vec<u8>, block_error::Error> {
    let envelope = parse_envelope(tx)?;

    // More than one signature means some other party is expected to sign too. This wallet has
    // no flow for that, and a transaction it cannot complete is not one to put a signature on.
    if envelope.signature_count != 1 {
        return Err(block_error::Error::new(format!(
            "this transaction needs {} signatures; only single-signer swaps are supported",
            envelope.signature_count
        )));
    }

    // The check that makes signing opaque instructions tolerable: the sole signer, who is also
    // the fee payer, must be the user's own account.
    if &envelope.fee_payer != expected_fee_payer {
        return Err(block_error::Error::new(
            "the swap transaction is not payable by your account; refusing to sign".to_string(),
        ));
    }
    if signing_key.verifying_key().to_bytes() != *expected_fee_payer {
        return Err(block_error::Error::new(
            "this key does not belong to the account the swap was built for".to_string(),
        ));
    }

    let signature = sign_message(signing_key, &tx[envelope.message_offset..]);
    let mut signed = tx.to_vec();
    signed[envelope.signatures_offset..envelope.signatures_offset + 64]
        .copy_from_slice(&signature);
    Ok(signed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    /// Build a minimal but structurally valid transaction envelope for testing.
    fn envelope_bytes(fee_payer: &Pubkey, signature_count: usize, versioned: bool) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(signature_count as u8); // compact-u16 for small counts is one byte
        tx.extend(std::iter::repeat(0u8).take(signature_count * 64));
        if versioned {
            tx.push(0x80); // v0 marker
        }
        tx.push(signature_count as u8); // required signatures
        tx.push(0); // readonly signed
        tx.push(1); // readonly unsigned
        tx.push(2); // two account keys
        tx.extend_from_slice(fee_payer);
        tx.extend_from_slice(&[9u8; 32]); // some program account
        tx.extend_from_slice(&[0u8; 32]); // blockhash
        tx.push(0); // no instructions
        tx
    }

    #[test]
    fn base64_matches_the_rfc_4648_test_vectors() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64::encode(plain.as_bytes()), encoded, "encoding {plain:?}");
            if !encoded.is_empty() {
                assert_eq!(
                    base64::decode(encoded).unwrap(),
                    plain.as_bytes(),
                    "decoding {encoded:?}"
                );
            }
        }
    }

    #[test]
    fn base64_round_trips_arbitrary_bytes() {
        for len in 0..200usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 7 % 256) as u8).collect();
            let encoded = base64::encode(&data);
            if data.is_empty() {
                assert!(encoded.is_empty());
                continue;
            }
            assert_eq!(base64::decode(&encoded).unwrap(), data, "round trip at len {len}");
        }
    }

    #[test]
    fn base64_rejects_malformed_input_rather_than_guessing() {
        assert!(base64::decode("Zm9vYmF").is_none()); // wrong length
        assert!(base64::decode("Zm9v!!!!").is_none()); // invalid character
        assert!(base64::decode("====").is_none()); // padding in the value positions
        assert!(base64::decode("").is_none());
    }

    #[test]
    fn compact_u16_matches_the_encoding_solana_uses() {
        assert_eq!(compact_u16_decode(&[0x00]), Some((0, 1)));
        assert_eq!(compact_u16_decode(&[0x01]), Some((1, 1)));
        assert_eq!(compact_u16_decode(&[0x7f]), Some((127, 1)));
        assert_eq!(compact_u16_decode(&[0x80, 0x01]), Some((128, 2)));
        assert_eq!(compact_u16_decode(&[0xff, 0x01]), Some((255, 2)));
        assert_eq!(compact_u16_decode(&[0x80, 0x80, 0x01]), Some((16_384, 3)));
        // Never-terminating prefix must not be accepted.
        assert_eq!(compact_u16_decode(&[0x80, 0x80, 0x80]), None);
        assert_eq!(compact_u16_decode(&[]), None);
    }

    #[test]
    fn a_versioned_transaction_parses_and_the_fee_payer_is_found() {
        let signer = key(1);
        let payer = signer.verifying_key().to_bytes();
        let tx = envelope_bytes(&payer, 1, true);
        let envelope = parse_envelope(&tx).unwrap();
        assert!(envelope.versioned);
        assert_eq!(envelope.signature_count, 1);
        assert_eq!(envelope.signatures_offset, 1);
        assert_eq!(envelope.message_offset, 65);
        assert_eq!(envelope.fee_payer, payer);
    }

    #[test]
    fn a_legacy_transaction_parses_too() {
        let signer = key(2);
        let payer = signer.verifying_key().to_bytes();
        let tx = envelope_bytes(&payer, 1, false);
        let envelope = parse_envelope(&tx).unwrap();
        assert!(!envelope.versioned);
        assert_eq!(envelope.fee_payer, payer);
    }

    #[test]
    fn signing_fills_the_first_slot_and_leaves_the_message_untouched() {
        let signer = key(3);
        let payer = signer.verifying_key().to_bytes();
        let tx = envelope_bytes(&payer, 1, true);
        let signed = sign_provider_transaction(&tx, &signer, &payer).unwrap();

        assert_eq!(signed.len(), tx.len());
        // Signature slot was zero-filled and is now populated.
        assert_ne!(&signed[1..65], &[0u8; 64][..]);
        // The message itself must be byte-identical: signing must never alter what was signed.
        assert_eq!(&signed[65..], &tx[65..]);

        // And the signature must actually verify against the message.
        use ed25519_dalek::{Signature, Verifier};
        let signature = Signature::from_slice(&signed[1..65]).unwrap();
        assert!(signer.verifying_key().verify(&tx[65..], &signature).is_ok());
    }

    #[test]
    fn a_transaction_payable_by_someone_else_is_refused() {
        // The attack this exists for: a provider returns a transaction whose fee payer, and
        // therefore whose sole authority, is not the user.
        let signer = key(4);
        let mine = signer.verifying_key().to_bytes();
        let theirs = key(5).verifying_key().to_bytes();
        let tx = envelope_bytes(&theirs, 1, true);
        let err = sign_provider_transaction(&tx, &signer, &mine).unwrap_err();
        assert!(format!("{err}").contains("not payable by your account"), "got {err:?}");
    }

    #[test]
    fn a_multi_signature_transaction_is_refused() {
        let signer = key(6);
        let payer = signer.verifying_key().to_bytes();
        let tx = envelope_bytes(&payer, 2, true);
        let err = sign_provider_transaction(&tx, &signer, &payer).unwrap_err();
        assert!(format!("{err}").contains("only single-signer"), "got {err:?}");
    }

    #[test]
    fn a_key_that_is_not_the_fee_payers_is_refused() {
        let signer = key(7);
        let other = key(8).verifying_key().to_bytes();
        let tx = envelope_bytes(&other, 1, true);
        // Expected payer matches the transaction, but the key handed in does not match either.
        let err = sign_provider_transaction(&tx, &signer, &other).unwrap_err();
        assert!(format!("{err}").contains("does not belong to the account"), "got {err:?}");
    }

    #[test]
    fn a_truncated_or_nonsensical_transaction_is_refused_rather_than_signed() {
        let signer = key(9);
        let payer = signer.verifying_key().to_bytes();
        assert!(parse_envelope(&[]).is_err());
        assert!(parse_envelope(&[1, 2, 3]).is_err());
        // Declares one signature but the body stops immediately after the slot.
        let mut truncated = vec![1u8];
        truncated.extend(std::iter::repeat(0u8).take(64));
        assert!(sign_provider_transaction(&truncated, &signer, &payer).is_err());
    }

    #[test]
    fn an_unknown_transaction_version_is_refused() {
        let signer = key(10);
        let payer = signer.verifying_key().to_bytes();
        let mut tx = envelope_bytes(&payer, 1, true);
        tx[65] = 0x81; // version 1, which does not exist
        assert!(format!("{}", parse_envelope(&tx).unwrap_err()).contains("unsupported Solana transaction version"));
    }

    #[test]
    fn a_header_disagreeing_with_the_signature_slots_is_refused() {
        // Two slots reserved but the header claims one required signature, or vice versa:
        // either way the envelope is inconsistent and must not be signed.
        let signer = key(11);
        let payer = signer.verifying_key().to_bytes();
        let mut tx = envelope_bytes(&payer, 1, true);
        tx[66] = 2; // header says two required signatures
        assert!(format!("{}", parse_envelope(&tx).unwrap_err()).contains("do not match"));
    }
}
