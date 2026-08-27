//! Jupiter: Solana's DEX aggregator.
//!
//! Two round trips. `/swap/v1/quote` prices the route, and `/swap/v1/swap` turns the chosen
//! route into a ready-to-sign transaction. Unauthenticated on the lite endpoint, which is why
//! it is used here in preference to anything needing a key.
//!
//! # The honest caveat
//!
//! Jupiter returns a fully-built transaction. This wallet signs it without being able to say
//! what it does: the instructions reference program IDs and accounts that would need a full
//! Solana runtime to interpret, and doing that on a phone is not realistic. Two things make it
//! tolerable rather than reckless:
//!
//! * the fee payer is verified to be the user's own account before signing, so the transaction
//!   cannot be built to have somebody else pay or to move a different account's funds without
//!   that account's signature, which this wallet never provides;
//! * `otherAmountThreshold` from the quote is a floor Jupiter's own program enforces on-chain,
//!   so the route either delivers at least that much or reverts.
//!
//! What is not covered: Jupiter could route through a pool that is merely bad rather than
//! malicious, within the slippage the user allowed. That is a priced risk.

use serde_json::{json, Value};

use crate::configuration::block_error;
use crate::configuration::http;
use crate::currencies::sol_chain;
use crate::currencies::swap::{
    Custody, SwapAsset, SwapExecution, SwapProvider, SwapQuote, SwapRequest,
};

const QUOTE_API: &str = "https://lite-api.jup.ag/swap/v1/quote";
const SWAP_API: &str = "https://lite-api.jup.ag/swap/v1/swap";

/// Jupiter identifies native SOL by the wrapped-SOL mint.
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

pub struct Jupiter;

impl Jupiter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Jupiter {
    fn default() -> Self {
        Self::new()
    }
}

fn mint_param(asset: &SwapAsset) -> String {
    if asset.native {
        WSOL_MINT.to_string()
    } else {
        asset.address.clone()
    }
}

/// The quote fields this wallet acts on.
#[derive(Clone, Debug, PartialEq)]
pub struct JupiterQuote {
    pub in_amount: u128,
    pub out_amount: u128,
    /// Jupiter's on-chain-enforced floor for the output.
    pub other_amount_threshold: u128,
    pub price_impact_pct: Option<f64>,
    /// The whole quote object, echoed back verbatim to `/swap`, which requires it unmodified.
    pub raw: Value,
}

pub fn parse_quote(json: &Value) -> Result<JupiterQuote, block_error::Error> {
    if let Some(message) = json.get("error").and_then(Value::as_str) {
        return Err(block_error::Error::new(format!("Jupiter declined: {message}")));
    }
    let u128_at = |key: &str| {
        json.get(key)
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<u128>().ok())
    };

    let in_amount = u128_at("inAmount")
        .ok_or_else(|| block_error::Error::new("Jupiter quote has no input amount".to_string()))?;
    let out_amount = u128_at("outAmount")
        .ok_or_else(|| block_error::Error::new("Jupiter quote has no output amount".to_string()))?;
    let other_amount_threshold = u128_at("otherAmountThreshold").ok_or_else(|| {
        block_error::Error::new("Jupiter quote has no minimum output threshold".to_string())
    })?;

    // ExactIn is the only mode this wallet asks for; anything else would mean the input is not
    // the amount the user typed.
    if let Some(mode) = json.get("swapMode").and_then(Value::as_str) {
        if !mode.eq_ignore_ascii_case("ExactIn") {
            return Err(block_error::Error::new(format!(
                "Jupiter returned an unexpected swap mode: {mode}"
            )));
        }
    }

    Ok(JupiterQuote {
        in_amount,
        out_amount,
        other_amount_threshold,
        price_impact_pct: json
            .get("priceImpactPct")
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok()),
        raw: json.clone(),
    })
}

/// Build the `/swap` request body.
///
/// Jupiter enforces `feeAccount` whenever the quote carried `platformFeeBps > 0`: omitting it
/// does not silently skip the fee, it fails the whole swap. `feeAccount` must itself be a
/// token account whose mint is part of the swap pair (input or output, since this wallet only
/// ever asks for ExactIn), not an arbitrary wallet address, so a single fixed account only
/// works for pairs that actually include its mint.
fn swap_body(quote_raw: &Value, user_pubkey: &str, fee_bps: u32, payout: &str) -> Value {
    let mut body = json!({
        "quoteResponse": quote_raw,
        "userPublicKey": user_pubkey,
        "wrapAndUnwrapSol": true,
        "dynamicComputeUnitLimit": true,
    });
    if fee_bps > 0 {
        body["feeAccount"] = json!(payout);
    }
    body
}

/// Pull the base64 transaction out of a `/swap` response.
pub fn parse_swap_transaction(json: &Value) -> Result<String, block_error::Error> {
    if let Some(message) = json.get("error").and_then(Value::as_str) {
        return Err(block_error::Error::new(format!("Jupiter declined: {message}")));
    }
    json.get("swapTransaction")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .ok_or_else(|| block_error::Error::new("Jupiter returned no transaction to sign".to_string()))
}

impl SwapProvider for Jupiter {
    fn id(&self) -> &'static str {
        "jupiter"
    }

    fn display_name(&self) -> &'static str {
        "Jupiter"
    }

    fn custody(&self) -> Custody {
        Custody::AtomicOnChain
    }

    fn supports(&self, from: &SwapAsset, to: &SwapAsset) -> bool {
        from.chain == "sol"
            && to.chain == "sol"
            && mint_param(from) != mint_param(to)
    }

    fn quote(&self, request: &SwapRequest) -> Result<SwapQuote, block_error::Error> {
        // Jupiter has no devnet liquidity worth quoting, so a devnet wallet is told so rather
        // than shown mainnet prices it cannot act on.
        if sol_chain::parse_network(&request.sol_network) == sol_chain::SolNetwork::Devnet {
            return Err(block_error::Error::new(
                "Jupiter has no devnet liquidity; switch to mainnet to swap".to_string(),
            ));
        }

        let slippage = request
            .slippage_bps
            .clamp(1, super::safety::MAX_SLIPPAGE_BPS);
        // Jupiter pays a platform fee into a referral token account, which has to be created
        // through their referral program first; a wallet address will not do. Only requested
        // when one is configured.
        let payout = request.fee.solana.trim();
        let fee_bps = request.fee.bps_for(payout);
        let fee_param = if fee_bps > 0 {
            format!("&platformFeeBps={fee_bps}")
        } else {
            String::new()
        };
        let url = format!(
            "{QUOTE_API}?inputMint={}&outputMint={}&amount={}&slippageBps={slippage}{fee_param}",
            mint_param(&request.from),
            mint_param(&request.to),
            request.amount_in_base,
        );
        let text = http::get_text(&url)?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| block_error::Error::new(format!("invalid Jupiter response: {e}")))?;
        let quote = parse_quote(&json)?;

        if quote.in_amount != request.amount_in_base {
            return Err(block_error::Error::new(
                "Jupiter quoted a different input amount than was requested".to_string(),
            ));
        }

        // Second round trip: turn the route into a signable transaction. The quote object is
        // echoed back exactly as received, which is what Jupiter requires.
        let body = swap_body(&quote.raw, request.from_address.trim(), fee_bps, payout);
        let swap_text = http::post_json(SWAP_API, &body)?;
        let swap_json: Value = serde_json::from_str(&swap_text)
            .map_err(|e| block_error::Error::new(format!("invalid Jupiter swap response: {e}")))?;
        let transaction_b64 = parse_swap_transaction(&swap_json)?;

        Ok(SwapQuote {
            provider_id: self.id(),
            provider_name: self.display_name(),
            custody: self.custody(),
            from: request.from.clone(),
            to: request.to.clone(),
            amount_in_base: request.amount_in_base,
            expected_out_base: quote.out_amount,
            min_out_base: quote.other_amount_threshold,
            // A Solana swap settles back into the signer's own accounts.
            destination: request.from_address.clone(),
            expiry: None,
            // Solana settles in a slot or two; the honest number is "seconds".
            eta_seconds: Some(30),
            route_note: quote
                .price_impact_pct
                .map(|p| format!("{:.3}% price impact", p * 100.0)),
            fee_total_base: None,
            min_in_base: None,
            fee_bps,
            execution: SwapExecution::SolanaTx { transaction_b64 },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from a real unauthenticated Jupiter quote for 0.1 SOL to USDC.
    const CAPTURED: &str = r#"{
      "inputMint":"So11111111111111111111111111111111111111112",
      "inAmount":"100000000",
      "outputMint":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      "outAmount":"9636532",
      "otherAmountThreshold":"9588350",
      "swapMode":"ExactIn",
      "slippageBps":50,
      "platformFee":null,
      "priceImpactPct":"0.0000176689473909336431959176",
      "routePlan":[],
      "contextSlot":123456
    }"#;

    #[test]
    fn captured_quote_parses_every_field_we_rely_on() {
        let quote = parse_quote(&serde_json::from_str(CAPTURED).unwrap()).unwrap();
        assert_eq!(quote.in_amount, 100_000_000);
        assert_eq!(quote.out_amount, 9_636_532);
        assert_eq!(quote.other_amount_threshold, 9_588_350);
        assert!(quote.price_impact_pct.unwrap() < 0.001);
        // The raw object must survive intact for the /swap round trip.
        assert_eq!(quote.raw.get("swapMode").unwrap(), "ExactIn");
    }

    #[test]
    fn a_quote_without_its_threshold_is_refused() {
        let body = r#"{"inAmount":"1","outAmount":"2","swapMode":"ExactIn"}"#;
        let err = parse_quote(&serde_json::from_str(body).unwrap()).unwrap_err();
        assert!(format!("{err}").contains("minimum output threshold"));
    }

    #[test]
    fn an_unexpected_swap_mode_is_refused() {
        // ExactOut would mean the input is not the amount the user typed.
        let body = r#"{"inAmount":"1","outAmount":"2","otherAmountThreshold":"2","swapMode":"ExactOut"}"#;
        let err = parse_quote(&serde_json::from_str(body).unwrap()).unwrap_err();
        assert!(format!("{err}").contains("unexpected swap mode"));
    }

    #[test]
    fn swap_transaction_is_extracted_or_errors_clearly() {
        let ok = r#"{"swapTransaction":"AQABAg=="}"#;
        assert_eq!(
            parse_swap_transaction(&serde_json::from_str(ok).unwrap()).unwrap(),
            "AQABAg=="
        );
        let empty = r#"{"swapTransaction":""}"#;
        assert!(parse_swap_transaction(&serde_json::from_str(empty).unwrap()).is_err());
        let failed = r#"{"error":"Route not found"}"#;
        assert!(format!(
            "{}",
            parse_swap_transaction(&serde_json::from_str(failed).unwrap()).unwrap_err()
        )
        .contains("Route not found"));
    }

    #[test]
    fn native_sol_is_addressed_by_the_wrapped_mint() {
        let sol = SwapAsset { chain: "sol".into(), symbol: "SOL".into(), address: String::new(), decimals: 9, native: true };
        assert_eq!(mint_param(&sol), WSOL_MINT);
    }

    #[test]
    fn fee_account_is_sent_only_when_a_fee_is_actually_charged() {
        let raw = json!({"swapMode": "ExactIn"});

        // No payout configured: Jupiter would reject a feeAccount it was never asked to pay
        // through platformFeeBps, so the field must be entirely absent, not empty.
        let unconfigured = swap_body(&raw, "userpubkey", 0, "");
        assert!(unconfigured.get("feeAccount").is_none());

        // A fee was requested: Jupiter requires feeAccount whenever platformFeeBps > 0, or it
        // fails the whole swap rather than silently skipping the fee.
        let configured = swap_body(&raw, "userpubkey", 100, "FeeTokenAccountPubkey11111111111");
        assert_eq!(
            configured.get("feeAccount").and_then(Value::as_str),
            Some("FeeTokenAccountPubkey11111111111")
        );
        assert_eq!(
            configured.get("userPublicKey").and_then(Value::as_str),
            Some("userpubkey")
        );
    }

    #[test]
    fn only_solana_pairs_are_offered_and_never_a_no_op() {
        let jup = Jupiter::new();
        let sol = SwapAsset { chain: "sol".into(), symbol: "SOL".into(), address: String::new(), decimals: 9, native: true };
        let usdc = SwapAsset {
            chain: "sol".into(),
            symbol: "USDC".into(),
            address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            decimals: 6,
            native: false,
        };
        let eth = SwapAsset { chain: "eth".into(), symbol: "ETH".into(), address: String::new(), decimals: 18, native: true };

        assert!(jup.supports(&sol, &usdc));
        assert!(!jup.supports(&sol, &eth));
        assert!(!jup.supports(&sol, &sol));

        // Wrapped SOL and native SOL are the same asset to Jupiter, so that pair is a no-op
        // rather than a swap.
        let wsol = SwapAsset {
            chain: "sol".into(),
            symbol: "wSOL".into(),
            address: WSOL_MINT.into(),
            decimals: 9,
            native: false,
        };
        assert!(!jup.supports(&sol, &wsol));
    }
}
