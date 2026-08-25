//! Sanity bounds on network fees.
//!
//! Fee estimates arrive from whichever node the user is pointed at, and the defaults are
//! public third-party endpoints. Nothing about that data is authenticated, so a hostile or
//! simply broken node can return an arbitrarily large fee rate. Without a ceiling the only
//! backstop was the balance check, which caps the damage at *the entire balance*: with 1 BTC,
//! sending 0.01, a node claiming 200_000 sat/vB produces a fee that passes every other check
//! and burns the lot on confirm.
//!
//! Two independent guards, because they fail in different places:
//!
//! * a ceiling on the *rate*, applied where the estimate is parsed, so an absurd number never
//!   reaches transaction building;
//! * a ceiling on the *absolute fee* relative to what is being sent, applied at prepare and
//!   again at broadcast, which catches a large fee arrived at some other way.

use crate::configuration::block_error;

/// Bitcoin/Litecoin fee-rate ceiling, sat/vB. Bitcoin's worst historical congestion peaked
/// near 1000 sat/vB; anything past that is a bad estimate rather than a busy mempool. Users
/// who genuinely need more can still get there by other means — this only bounds what a node
/// can talk the wallet into on its own.
pub const MAX_FEE_RATE_SAT_VB: f32 = 1_000.0;

/// Ethereum gas-price ceiling, wei per gas (10_000 gwei). Mainnet spikes have touched a few
/// thousand gwei; this leaves headroom above that and still refuses a fabricated number.
pub const MAX_GAS_PRICE_WEI: u128 = 10_000_000_000_000;

/// Below this, the "fee must not exceed the amount" rule is switched off. Sweeping a small
/// UTXO legitimately costs more than it carries, and rejecting that would be wrong. Roughly
/// a low-value send on either chain, in that chain's smallest unit.
pub const FEE_RATIO_FLOOR: u64 = 50_000;

/// Clamp a node-supplied sat/vB estimate into a usable range.
///
/// Rejects NaN, infinities, negatives and zero by falling back to `fallback`, then caps at
/// [`MAX_FEE_RATE_SAT_VB`]. A `f32 as u64` cast saturates rather than wrapping, so an
/// unclamped infinity would otherwise have become `u64::MAX` silently.
pub fn clamp_fee_rate(rate: f32, fallback: f32) -> f32 {
    let rate = if rate.is_finite() && rate > 0.0 { rate } else { fallback };
    rate.min(MAX_FEE_RATE_SAT_VB)
}

/// Clamp a node-supplied gas price into a usable range.
pub fn clamp_gas_price(wei: u128) -> u128 {
    wei.clamp(1, MAX_GAS_PRICE_WEI)
}

/// Refuse a fee that exceeds the amount being sent, once the amount is large enough for the
/// comparison to mean anything.
///
/// Deliberately not a percentage: the failure this defends against is a fee orders of
/// magnitude too large, not one a few percent high, and a tighter rule would reject
/// legitimate small sends during real congestion.
pub fn check_fee_is_sane(fee: u64, amount: u64) -> Result<(), block_error::Error> {
    if fee > amount && fee > FEE_RATIO_FLOOR {
        return Err(block_error::Error::new(format!(
            "network fee ({fee}) is larger than the amount being sent ({amount}); \
             check the fee tier and the node this wallet is pointed at"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_rejects_non_finite_and_negative_rates() {
        // Anything that is not a usable positive number falls back to the caller's default
        // rather than to the ceiling: a garbage estimate should produce an ordinary fee, not
        // the most expensive one this wallet is willing to pay.
        assert_eq!(clamp_fee_rate(f32::NAN, 2.0), 2.0);
        assert_eq!(clamp_fee_rate(f32::INFINITY, 2.0), 2.0);
        assert_eq!(clamp_fee_rate(f32::NEG_INFINITY, 2.0), 2.0);
        assert_eq!(clamp_fee_rate(-5.0, 2.0), 2.0);
        assert_eq!(clamp_fee_rate(0.0, 2.0), 2.0);
    }

    #[test]
    fn clamp_caps_an_absurd_rate_but_passes_a_normal_one() {
        assert_eq!(clamp_fee_rate(200_000.0, 2.0), MAX_FEE_RATE_SAT_VB);
        assert_eq!(clamp_fee_rate(12.5, 2.0), 12.5);
    }

    #[test]
    fn gas_price_is_bounded_at_both_ends() {
        assert_eq!(clamp_gas_price(0), 1);
        assert_eq!(clamp_gas_price(u128::MAX), MAX_GAS_PRICE_WEI);
        assert_eq!(clamp_gas_price(30_000_000_000), 30_000_000_000);
    }

    #[test]
    fn a_fee_larger_than_the_send_is_refused() {
        // The reported attack: 0.98 BTC of fee on a 0.01 BTC send.
        assert!(check_fee_is_sane(98_000_000, 1_000_000).is_err());
    }

    #[test]
    fn sweeping_a_small_utxo_is_still_allowed() {
        // Fee exceeds the amount, but both are far below the floor, so this is a normal
        // small send rather than a fabricated estimate.
        assert!(check_fee_is_sane(3_000, 1_000).is_ok());
    }

    #[test]
    fn an_ordinary_fee_passes() {
        assert!(check_fee_is_sane(2_500, 5_000_000).is_ok());
    }
}
