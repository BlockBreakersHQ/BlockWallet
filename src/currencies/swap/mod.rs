//! Swaps, across several providers, without giving anyone custody of your coins.
//!
//! Two shapes of swap live behind one interface here:
//!
//! * **Same-chain DEX aggregation** — Ethereum and its L2s via LI.FI, Solana via Jupiter. The
//!   aggregator returns a transaction; this wallet signs it locally and broadcasts it. Nobody
//!   holds the funds; they move from your account to a router contract and back in one
//!   transaction.
//! * **Cross-chain via THORChain** — the only way to swap BTC or LTC at all, since neither has
//!   an on-chain DEX. You pay a THORChain vault and the network settles the other side to your
//!   own address. Still not custodial in the "a company holds your coins" sense, but it is a
//!   real protocol dependency with real failure modes, so it gets the strictest checks.
//!
//! # What is actually being trusted
//!
//! Every provider here hands back either opaque EVM calldata, an opaque Solana transaction, or
//! a memo string. None of it is readable by the user, and all of it arrives over the network.
//! That is the honest security position of local-signed swapping, and it is why
//! [`safety`] exists: the checks there do not try to understand what the payload *does*, they
//! bound what it can *cost* and verify that the proceeds are addressed to the user. See
//! [`safety::check_quote`] for the full list.

pub mod execute;
pub mod jupiter;
pub mod lifi;
pub mod safety;
pub mod solana_tx;
pub mod thorchain;

use std::fmt;

use crate::configuration::block_error;
use crate::currencies::tokens::Token;

/// An asset a swap can move, identified consistently across providers.
///
/// Deliberately separate from [`Token`]: a `Token` is something the wallet displays a balance
/// for, whereas a `SwapAsset` needs a stable cross-provider identity and an explicit
/// `native` flag, because "is this the chain's gas token" changes how every provider encodes
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapAsset {
    /// Internal chain key: `btc`, `eth`, `sol` or `ltc`. Matches `Token::chain`.
    pub chain: String,
    pub symbol: String,
    /// Contract or mint address. Empty for a chain's native asset.
    pub address: String,
    pub decimals: u8,
    pub native: bool,
}

impl SwapAsset {
    pub fn from_token(token: &Token) -> Self {
        let native = match token.chain.as_str() {
            "eth" => crate::currencies::eth_chain::is_native_token(token),
            "sol" => crate::currencies::sol_chain::is_native_token(token),
            // BTC and LTC have no token layer here, so their only asset is the native one.
            _ => true,
        };
        Self {
            chain: token.chain.clone(),
            symbol: token.symbol.clone(),
            address: if native { String::new() } else { token.address.trim().to_string() },
            decimals: token.decimals.max(0) as u8,
            native,
        }
    }

    /// THORChain's asset notation: `CHAIN.SYMBOL` for native assets, `CHAIN.SYMBOL-ADDRESS`
    /// for contract assets, with the address upper-cased as THORChain expects.
    pub fn thorchain_notation(&self) -> Option<String> {
        let chain = match self.chain.as_str() {
            "btc" => "BTC",
            "ltc" => "LTC",
            "eth" => "ETH",
            // THORChain has no Solana pool in the form this wallet would use.
            _ => return None,
        };
        if self.native {
            let symbol = match self.chain.as_str() {
                "btc" => "BTC",
                "ltc" => "LTC",
                _ => "ETH",
            };
            Some(format!("{chain}.{symbol}"))
        } else {
            Some(format!(
                "{chain}.{}-{}",
                self.symbol.to_uppercase(),
                self.address.to_uppercase()
            ))
        }
    }

    /// THORChain works in fixed 1e8 units regardless of the asset's own precision.
    ///
    /// Converting through `u128` because 1e18-decimal amounts overflow `u64` long before
    /// they reach anything a person would swap.
    pub fn to_thorchain_units(&self, amount_base: u128) -> u128 {
        let decimals = self.decimals as u32;
        if decimals >= 8 {
            amount_base / 10u128.pow(decimals - 8)
        } else {
            amount_base * 10u128.pow(8 - decimals)
        }
    }

    /// Inverse of [`Self::to_thorchain_units`], for turning a quoted output back into the
    /// destination asset's own base units.
    pub fn from_thorchain_units(&self, amount_1e8: u128) -> u128 {
        let decimals = self.decimals as u32;
        if decimals >= 8 {
            amount_1e8 * 10u128.pow(decimals - 8)
        } else {
            amount_1e8 / 10u128.pow(8 - decimals)
        }
    }
}

impl fmt::Display for SwapAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.symbol, self.chain.to_uppercase())
    }
}

/// Who, if anyone, holds the funds while the swap is in flight.
///
/// Surfaced in the UI rather than kept internal: the difference between "this settles in one
/// transaction" and "a protocol vault holds this for twenty minutes" is exactly the kind of
/// thing a person swapping their own money should be told before they agree to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Custody {
    /// Funds leave and return within a single signed transaction.
    AtomicOnChain,
    /// A protocol vault holds the inbound side until the outbound side settles. No company
    /// takes ownership, but the funds are out of the user's control in the meantime.
    ProtocolVault,
}

impl Custody {
    pub fn describe(&self) -> &'static str {
        match self {
            Self::AtomicOnChain => "Settles in one transaction. Your funds are never held by anyone.",
            Self::ProtocolVault => {
                "Paid into a THORChain vault, which settles the other side to your address. \
                 Not held by a company, but out of your control until it completes."
            }
        }
    }
}

/// What the wallet has to do on-chain to execute a quote.
#[derive(Clone, Debug, PartialEq)]
pub enum SwapExecution {
    /// Pay a vault on a UTXO chain and attach the memo as an `OP_RETURN` output.
    ///
    /// THORChain requires a specific output ordering, which its own quote response spells
    /// out: vault first, change second, `OP_RETURN` third.
    UtxoWithMemo {
        vault: String,
        amount_base: u64,
        memo: String,
        /// Provider's recommended sat/vB. Still passed through the wallet's own fee ceiling.
        gas_rate: Option<f32>,
    },
    /// An EVM contract call the provider built. Opaque calldata, bounded by the checks in
    /// [`safety`] rather than by reading it.
    EvmCall {
        chain_id: u64,
        to: String,
        data: String,
        value: String,
        gas_limit: u64,
        /// ERC-20 spender that must be approved first, if the input is not the gas token.
        approval_spender: Option<String>,
        approval_amount: Option<String>,
    },
    /// A provider-built Solana transaction that this wallet only signs.
    SolanaTx { transaction_b64: String },
}

impl SwapExecution {
    /// The chain this execution happens on, as an internal chain key.
    pub fn chain(&self) -> &'static str {
        match self {
            Self::UtxoWithMemo { .. } => "utxo",
            Self::EvmCall { .. } => "eth",
            Self::SolanaTx { .. } => "sol",
        }
    }
}

/// A priced, executable swap offer from one provider.
#[derive(Clone, Debug, PartialEq)]
pub struct SwapQuote {
    pub provider_id: &'static str,
    pub provider_name: &'static str,
    pub custody: Custody,
    pub from: SwapAsset,
    pub to: SwapAsset,
    /// Input, in the source asset's own base units (sats / wei / lamports).
    pub amount_in_base: u128,
    /// Expected output, in the destination asset's base units.
    pub expected_out_base: u128,
    /// Worst acceptable output. Below this the swap must not proceed.
    ///
    /// Every provider supplies one, because without it a quote is just a suggestion and the
    /// user has no protection against the price moving between quoting and settlement.
    pub min_out_base: u128,
    /// The address the proceeds are directed to. Checked against the user's own address.
    pub destination: String,
    /// Unix seconds after which the quote must not be acted on. `None` where the provider
    /// gives no expiry, in which case [`safety`] applies its own age limit.
    pub expiry: Option<u64>,
    /// Rough end-to-end time, for display.
    pub eta_seconds: Option<u64>,
    /// Provider-reported total fee, in the destination asset's base units, for display.
    pub fee_note: Option<String>,
    /// Smallest input the provider will honour, in source base units. Sending less than this
    /// to THORChain is how people lose money to it, so it is enforced rather than displayed.
    pub min_in_base: Option<u128>,
    pub execution: SwapExecution,
}

impl SwapQuote {
    /// Effective price, as destination units per source unit, for comparing offers.
    pub fn rate(&self) -> f64 {
        let from_scale = 10f64.powi(self.from.decimals as i32);
        let to_scale = 10f64.powi(self.to.decimals as i32);
        let inp = self.amount_in_base as f64 / from_scale;
        let out = self.expected_out_base as f64 / to_scale;
        if inp <= 0.0 {
            return 0.0;
        }
        out / inp
    }

    /// Human-readable expected output.
    pub fn expected_out_display(&self) -> String {
        format_base_units(self.expected_out_base, self.to.decimals)
    }

    pub fn min_out_display(&self) -> String {
        format_base_units(self.min_out_base, self.to.decimals)
    }

    pub fn amount_in_display(&self) -> String {
        format_base_units(self.amount_in_base, self.from.decimals)
    }
}

/// Render a base-unit integer as a decimal string, trimming trailing zeros.
pub fn format_base_units(amount: u128, decimals: u8) -> String {
    let decimals = decimals as usize;
    if decimals == 0 {
        return amount.to_string();
    }
    let raw = format!("{amount:0>width$}", width = decimals + 1);
    let split = raw.len() - decimals;
    let whole = &raw[..split];
    let frac = raw[split..].trim_end_matches('0');
    if frac.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{frac}")
    }
}

/// What the caller asks a provider for.
#[derive(Clone, Debug)]
pub struct SwapRequest {
    pub from: SwapAsset,
    pub to: SwapAsset,
    pub amount_in_base: u128,
    /// The user's own address on the source chain, spending the input.
    pub from_address: String,
    /// The user's own address on the destination chain, receiving the output.
    pub destination: String,
    /// Slippage tolerance in basis points. Bounded by [`safety::MAX_SLIPPAGE_BPS`].
    pub slippage_bps: u32,
    /// Which EVM network the Ethereum-family assets live on, so an aggregator is asked about
    /// the chain the wallet is actually pointed at rather than always mainnet. `None` on a
    /// network with no aggregator liquidity worth quoting, such as a testnet.
    pub evm_chain_id: Option<u64>,
    /// THORNode endpoint override, so a user running their own infrastructure is not forced
    /// onto a public default just because they are swapping.
    pub thornode_url: String,
    /// Solana RPC override, used when a provider needs to build against a specific cluster.
    pub sol_node: String,
    /// Solana cluster name ("mainnet" / "devnet"). Separate from `sol_node`, which is a URL:
    /// `sol_chain::parse_network` matches on the name, so feeding it the URL silently
    /// resolved to mainnet and made the devnet guard unreachable.
    pub sol_network: String,
}

/// One swap venue.
///
/// Blocking rather than async: the rest of this codebase does network work on a plain thread
/// and reports back through `ui_channel`, and a swap is not special enough to justify a
/// second concurrency model.
pub trait SwapProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn custody(&self) -> Custody;

    /// Whether this provider can even attempt the pair. Checked before quoting so the UI can
    /// say "no route" without four pointless network calls.
    fn supports(&self, from: &SwapAsset, to: &SwapAsset) -> bool;

    /// Ask for a price. Network call.
    fn quote(&self, request: &SwapRequest) -> Result<SwapQuote, block_error::Error>;
}

/// Every provider this build knows about.
///
/// Order is the tie-break when two providers quote an identical rate, so the atomic
/// same-chain venues come before the vault-based one.
pub fn providers() -> Vec<Box<dyn SwapProvider>> {
    vec![
        Box::new(lifi::LiFi::new()),
        Box::new(jupiter::Jupiter::new()),
        Box::new(thorchain::ThorChain::new()),
    ]
}

/// The outcome of asking every provider at once.
#[derive(Clone, Debug)]
pub struct QuoteSet {
    /// Accepted quotes, best rate first.
    pub quotes: Vec<SwapQuote>,
    /// Why the others are missing, for display. Providers failing is normal, not exceptional:
    /// a halted chain or an unsupported pair should be explained rather than hidden.
    pub rejected: Vec<(&'static str, String)>,
}

impl QuoteSet {
    pub fn best(&self) -> Option<&SwapQuote> {
        self.quotes.first()
    }
}

/// Ask every provider that supports the pair, validate each answer, and rank what survives.
///
/// A provider that errors, or that returns a quote failing [`safety::check_quote`], is
/// dropped with its reason recorded rather than being allowed to reach the UI. A bad quote is
/// not a lesser quote; it is one that must never be signable.
pub fn collect_quotes(request: &SwapRequest) -> QuoteSet {
    let mut quotes = Vec::new();
    let mut rejected = Vec::new();

    // Providers that cannot route the pair are settled without a network call.
    let mut candidates = Vec::new();
    for provider in providers() {
        if provider.supports(&request.from, &request.to) {
            candidates.push(provider);
        } else {
            rejected.push((provider.id(), "does not route this pair".to_string()));
        }
    }

    // Queried in parallel. Each provider has its own timeout budget, so asking them one after
    // another would make the user wait for the sum of the slow ones rather than the slowest.
    // `thread::scope` keeps the borrow of `request` valid without cloning it per provider.
    let results: Vec<(&'static str, Result<SwapQuote, block_error::Error>)> =
        std::thread::scope(|scope| {
            let handles: Vec<_> = candidates
                .iter()
                .map(|provider| {
                    let id = provider.id();
                    (id, scope.spawn(move || provider.quote(request)))
                })
                .collect();
            handles
                .into_iter()
                .map(|(id, handle)| {
                    let outcome = handle.join().unwrap_or_else(|_| {
                        Err(block_error::Error::new(
                            "provider lookup failed unexpectedly".to_string(),
                        ))
                    });
                    (id, outcome)
                })
                .collect()
        });

    for (id, result) in results {
        match result {
            Ok(quote) => match safety::check_quote(&quote, request) {
                Ok(()) => quotes.push(quote),
                Err(why) => rejected.push((id, format!("{why}"))),
            },
            Err(why) => rejected.push((id, format!("{why}"))),
        }
    }

    // Best expected output first. Ties fall back to registry order, which puts the atomic
    // venues ahead of the vault-based one.
    quotes.sort_by(|a, b| {
        b.rate()
            .partial_cmp(&a.rate())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    QuoteSet { quotes, rejected }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(chain: &str, symbol: &str, decimals: u8, native: bool) -> SwapAsset {
        SwapAsset {
            chain: chain.to_string(),
            symbol: symbol.to_string(),
            address: if native { String::new() } else { "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".into() },
            decimals,
            native,
        }
    }

    #[test]
    fn thorchain_notation_matches_the_protocols_spelling() {
        assert_eq!(asset("btc", "BTC", 8, true).thorchain_notation().unwrap(), "BTC.BTC");
        assert_eq!(asset("ltc", "LTC", 8, true).thorchain_notation().unwrap(), "LTC.LTC");
        assert_eq!(asset("eth", "ETH", 18, true).thorchain_notation().unwrap(), "ETH.ETH");
        assert_eq!(
            asset("eth", "USDC", 6, false).thorchain_notation().unwrap(),
            "ETH.USDC-0XA0B86991C6218B36C1D19D4A2E9EB0CE3606EB48"
        );
        // Solana has no THORChain route in this wallet's shape.
        assert!(asset("sol", "SOL", 9, true).thorchain_notation().is_none());
    }

    #[test]
    fn thorchain_unit_conversion_round_trips_across_decimal_widths() {
        let btc = asset("btc", "BTC", 8, true);
        assert_eq!(btc.to_thorchain_units(100_000_000), 100_000_000);

        // 1 ETH is 1e18 wei but 1e8 THORChain units.
        let eth = asset("eth", "ETH", 18, true);
        assert_eq!(eth.to_thorchain_units(1_000_000_000_000_000_000), 100_000_000);
        assert_eq!(eth.from_thorchain_units(100_000_000), 1_000_000_000_000_000_000);

        // 6-decimal USDC scales the other way.
        let usdc = asset("eth", "USDC", 6, false);
        assert_eq!(usdc.to_thorchain_units(1_000_000), 100_000_000);
        assert_eq!(usdc.from_thorchain_units(100_000_000), 1_000_000);
    }

    #[test]
    fn base_unit_formatting_trims_without_losing_precision() {
        assert_eq!(format_base_units(100_000_000, 8), "1");
        assert_eq!(format_base_units(100_050_000, 8), "1.0005");
        assert_eq!(format_base_units(1, 8), "0.00000001");
        assert_eq!(format_base_units(0, 8), "0");
        assert_eq!(format_base_units(1_500_000, 6), "1.5");
        assert_eq!(format_base_units(42, 0), "42");
    }

    #[test]
    fn rate_compares_across_different_decimal_widths() {
        let quote = SwapQuote {
            provider_id: "test",
            provider_name: "Test",
            custody: Custody::AtomicOnChain,
            from: asset("eth", "USDC", 6, false),
            to: asset("eth", "ETH", 18, true),
            amount_in_base: 4_000_000_000,               // 4000 USDC
            expected_out_base: 1_000_000_000_000_000_000, // 1 ETH
            min_out_base: 990_000_000_000_000_000,
            destination: "0x0".into(),
            expiry: None,
            eta_seconds: None,
            fee_note: None,
            min_in_base: None,
            execution: SwapExecution::SolanaTx { transaction_b64: String::new() },
        };
        // 1 ETH for 4000 USDC is a rate of 1/4000.
        assert!((quote.rate() - 0.00025).abs() < 1e-12);
    }
}
