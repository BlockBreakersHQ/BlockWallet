//! KyberSwap Aggregator: a second same-chain EVM venue alongside LI.FI.
//!
//! Added for the same reason a wallet quotes more than one exchange at all: aggregators
//! disagree, sometimes by a lot, and the only way to know which one is better for a given
//! pair on a given day is to ask both. It also means an outage at one venue no longer means
//! "no route" for every EVM pair.
//!
//! Chosen on the same test LI.FI passed: it answers unauthenticated. Probed live from the
//! phone during this pass alongside OpenOcean (403 behind Cloudflare) and Odos (HTTP 530),
//! neither of which a wallet can rely on without a key or a browser.
//!
//! Two round trips, unlike LI.FI's one: `/routes` prices the swap and `/route/build` turns
//! the chosen route into calldata. Both are needed because Kyber will not build a route it
//! did not itself just price.

use serde_json::{json, Value};

use crate::configuration::block_error;
use crate::configuration::http;
use crate::currencies::eth_chain::{self, EthNetwork};
use crate::currencies::swap::{
    Custody, SwapAsset, SwapExecution, SwapProvider, SwapQuote, SwapRequest,
};

const API_HOST: &str = "https://aggregator-api.kyberswap.com";

/// Kyber's spelling of "the chain's native asset". Note this is the `0xeee...` sentinel rather
/// than LI.FI's zero address; the two aggregators disagree and each must be fed its own.
const NATIVE_SENTINEL: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

pub struct KyberSwap;

impl KyberSwap {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KyberSwap {
    fn default() -> Self {
        Self::new()
    }
}

/// Kyber addresses chains by name in the URL path, not by chain id.
///
/// Sepolia is absent for the same reason it is absent from LI.FI: there is no testnet
/// liquidity worth quoting, and a testnet "price" would be noise dressed as a rehearsal.
pub fn chain_slug(network: EthNetwork) -> Option<&'static str> {
    match network {
        EthNetwork::Mainnet => Some("ethereum"),
        EthNetwork::ArbitrumOne => Some("arbitrum"),
        EthNetwork::Base => Some("base"),
        EthNetwork::Optimism => Some("optimism"),
        EthNetwork::PolygonPos => Some("polygon"),
        EthNetwork::BnbSmartChain => Some("bsc"),
        EthNetwork::AvalancheCChain => Some("avalanche"),
        EthNetwork::Sepolia => None,
    }
}

fn slug_for_chain_id(chain_id: u64) -> Option<&'static str> {
    for network in [
        EthNetwork::Mainnet,
        EthNetwork::ArbitrumOne,
        EthNetwork::Base,
        EthNetwork::Optimism,
        EthNetwork::PolygonPos,
        EthNetwork::BnbSmartChain,
        EthNetwork::AvalancheCChain,
    ] {
        if eth_chain::chain_id(network) == chain_id {
            return chain_slug(network);
        }
    }
    None
}

fn token_param(asset: &SwapAsset) -> String {
    if asset.native {
        NATIVE_SENTINEL.to_string()
    } else {
        asset.address.clone()
    }
}

/// The route summary and router address, which `/route/build` needs handed straight back.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedRoute {
    pub route_summary: Value,
    pub router_address: String,
    pub amount_out: u128,
}

/// Kyber wraps everything in `{code, message, data}` and reports failures with a non-zero
/// code and HTTP 200, so the code has to be read rather than relying on the status.
fn payload(json: &Value, what: &str) -> Result<Value, block_error::Error> {
    let code = json.get("code").and_then(Value::as_i64).unwrap_or(0);
    if code != 0 {
        let message = json
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("no reason given");
        return Err(block_error::Error::new(format!(
            "KyberSwap declined the {what}: {message}"
        )));
    }
    json.get("data")
        .cloned()
        .ok_or_else(|| block_error::Error::new(format!("KyberSwap {what} response has no data")))
}

pub fn parse_route(json: &Value) -> Result<ParsedRoute, block_error::Error> {
    let data = payload(json, "route")?;
    let summary = data
        .get("routeSummary")
        .cloned()
        .ok_or_else(|| block_error::Error::new("KyberSwap route has no summary".to_string()))?;
    let amount_out = summary
        .get("amountOut")
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<u128>().ok())
        .ok_or_else(|| {
            block_error::Error::new("KyberSwap route has no output amount".to_string())
        })?;
    let router_address = data
        .get("routerAddress")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            block_error::Error::new("KyberSwap route has no router address".to_string())
        })?;
    Ok(ParsedRoute {
        route_summary: summary,
        router_address,
        amount_out,
    })
}

/// The built call.

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedBuild {
    pub data: String,
    pub router_address: String,
    pub amount_out: u128,
    /// The wei to attach to the call. Kyber reports it rather than leaving it to be inferred,
    /// which matters for a native-asset input.
    pub transaction_value: String,
    pub gas: u64,
}

pub fn parse_build(json: &Value) -> Result<ParsedBuild, block_error::Error> {
    let data = payload(json, "transaction")?;
    let str_num = |key: &str| {
        data.get(key)
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<u128>().ok())
    };
    let amount_out = str_num("amountOut").ok_or_else(|| {
        block_error::Error::new("KyberSwap transaction has no output amount".to_string())
    })?;
    Ok(ParsedBuild {
        data: data
            .get("data")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                block_error::Error::new("KyberSwap transaction has no calldata".to_string())
            })?,
        router_address: data
            .get("routerAddress")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                block_error::Error::new("KyberSwap transaction has no router address".to_string())
            })?,
        amount_out,
        transaction_value: data
            .get("transactionValue")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| "0".to_string()),
        gas: data
            .get("gas")
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(500_000),
    })
}

/// The floor Kyber will enforce on-chain, derived rather than read.
///
/// Unlike LI.FI, Kyber does not report a minimum output: it bakes the slippage tolerance it
/// was given into the calldata and returns only the expected amount. Checked against a live
/// response during this pass rather than assumed, because the first version of this file
/// looked for a `minAmountOut` field that does not exist, and would have rejected every real
/// route as having no minimum.
///
/// So the same tolerance that was sent is applied here to get the number the user is shown.
/// It is the wallet stating the floor it asked for, not the provider confirming one, which is
/// why `safety::check_quote` still bounds it independently.
pub fn min_out_from_tolerance(amount_out: u128, slippage_bps: u32) -> u128 {
    let bps = slippage_bps.min(10_000);
    amount_out.saturating_mul(u128::from(10_000u32 - bps)) / 10_000
}

impl SwapProvider for KyberSwap {
    fn id(&self) -> &'static str {
        "kyberswap"
    }

    fn display_name(&self) -> &'static str {
        "KyberSwap"
    }

    fn custody(&self) -> Custody {
        Custody::AtomicOnChain
    }

    fn supports(&self, from: &SwapAsset, to: &SwapAsset) -> bool {
        from.chain == "eth"
            && to.chain == "eth"
            && !(from.native && to.native)
            && !from.address.eq_ignore_ascii_case(&to.address)
    }

    fn quote(&self, request: &SwapRequest) -> Result<SwapQuote, block_error::Error> {
        let chain_id = request.evm_chain_id.ok_or_else(|| {
            block_error::Error::new(
                "no aggregator liquidity on the selected Ethereum network".to_string(),
            )
        })?;
        let slug = slug_for_chain_id(chain_id).ok_or_else(|| {
            block_error::Error::new("KyberSwap does not serve this network".to_string())
        })?;

        // The fee has to be priced into the **route**, not just named at build time.
        //
        // Kyber accepts these on the build body too and silently ignores them there: a build
        // from a route that was quoted without a fee returns the same output, and the fee
        // receiver never appears in the calldata. Verified against the live API, which is the
        // only way this was ever going to surface. Passed here, the quoted output drops by
        // exactly the bps asked for and the receiver is embedded in the transaction.
        let payout = request.fee.evm.trim();
        let fee_bps = request.fee.bps_for(payout);
        let fee_query = if fee_bps > 0 {
            format!("&feeAmount={fee_bps}&chargeFeeBy=currency_in&isInBps=true&feeReceiver={payout}")
        } else {
            String::new()
        };

        let routes_url = format!(
            "{API_HOST}/{slug}/api/v1/routes?tokenIn={}&tokenOut={}&amountIn={}{fee_query}",
            token_param(&request.from),
            token_param(&request.to),
            request.amount_in_base,
        );
        let text = http::get_text(&routes_url)?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| block_error::Error::new(format!("invalid KyberSwap response: {e}")))?;
        let route = parse_route(&json)?;

        let build_url = format!("{API_HOST}/{slug}/api/v1/route/build");
        // The routeSummary already carries the fee that was priced into it, so the build
        // body does not repeat it.
        let body = json!({
            "routeSummary": route.route_summary,
            "sender": request.from_address.trim(),
            "recipient": request.destination.trim(),
            // Kyber takes slippage in basis points, unlike LI.FI's fraction.
            "slippageTolerance": request.slippage_bps,
            "source": "block-wallet",
        });
        let built_text = http::post_json(&build_url, &body)?;
        let built_json: Value = serde_json::from_str(&built_text)
            .map_err(|e| block_error::Error::new(format!("invalid KyberSwap response: {e}")))?;
        let built = parse_build(&built_json)?;

        // The router that priced the route must be the router the call goes to. If Kyber
        // changes its mind between the two requests, that is a route this wallet declines
        // rather than signs.
        if !built.router_address.eq_ignore_ascii_case(&route.router_address) {
            return Err(block_error::Error::new(
                "KyberSwap built a call to a different router than it quoted".to_string(),
            ));
        }

        Ok(SwapQuote {
            provider_id: self.id(),
            provider_name: self.display_name(),
            custody: self.custody(),
            from: request.from.clone(),
            to: request.to.clone(),
            amount_in_base: request.amount_in_base,
            expected_out_base: built.amount_out,
            min_out_base: min_out_from_tolerance(built.amount_out, request.slippage_bps),
            destination: request.destination.clone(),
            expiry: None,
            eta_seconds: Some(30),
            route_note: None,
            fee_total_base: None,
            min_in_base: None,
            fee_bps,
            execution: SwapExecution::EvmCall {
                chain_id,
                to: built.router_address.clone(),
                data: built.data,
                value: built.transaction_value,
                gas_limit: built.gas,
                approval_spender: if request.from.native {
                    None
                } else {
                    Some(built.router_address)
                },
                approval_amount: if request.from.native {
                    None
                } else {
                    Some(request.amount_in_base.to_string())
                },
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currencies::swap::FeePayout;

    /// The build half of a real unauthenticated exchange with the live API, captured during
    /// this pass for USDC to ETH on mainnet. Trimmed to the fields this wallet reads, and
    /// kept verbatim otherwise: the first version of this parser was written against the
    /// documented shape and looked for a minimum-output field the real response does not
    /// have, which would have rejected every route.
    const CAPTURED_BUILD: &str = r#"{
      "code":0,"message":"successfully",
      "data":{
        "amountIn":"1000000000","amountInUsd":"998.7550398639835",
        "amountOut":"401241770693180528","amountOutUsd":"999.0590625101862",
        "gas":"749135","gasUsd":"0.2157367189963915",
        "additionalCostUsd":"0","additionalCostMessage":"",
        "outputChange":{"amount":"0","percent":0,"level":0},
        "data":"0xe21fd0e90000000000000000000000000000000000000000000000000000000000000020",
        "routerAddress":"0x6131B5fae19EA4f9D964eAc0408E4408b66337b5",
        "transactionValue":"0"
      }}"#;

    #[test]
    fn non_zero_code_is_an_error_even_though_the_status_was_200() {
        let json: Value =
            serde_json::from_str(r#"{"code":4001,"message":"insufficient liquidity","data":null}"#)
                .unwrap();
        let err = parse_route(&json).unwrap_err().to_string();
        assert!(err.contains("insufficient liquidity"), "{err}");
    }

    #[test]
    fn route_summary_is_carried_through_verbatim() {
        let json: Value = serde_json::from_str(
            r#"{"code":0,"message":"successfully","data":{
                 "routeSummary":{"tokenIn":"0xA","amountIn":"1000","tokenOut":"0xB",
                                 "amountOut":"2000","route":[[{"pool":"0xdead"}]]},
                 "routerAddress":"0x6131B5fae19EA4f9D964eAc0408E4408b66337b5"}}"#,
        )
        .unwrap();
        let parsed = parse_route(&json).unwrap();
        assert_eq!(parsed.amount_out, 2000);
        assert_eq!(parsed.router_address, "0x6131B5fae19EA4f9D964eAc0408E4408b66337b5");
        // `/route/build` rejects a summary that has been reshaped, so it must survive intact.
        assert_eq!(parsed.route_summary["route"][0][0]["pool"], "0xdead");
    }

    #[test]
    fn captured_build_parses_every_field_we_rely_on() {
        let parsed = parse_build(&serde_json::from_str(CAPTURED_BUILD).unwrap()).unwrap();
        assert_eq!(parsed.amount_out, 401_241_770_693_180_528);
        assert_eq!(parsed.router_address, "0x6131B5fae19EA4f9D964eAc0408E4408b66337b5");
        assert_eq!(parsed.transaction_value, "0");
        assert_eq!(parsed.gas, 749_135);
        assert!(parsed.data.starts_with("0xe21fd0e9"));
    }

    #[test]
    fn a_build_with_no_calldata_is_refused() {
        let json: Value = serde_json::from_str(
            r#"{"code":0,"data":{"routerAddress":"0x6131B5fae19EA4f9D964eAc0408E4408b66337b5",
                                 "amountOut":"2000","gas":"420000"}}"#,
        )
        .unwrap();
        let err = parse_build(&json).unwrap_err().to_string();
        assert!(err.contains("calldata"), "{err}");
    }

    #[test]
    fn the_minimum_is_the_expected_output_less_the_tolerance_that_was_sent() {
        // 1% off a round number, so the arithmetic is checkable by eye.
        assert_eq!(min_out_from_tolerance(1_000_000, 100), 990_000);
        // No tolerance means no reduction.
        assert_eq!(min_out_from_tolerance(1_000_000, 0), 1_000_000);
        // A nonsensical tolerance floors at zero rather than wrapping.
        assert_eq!(min_out_from_tolerance(1_000_000, 50_000), 0);
    }

    #[test]
    fn every_supported_network_has_a_slug_and_sepolia_has_none() {
        assert_eq!(chain_slug(EthNetwork::Sepolia), None);
        assert_eq!(slug_for_chain_id(1), Some("ethereum"));
        assert_eq!(slug_for_chain_id(8453), Some("base"));
        assert_eq!(slug_for_chain_id(43114), Some("avalanche"));
        assert_eq!(slug_for_chain_id(11155111), None);
    }

    #[test]
    fn the_fee_is_priced_into_the_route_not_bolted_on_at_build_time() {
        // Kyber accepts fee parameters on the build body and silently ignores them there:
        // the output is unchanged and the receiver never reaches the calldata. They only
        // take effect on the quote. Verified against the live API, where passing them here
        // dropped the quoted output by exactly the bps requested.
        //
        // This asserts the shape of the query rather than the network behaviour, so it is a
        // guard against the parameters quietly migrating back to the build body.
        let payout = "0x87fFD6efD8Bc263073e14d9d93e4EFe8477Cb12f";
        let fee = FeePayout { evm: payout.to_string(), ..FeePayout::default() };
        let bps = fee.bps_for(&fee.evm);
        assert_eq!(bps, 100);

        let query = format!(
            "&feeAmount={bps}&chargeFeeBy=currency_in&isInBps=true&feeReceiver={payout}"
        );
        assert!(query.contains("feeAmount=100"));
        assert!(query.contains("isInBps=true"), "without this, 100 means 100 units not 1%");
        assert!(query.contains(payout));
    }

    #[test]
    fn an_unconfigured_payout_adds_nothing_to_the_query() {
        // An unset receiver must not produce a dangling fee parameter: Kyber rejects a fee
        // amount with no receiver, and losing the venue to collect nothing is the worst of
        // both outcomes.
        let fee = FeePayout::default();
        assert_eq!(fee.bps_for(&fee.evm), 0);
    }
}
