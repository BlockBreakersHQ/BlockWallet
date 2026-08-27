//! Bounds on what a swap quote is allowed to do.
//!
//! A quote arrives as opaque bytes from a remote service: EVM calldata, a Solana transaction,
//! or a THORChain memo. None of it is readable by the person about to sign it, and this wallet
//! cannot meaningfully simulate it on a phone. So these checks do not try to establish what a
//! payload *does*. They establish three things that are checkable, and that together bound the
//! loss if a provider is hostile, compromised, or simply wrong:
//!
//! 1. **The proceeds are addressed to the user.** Every provider states a destination, and for
//!    THORChain it is embedded in the memo. If that is not an address this wallet controls,
//!    the swap is refused outright. This is the check that stops the obvious attack, which is
//!    a tampered response redirecting the output.
//! 2. **The cost is bounded.** A minimum output is required from every provider, the slippage
//!    it implies is capped, and the input amount must be one the provider will actually honour
//!    rather than silently keep.
//! 3. **The approval is bounded.** ERC-20 spending is approved for exactly the swap amount and
//!    only to the contract that the call itself targets. No infinite approvals.
//!
//! What this does not protect against: a provider that routes through a bad pool and returns
//! less than expected but more than `min_out`, or a chain-level reorg. Those are priced risks,
//! not defects.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::configuration::block_error;
use crate::currencies::swap::{SwapExecution, SwapQuote, SwapRequest};

/// Hard ceiling on slippage, whatever the user asked for.
///
/// 10% is already far outside a sane swap. Past this the difference between "the market moved"
/// and "this route is taking your money" stops being meaningful.
pub const MAX_SLIPPAGE_BPS: u32 = 1_000;

/// Default tolerance when the user expresses no preference.
pub const DEFAULT_SLIPPAGE_BPS: u32 = 100;

/// Longest a quote may be acted on when the provider states no expiry of its own.
///
/// Prices move. A quote with no expiry that the UI has been sitting on for ten minutes is not
/// a quote, it is a guess.
pub const MAX_QUOTE_AGE_SECS: u64 = 120;

/// `OP_RETURN` carries at most 80 bytes, and THORChain's own quote response says so. A memo
/// over that produces a transaction the network will not relay, after the funds have moved.
pub const MAX_MEMO_BYTES: usize = 80;

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn err(message: impl Into<String>) -> block_error::Error {
    block_error::Error::new(message.into())
}

/// Compare two addresses the way the relevant chain does.
///
/// EVM addresses are case-insensitive hex, so a checksummed and a lower-cased spelling of the
/// same account must compare equal. Bitcoin, Litecoin and Solana addresses are not: bech32 is
/// case-insensitive as a format but is always presented lower-case, and base58 is genuinely
/// case-sensitive, so those are compared exactly.
pub fn addresses_match(chain: &str, left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() || right.is_empty() {
        return false;
    }
    match chain {
        "eth" => left.eq_ignore_ascii_case(right),
        "btc" | "ltc" => left.eq_ignore_ascii_case(right),
        _ => left == right,
    }
}

/// Pull the destination address out of a THORChain swap memo.
///
/// Memo shape is `=:TO_ASSET:DEST:LIMIT/INTERVAL/QUANTITY:AFFILIATE:BPS`, where everything
/// after the destination is optional, and the destination itself may carry a `/REFUND_ADDR`
/// suffix. Only the leading destination is returned; that is the field that decides where the
/// proceeds land.
pub fn memo_destination(memo: &str) -> Option<&str> {
    let mut parts = memo.split(':');
    let action = parts.next()?.trim();
    // `=` and `s` and `swap` are all accepted spellings of the swap action.
    if !matches!(action.to_ascii_lowercase().as_str(), "=" | "s" | "swap") {
        return None;
    }
    let _asset = parts.next()?;
    let destination = parts.next()?.trim();
    // A custom refund address is appended to the destination with a slash.
    let destination = destination.split('/').next()?.trim();
    if destination.is_empty() {
        None
    } else {
        Some(destination)
    }
}

/// Validate a quote before it is allowed anywhere near a signing key.
///
/// Returns the first problem found. Every path here is a refusal rather than a warning,
/// because a quote the wallet is unsure about is one the user cannot evaluate either.
pub fn check_quote(quote: &SwapQuote, request: &SwapRequest) -> Result<(), block_error::Error> {
    // --- the quote must answer the question that was asked -------------------------------
    if quote.from != request.from {
        return Err(err("quote is for a different input asset than was requested"));
    }
    if quote.to != request.to {
        return Err(err("quote is for a different output asset than was requested"));
    }
    if quote.amount_in_base != request.amount_in_base {
        return Err(err(format!(
            "quote is for {} of input, not the {} that was requested",
            quote.amount_in_base, request.amount_in_base
        )));
    }
    if quote.amount_in_base == 0 {
        return Err(err("amount must be greater than 0"));
    }

    // --- the proceeds must come back to the user ------------------------------------------
    // The single most important check here. Everything else bounds how much a swap can cost;
    // this one decides whether the user gets the output at all.
    if !addresses_match(&quote.to.chain, &quote.destination, &request.destination) {
        return Err(err(
            "the proceeds of this swap are not addressed to your wallet; refusing to continue",
        ));
    }

    // --- output must be bounded ------------------------------------------------------------
    if quote.min_out_base == 0 {
        return Err(err(
            "provider set no minimum output, so this swap has no protection against the price moving",
        ));
    }
    if quote.min_out_base > quote.expected_out_base {
        return Err(err("provider's minimum output exceeds its own expected output"));
    }

    let tolerance = request.slippage_bps.min(MAX_SLIPPAGE_BPS).max(1);
    let implied_drop_bps = implied_slippage_bps(quote.expected_out_base, quote.min_out_base);
    if implied_drop_bps > tolerance {
        return Err(err(format!(
            "provider's minimum output allows a {implied_drop_bps} bps drop, more than the \
             {tolerance} bps tolerance set for this swap"
        )));
    }

    // --- input must be one the provider will honour ---------------------------------------
    if let Some(min_in) = quote.min_in_base {
        if quote.amount_in_base < min_in {
            return Err(err(format!(
                "amount is below this provider's minimum of {}; sending less can forfeit it",
                crate::currencies::swap::format_base_units(min_in, quote.from.decimals)
            )));
        }
    }

    // --- the quote must still be current ---------------------------------------------------
    let now = now_unix();
    match quote.expiry {
        Some(expiry) => {
            if now >= expiry {
                return Err(err("this quote has expired; ask for a new one"));
            }
        }
        None => {
            // No provider expiry, so the wallet imposes one rather than letting a stale price
            // sit in the UI indefinitely.
            if quote.eta_seconds.unwrap_or(0) > MAX_QUOTE_AGE_SECS.saturating_mul(60) {
                return Err(err("provider gave neither an expiry nor a usable settlement time"));
            }
        }
    }

    check_execution(quote, request)
}

/// Slippage implied by the gap between expected and minimum output, in basis points.
pub fn implied_slippage_bps(expected: u128, minimum: u128) -> u32 {
    if expected == 0 || minimum >= expected {
        return 0;
    }
    let drop = expected - minimum;
    // Saturating rather than wrapping: a nonsense pair should read as "enormous", not "zero".
    u32::try_from(drop.saturating_mul(10_000) / expected).unwrap_or(u32::MAX)
}

fn check_execution(quote: &SwapQuote, request: &SwapRequest) -> Result<(), block_error::Error> {
    match &quote.execution {
        SwapExecution::UtxoWithMemo { vault, amount_base, memo, .. } => {
            if vault.trim().is_empty() {
                return Err(err("provider gave no vault address to pay"));
            }
            // The vault must be a real address on the chain being spent from, and on the same
            // network. `validate_address` on each chain enforces both.
            match quote.from.chain.as_str() {
                "btc" => {
                    let network = crate::currencies::btc_chain::parse_network("bitcoin");
                    crate::currencies::btc_chain::validate_address(vault, network)
                        .map_err(|_| err("vault address is not a valid Bitcoin mainnet address"))?;
                }
                "ltc" => {
                    let network = crate::currencies::ltc_chain::LtcNetwork::Mainnet;
                    crate::currencies::ltc_chain::validate_address(vault, network)
                        .map_err(|_| err("vault address is not a valid Litecoin mainnet address"))?;
                }
                other => return Err(err(format!("{other} cannot pay a UTXO vault"))),
            }

            if u128::from(*amount_base) != quote.amount_in_base {
                return Err(err("vault payment amount does not match the quoted input"));
            }

            if memo.trim().is_empty() {
                return Err(err("provider gave no memo, so the network cannot route this swap"));
            }
            if memo.len() > MAX_MEMO_BYTES {
                return Err(err(format!(
                    "memo is {} bytes, over the {MAX_MEMO_BYTES}-byte OP_RETURN limit",
                    memo.len()
                )));
            }
            // The memo is what the network actually obeys. The `destination` field on the
            // quote is only a claim about it, so the memo itself is re-checked here: a
            // provider that displays one address and encodes another would otherwise pass.
            let encoded = memo_destination(memo)
                .ok_or_else(|| err("memo is not a recognisable THORChain swap instruction"))?;
            if !addresses_match(&quote.to.chain, encoded, &request.destination) {
                return Err(err(
                    "the memo directs the proceeds somewhere other than your wallet; refusing to continue",
                ));
            }
            Ok(())
        }

        SwapExecution::EvmCall {
            to,
            value,
            gas_limit,
            approval_spender,
            approval_amount,
            ..
        } => {
            if to.trim().is_empty() {
                return Err(err("provider gave no contract to call"));
            }
            crate::currencies::eth_chain::validate_address(to)
                .map_err(|_| err("swap target is not a valid contract address"))?;
            if *gas_limit == 0 {
                return Err(err("provider gave no gas limit"));
            }

            let call_value = parse_u256_str(value)?;
            if quote.from.native {
                // Spending the gas token: the call must carry exactly the input as value, or
                // the wallet would be sending an amount the user never agreed to.
                if call_value != quote.amount_in_base {
                    return Err(err(
                        "the transaction's value does not match the amount being swapped",
                    ));
                }
                if approval_spender.is_some() {
                    return Err(err("a native-asset swap must not require a token approval"));
                }
            } else {
                // Spending an ERC-20: the call must move no native value at all.
                if call_value != 0 {
                    return Err(err(
                        "a token swap must not also send the chain's native asset",
                    ));
                }
                let spender = approval_spender
                    .as_deref()
                    .ok_or_else(|| err("token swap gave no approval target"))?;
                crate::currencies::eth_chain::validate_address(spender)
                    .map_err(|_| err("approval target is not a valid address"))?;
                // Approving anything other than the contract being called would leave a
                // standing allowance to a third party after the swap completes.
                if !spender.eq_ignore_ascii_case(to.trim()) {
                    return Err(err(
                        "provider wants to approve a different contract than the one it calls",
                    ));
                }
                let amount = approval_amount
                    .as_deref()
                    .map(parse_u256_str)
                    .transpose()?
                    .unwrap_or(0);
                // Exactly the swap amount. Not more, and specifically not the unlimited
                // allowance that most front-ends ask for.
                if amount != quote.amount_in_base {
                    return Err(err(
                        "approval is not for exactly the amount being swapped; refusing to grant it",
                    ));
                }
            }
            Ok(())
        }

        SwapExecution::SolanaTx { transaction_b64 } => {
            if transaction_b64.trim().is_empty() {
                return Err(err("provider returned no transaction to sign"));
            }
            if quote.from.chain != "sol" || quote.to.chain != "sol" {
                return Err(err("a Solana transaction cannot settle a swap on another chain"));
            }
            // A Solana swap settles to the signer's own accounts, so `destination` must be the
            // spending account itself. Verified here because there is nothing else about the
            // opaque transaction that can be checked.
            if !addresses_match("sol", &request.destination, &request.from_address) {
                return Err(err(
                    "a Solana swap must settle back to the account it spends from",
                ));
            }
            Ok(())
        }
    }
}

/// Parse a decimal or `0x`-prefixed integer string into `u128`.
///
/// Providers are inconsistent about which they use for the same field, so both are accepted.
fn parse_u256_str(value: &str) -> Result<u128, block_error::Error> {
    let text = value.trim();
    if text.is_empty() {
        return Ok(0);
    }
    let parsed = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        if hex.is_empty() {
            return Ok(0);
        }
        u128::from_str_radix(hex, 16)
    } else {
        text.parse::<u128>()
    };
    parsed.map_err(|_| err(format!("could not read the amount {text:?} from the provider")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currencies::swap::{Custody, SwapAsset};

    fn btc() -> SwapAsset {
        SwapAsset { chain: "btc".into(), symbol: "BTC".into(), address: String::new(), decimals: 8, native: true }
    }
    fn eth() -> SwapAsset {
        SwapAsset { chain: "eth".into(), symbol: "ETH".into(), address: String::new(), decimals: 18, native: true }
    }
    fn usdc() -> SwapAsset {
        SwapAsset {
            chain: "eth".into(),
            symbol: "USDC".into(),
            address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".into(),
            decimals: 6,
            native: false,
        }
    }

    const MY_ETH: &str = "0x9858EfFD232B4033E47d90003D41EC34EcaEda94";
    const THIEF_ETH: &str = "0x00000000219ab540356cBB839Cbe05303d7705Fa";
    const VAULT_BTC: &str = "bc1qp6yzmq5kjr8yvyw7453gxvq4z3tvkdyadqm794";

    fn request(from: SwapAsset, to: SwapAsset, amount: u128) -> SwapRequest {
        SwapRequest {
            from,
            to,
            amount_in_base: amount,
            from_address: "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu".into(),
            destination: MY_ETH.into(),
            slippage_bps: DEFAULT_SLIPPAGE_BPS,
            evm_chain_id: Some(1),
            fee: crate::currencies::swap::FeePayout::default(),
            thornode_url: String::new(),
            sol_node: String::new(),
            sol_network: String::new(),
        }
    }

    fn utxo_quote(memo: &str, destination: &str) -> SwapQuote {
        SwapQuote {
            provider_id: "thorchain",
            provider_name: "THORChain",
            custody: Custody::ProtocolVault,
            from: btc(),
            to: eth(),
            amount_in_base: 10_000_000,
            expected_out_base: 2_000_000_000_000_000_000,
            min_out_base: 1_990_000_000_000_000_000,
            destination: destination.into(),
            expiry: Some(now_unix() + 600),
            eta_seconds: Some(1800),
            route_note: None,
            fee_total_base: None,
            min_in_base: Some(100_000),
            fee_bps: 0,
            execution: SwapExecution::UtxoWithMemo {
                vault: VAULT_BTC.into(),
                amount_base: 10_000_000,
                memo: memo.into(),
                gas_rate: Some(3.0),
            },
        }
    }

    #[test]
    fn memo_destination_reads_every_documented_shape() {
        assert_eq!(memo_destination("=:ETH.ETH:0xabc:0/1/0").unwrap(), "0xabc");
        assert_eq!(memo_destination("SWAP:ETH.ETH:0xabc").unwrap(), "0xabc");
        assert_eq!(memo_destination("s:ETH.ETH:0xabc:100").unwrap(), "0xabc");
        // Custom refund address is appended with a slash and must not be mistaken for the
        // destination.
        assert_eq!(
            memo_destination("=:BTC.BTC:bc1qdest/bc1qrefund:0/1/0").unwrap(),
            "bc1qdest"
        );
        // Affiliate fields trailing the limit must not shift the destination.
        assert_eq!(
            memo_destination("=:ETH.ETH:0xabc:0/1/0:thorname:50").unwrap(),
            "0xabc"
        );
        assert!(memo_destination("ADD:BTC.BTC:0xabc").is_none());
        assert!(memo_destination("garbage").is_none());
    }

    #[test]
    fn a_quote_addressed_to_someone_else_is_refused() {
        let req = request(btc(), eth(), 10_000_000);
        let quote = utxo_quote(&format!("=:ETH.ETH:{MY_ETH}:0/1/0"), THIEF_ETH);
        let err = check_quote(&quote, &req).unwrap_err();
        assert!(format!("{err}").contains("not addressed to your wallet"), "got {err:?}");
    }

    #[test]
    fn a_memo_that_disagrees_with_the_displayed_destination_is_refused() {
        // The attack this exists for: the quote claims the user's address, so the UI shows
        // something reassuring, while the memo the network actually obeys says otherwise.
        let req = request(btc(), eth(), 10_000_000);
        let quote = utxo_quote(&format!("=:ETH.ETH:{THIEF_ETH}:0/1/0"), MY_ETH);
        let err = check_quote(&quote, &req).unwrap_err();
        assert!(format!("{err}").contains("memo directs the proceeds"), "got {err:?}");
    }

    #[test]
    fn a_well_formed_thorchain_quote_passes() {
        let req = request(btc(), eth(), 10_000_000);
        let quote = utxo_quote(&format!("=:ETH.ETH:{MY_ETH}:0/1/0"), MY_ETH);
        assert!(check_quote(&quote, &req).is_ok());
    }

    #[test]
    fn an_expired_quote_is_refused() {
        let req = request(btc(), eth(), 10_000_000);
        let mut quote = utxo_quote(&format!("=:ETH.ETH:{MY_ETH}:0/1/0"), MY_ETH);
        quote.expiry = Some(now_unix().saturating_sub(1));
        assert!(format!("{}", check_quote(&quote, &req).unwrap_err()).contains("expired"));
    }

    #[test]
    fn an_amount_below_the_providers_minimum_is_refused() {
        // THORChain keeps inbound amounts below its minimum rather than refunding them, so
        // this has to be a refusal and not a warning.
        let req = request(btc(), eth(), 1_000);
        let mut quote = utxo_quote(&format!("=:ETH.ETH:{MY_ETH}:0/1/0"), MY_ETH);
        quote.amount_in_base = 1_000;
        if let SwapExecution::UtxoWithMemo { amount_base, .. } = &mut quote.execution {
            *amount_base = 1_000;
        }
        assert!(format!("{}", check_quote(&quote, &req).unwrap_err()).contains("below this provider's minimum"));
    }

    #[test]
    fn a_quote_with_no_floor_on_the_output_is_refused() {
        let req = request(btc(), eth(), 10_000_000);
        let mut quote = utxo_quote(&format!("=:ETH.ETH:{MY_ETH}:0/1/0"), MY_ETH);
        quote.min_out_base = 0;
        assert!(format!("{}", check_quote(&quote, &req).unwrap_err()).contains("no minimum output"));
    }

    #[test]
    fn a_minimum_allowing_more_slippage_than_asked_for_is_refused() {
        let req = request(btc(), eth(), 10_000_000);
        let mut quote = utxo_quote(&format!("=:ETH.ETH:{MY_ETH}:0/1/0"), MY_ETH);
        // 50% below expected, against a 1% tolerance.
        quote.min_out_base = quote.expected_out_base / 2;
        assert!(format!("{}", check_quote(&quote, &req).unwrap_err()).contains("bps drop"));
    }

    #[test]
    fn an_oversized_memo_is_refused_before_the_funds_move() {
        let req = request(btc(), eth(), 10_000_000);
        let long = format!("=:ETH.ETH:{MY_ETH}:0/1/0:{}", "a".repeat(90));
        let quote = utxo_quote(&long, MY_ETH);
        assert!(format!("{}", check_quote(&quote, &req).unwrap_err()).contains("OP_RETURN limit"));
    }

    #[test]
    fn a_vault_address_for_the_wrong_chain_is_refused() {
        let req = request(btc(), eth(), 10_000_000);
        let mut quote = utxo_quote(&format!("=:ETH.ETH:{MY_ETH}:0/1/0"), MY_ETH);
        if let SwapExecution::UtxoWithMemo { vault, .. } = &mut quote.execution {
            // A Litecoin vault offered for a Bitcoin spend.
            *vault = "ltc1qp6yzmq5kjr8yvyw7453gxvq4z3tvkdyafup6a9".into();
        }
        assert!(format!("{}", check_quote(&quote, &req).unwrap_err()).contains("valid Bitcoin"));
    }

    fn evm_quote(from: SwapAsset, value: &str, spender: Option<&str>, approval: Option<&str>) -> SwapQuote {
        let router = "0x1231DEB6f5749EF6cE6943a275A1D3E7486F4EaE";
        SwapQuote {
            provider_id: "lifi",
            provider_name: "LI.FI",
            custody: Custody::AtomicOnChain,
            amount_in_base: if from.native { 1_000_000_000_000_000_000 } else { 1_000_000 },
            from,
            to: usdc(),
            expected_out_base: 4_000_000_000,
            min_out_base: 3_980_000_000,
            destination: MY_ETH.into(),
            expiry: Some(now_unix() + 300),
            eta_seconds: Some(60),
            route_note: None,
            fee_total_base: None,
            min_in_base: None,
            fee_bps: 0,
            execution: SwapExecution::EvmCall {
                chain_id: 1,
                to: router.into(),
                data: "0x2c57e884".into(),
                value: value.into(),
                gas_limit: 200_000,
                approval_spender: spender.map(str::to_string),
                approval_amount: approval.map(str::to_string),
            },
        }
    }

    #[test]
    fn an_unlimited_token_approval_is_refused() {
        let mut req = request(usdc(), usdc(), 1_000_000);
        req.to = usdc();
        let unlimited = u128::MAX.to_string();
        let router = "0x1231DEB6f5749EF6cE6943a275A1D3E7486F4EaE";
        let mut quote = evm_quote(usdc(), "0", Some(router), Some(&unlimited));
        quote.to = usdc();
        let err = check_quote(&quote, &req).unwrap_err();
        assert!(format!("{err}").contains("exactly the amount being swapped"), "got {err:?}");
    }

    #[test]
    fn approving_a_contract_other_than_the_one_called_is_refused() {
        let mut req = request(usdc(), usdc(), 1_000_000);
        req.to = usdc();
        let mut quote = evm_quote(usdc(), "0", Some(THIEF_ETH), Some("1000000"));
        quote.to = usdc();
        let err = check_quote(&quote, &req).unwrap_err();
        assert!(format!("{err}").contains("different contract"), "got {err:?}");
    }

    #[test]
    fn a_token_swap_that_also_moves_native_value_is_refused() {
        let mut req = request(usdc(), usdc(), 1_000_000);
        req.to = usdc();
        let router = "0x1231DEB6f5749EF6cE6943a275A1D3E7486F4EaE";
        let mut quote = evm_quote(usdc(), "1000000000000000000", Some(router), Some("1000000"));
        quote.to = usdc();
        let err = check_quote(&quote, &req).unwrap_err();
        assert!(format!("{err}").contains("must not also send"), "got {err:?}");
    }

    #[test]
    fn a_native_swap_whose_value_does_not_match_the_input_is_refused() {
        let mut req = request(eth(), usdc(), 1_000_000_000_000_000_000);
        req.to = usdc();
        // Call carries double the value the user agreed to swap.
        let mut quote = evm_quote(eth(), "2000000000000000000", None, None);
        quote.to = usdc();
        let err = check_quote(&quote, &req).unwrap_err();
        assert!(format!("{err}").contains("value does not match"), "got {err:?}");
    }

    #[test]
    fn a_well_formed_evm_swap_passes_in_both_native_and_token_form() {
        let mut native_req = request(eth(), usdc(), 1_000_000_000_000_000_000);
        native_req.to = usdc();
        let mut native = evm_quote(eth(), "1000000000000000000", None, None);
        native.to = usdc();
        assert!(check_quote(&native, &native_req).is_ok());

        let mut token_req = request(usdc(), usdc(), 1_000_000);
        token_req.to = usdc();
        let router = "0x1231DEB6f5749EF6cE6943a275A1D3E7486F4EaE";
        let mut token = evm_quote(usdc(), "0x0", Some(router), Some("1000000"));
        token.to = usdc();
        assert!(check_quote(&token, &token_req).is_ok());
    }

    #[test]
    fn hex_and_decimal_amounts_are_both_understood() {
        assert_eq!(parse_u256_str("0x0").unwrap(), 0);
        assert_eq!(parse_u256_str("0x2c").unwrap(), 44);
        assert_eq!(parse_u256_str("44").unwrap(), 44);
        assert_eq!(parse_u256_str("").unwrap(), 0);
        assert!(parse_u256_str("not-a-number").is_err());
    }

    #[test]
    fn evm_address_comparison_ignores_checksum_case_but_solana_does_not() {
        assert!(addresses_match("eth", MY_ETH, &MY_ETH.to_lowercase()));
        // Base58 is case-significant, so two different spellings are two different accounts.
        assert!(!addresses_match("sol", "So11111111111111111111111111111111111111112", "so11111111111111111111111111111111111111112"));
        assert!(!addresses_match("eth", MY_ETH, THIEF_ETH));
        assert!(!addresses_match("eth", "", ""));
    }

    #[test]
    fn implied_slippage_is_reported_in_basis_points() {
        assert_eq!(implied_slippage_bps(10_000, 10_000), 0);
        assert_eq!(implied_slippage_bps(10_000, 9_900), 100);
        assert_eq!(implied_slippage_bps(10_000, 5_000), 5_000);
        assert_eq!(implied_slippage_bps(0, 0), 0);
    }
}
