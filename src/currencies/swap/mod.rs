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
pub mod kyberswap;
pub mod lifi;
pub mod safety;
pub mod solana_tx;
pub mod thorchain;

use std::fmt;

use crate::configuration::block_error;
use crate::currencies::tokens::Token;

/// Basis points taken from each swap as an affiliate fee. 100 bps is 1%.
///
/// Charged through each venue's own affiliate mechanism rather than by this wallet moving
/// money: the venue deducts it and pays it out, so there is no extra transaction and nothing
/// for the user to sign beyond the swap itself.
///
/// Sits inside every venue's cap (THORChain and Maya allow up to 1000 bps). It is also
/// disclosed on the offer and again on the review screen before anything can be confirmed;
/// a wallet that quietly skimmed a percent would be indefensible, whatever the number.
pub const SWAP_FEE_BPS: u32 = 100;

/// Payout address shipped with the app for the EVM aggregators.
///
/// Compiled in rather than left blank so a stock install actually pays the developer; the
/// Settings field overrides it, which is what a fork or a self-build would use. EVM addresses
/// are chain-agnostic, so this one collects on all seven supported EVM networks.
///
/// Only the EVM venues get a default. The other three need accounts that cannot be guessed:
/// a Jupiter referral token account, a THORChain address and a Maya address, none of which
/// this wallet derives.
pub const DEFAULT_FEE_EVM_ADDRESS: &str = "0xBfa9D462C7560d6822A9Dc2C24818eD6CF9eeb54";

/// LI.FI integrator name, registered on the LI.FI Partner Portal for fee collection.
///
/// Separate from the EVM payout address on purpose. LI.FI does not merely ignore a fee from
/// an unregistered integrator, it **rejects the whole quote**:
///
/// ```text
/// Integrator "block-wallet" is not configured for collecting fees.
/// Please sign up on https://portal.li.fi/ and configure your fee wallet.  (code 1011)
/// ```
///
/// So sending one before registering would not cost a fee, it would cost the venue: every
/// EVM pair would lose LI.FI entirely and fall back to whatever else answered. Registered,
/// with the payout wallet configured on the portal rather than here; leave this blank in a
/// fork or self-build unless it is registered under a different name.
pub const DEFAULT_FEE_LIFI_INTEGRATOR: &str = "BlockWallet";

/// Jupiter payout. Empty, so no fee is requested on Solana swaps.
///
/// Filling this in needs a Jupiter *referral token account*, not a wallet address: a program
/// account created through their referral program, with a separate fee account per token you
/// want to collect in. Pasting an ordinary Solana address here would not work.
pub const DEFAULT_FEE_SOLANA_ACCOUNT: &str = "";

/// THORChain payout. Empty, so no fee is requested on THORChain swaps.
///
/// Needs a `thor1…` address or a registered THORName, and pays out in RUNE. A Maya address
/// is not valid here; they are separate networks.
pub const DEFAULT_FEE_THORCHAIN_ADDRESS: &str = "";

/// Maya payout. Empty, so no fee is requested on Maya swaps.
///
/// Needs a `maya1…` address or a MAYAName, and pays out in CACAO.
pub const DEFAULT_FEE_MAYA_ADDRESS: &str = "";

/// Where an affiliate fee is paid, per venue family.
///
/// An empty field means **no fee is requested from that venue at all**, and the quote is asked
/// for exactly as it was before this existed. That is the safe default in both directions: an
/// unconfigured build never charges anyone, and it never loses a venue because a fee parameter
/// was rejected.
///
/// The addresses are separate because the venues pay out on different chains and, in two
/// cases, need an account that has to be registered first:
///
/// * `evm` is an ordinary address, used by both aggregators. **LI.FI only honours a fee for a
///   registered integrator**, so setting this alone does not start charging on LI.FI.
/// * `solana` must be a Jupiter referral token account, not a wallet address.
/// * `thorchain` must be a `thor1…` address or a THORName; `maya` a `maya1…` address. They are
///   different networks and an address from one is not valid on the other.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeePayout {
    pub evm: String,
    /// LI.FI integrator name. Empty means LI.FI is asked for no fee, which is required
    /// until the integrator is registered or it refuses to quote at all.
    pub lifi_integrator: String,
    pub solana: String,
    pub thorchain: String,
    pub maya: String,
}

impl FeePayout {
    /// The fee to request from a venue, in bps. Zero when that venue has no payout address,
    /// which is what keeps an unconfigured build behaving exactly as it did before.
    pub fn bps_for(&self, address: &str) -> u32 {
        if address.trim().is_empty() {
            0
        } else {
            SWAP_FEE_BPS
        }
    }
}

/// Human-readable fee note for display, or `None` when no fee is being taken.
pub fn fee_disclosure(bps: u32) -> Option<String> {
    if bps == 0 {
        return None;
    }
    let percent = bps as f64 / 100.0;
    Some(format!("includes a {percent}% wallet fee"))
}

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
    /// A note about the route rather than its cost: which DEX it went through, or the
    /// price impact. Kept apart from the fee because it is not one, and the field these two
    /// used to share made LI.FI's "routed via sushiswap" and Jupiter's price impact look
    /// like fee figures.
    pub route_note: Option<String>,
    /// Total fee the venue will take, in the destination asset's base units.
    ///
    /// Where a venue reports this it is the whole cost, **including** the affiliate cut this
    /// wallet asked for: THORChain's `fees.total` is outbound + liquidity + affiliate. So it
    /// must never be shown alongside the wallet fee as if they were additive.
    pub fee_total_base: Option<u128>,
    /// Wallet fee actually requested from this venue, in bps. Zero when none was asked for.
    /// Displayed to the user before they can confirm.
    pub fee_bps: u32,
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

/// One line naming the total cost of the swap.
///
/// Deliberately a single figure. The venues that report a total already fold the affiliate
/// cut into it, so listing the two separately would read as more being taken than actually
/// is. Where a venue reports no total, the wallet's own percentage is the only part that can
/// be stated honestly, and it is labelled as such rather than presented as the whole cost.
pub fn swap_fee_line(quote: &SwapQuote) -> Option<String> {
    match (quote.fee_total_base, quote.fee_bps) {
        (Some(total), bps) if bps > 0 => Some(format!(
            "{} {} (includes the {}% wallet fee)",
            format_base_units(total, quote.to.decimals),
            quote.to.symbol,
            bps as f64 / 100.0
        )),
        (Some(total), _) => Some(format!(
            "{} {}",
            format_base_units(total, quote.to.decimals),
            quote.to.symbol
        )),
        (None, bps) if bps > 0 => {
            Some(format!("{}% wallet fee; this venue does not report its own", bps as f64 / 100.0))
        }
        (None, _) => None,
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
    /// Where an affiliate fee is paid, per venue. Empty fields mean no fee is requested.
    pub fee: FeePayout,
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
        Box::new(kyberswap::KyberSwap::new()),
        Box::new(jupiter::Jupiter::new()),
        Box::new(thorchain::ThorChain::new()),
        Box::new(thorchain::ThorChain::maya()),
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

    /// A minimal quote to hang fee-display assertions on. Destination is 18-decimal ETH, so
    /// the formatted figures in the tests below read in familiar units.
    fn sample_quote() -> SwapQuote {
        SwapQuote {
            provider_id: "test",
            provider_name: "Test",
            custody: Custody::AtomicOnChain,
            from: asset("btc", "BTC", 8, true),
            to: asset("eth", "ETH", 18, true),
            amount_in_base: 1_000_000,
            expected_out_base: 1_000_000_000_000_000_000,
            min_out_base: 990_000_000_000_000_000,
            destination: "0x9858EfFD232B4033E47d90003D41EC34EcaEda94".into(),
            expiry: None,
            eta_seconds: Some(30),
            route_note: None,
            fee_total_base: None,
            min_in_base: None,
            fee_bps: 0,
            execution: SwapExecution::SolanaTx { transaction_b64: String::new() },
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
            route_note: None,
            fee_total_base: None,
            min_in_base: None,
            fee_bps: 0,
            execution: SwapExecution::SolanaTx { transaction_b64: String::new() },
        };
        // 1 ETH for 4000 USDC is a rate of 1/4000.
        assert!((quote.rate() - 0.00025).abs() < 1e-12);
    }

    #[test]
    fn no_payout_address_means_no_fee_is_requested() {
        // The safe default in both directions: an unconfigured build charges nobody, and
        // never loses a venue to a fee parameter that venue would reject.
        let none = FeePayout::default();
        assert_eq!(none.bps_for(&none.evm), 0);
        assert_eq!(none.bps_for("   "), 0);
        assert_eq!(fee_disclosure(0), None);
    }

    #[test]
    fn a_configured_address_requests_exactly_one_percent() {
        let payout = FeePayout {
            evm: "0x9858EfFD232B4033E47d90003D41EC34EcaEda94".into(),
            ..FeePayout::default()
        };
        assert_eq!(payout.bps_for(&payout.evm), 100);
        assert_eq!(SWAP_FEE_BPS, 100, "1% expressed in basis points");
        // Venues are independent: configuring the EVM payout must not start charging on
        // Solana or either cross-chain network.
        assert_eq!(payout.bps_for(&payout.solana), 0);
        assert_eq!(payout.bps_for(&payout.thorchain), 0);
        assert_eq!(payout.bps_for(&payout.maya), 0);
    }

    fn quote_with(fee_total_base: Option<u128>, fee_bps: u32) -> SwapQuote {
        let mut q = sample_quote();
        q.fee_total_base = fee_total_base;
        q.fee_bps = fee_bps;
        q
    }

    #[test]
    fn the_fee_line_is_one_total_not_a_sum_of_parts() {
        // A venue that reports a total already has the affiliate cut inside it, so the line
        // shows that one figure and says the wallet fee is part of it. Adding the two would
        // overstate what is actually taken.
        let q = quote_with(Some(2_000_000_000_000_000), 100);
        let line = swap_fee_line(&q).unwrap();
        assert!(line.contains("includes the 1% wallet fee"), "{line}");
        assert!(line.starts_with("0.002 "), "{line}");
    }

    #[test]
    fn a_venue_total_with_no_wallet_fee_is_shown_alone() {
        let q = quote_with(Some(2_000_000_000_000_000), 0);
        let line = swap_fee_line(&q).unwrap();
        assert!(!line.contains("wallet fee"), "{line}");
    }

    #[test]
    fn a_venue_that_reports_no_total_says_so_rather_than_implying_one() {
        // The aggregators do not report a fee total. Showing "1%" bare would read as the
        // whole cost of the swap, which it is not.
        let q = quote_with(None, 100);
        let line = swap_fee_line(&q).unwrap();
        assert!(line.contains("does not report its own"), "{line}");
    }

    #[test]
    fn no_total_and_no_wallet_fee_shows_no_fee_line_at_all() {
        assert_eq!(swap_fee_line(&quote_with(None, 0)), None);
    }

    #[test]
    fn the_rate_stays_inside_every_venue_cap() {
        // THORChain and Maya reject an affiliate above 1000 bps outright, and a fee at that
        // scale would be indefensible anyway. This is a tripwire on the constant, not on the
        // networks.
        assert!(SWAP_FEE_BPS <= 1_000, "above the cross-chain affiliate cap");
        assert!(SWAP_FEE_BPS < 10_000, "a fee cannot be the whole swap");
    }

    #[test]
    fn the_shipped_payout_address_is_a_valid_checksummed_address() {
        // A typo here sends every EVM fee somewhere unrecoverable, and nothing else in the
        // app would notice: the venues would happily pay out to a valid-looking address that
        // nobody holds a key for. Checked against EIP-55 so a single flipped character fails
        // the build rather than the payout.
        let parsed = crate::currencies::eth_chain::validate_address(DEFAULT_FEE_EVM_ADDRESS)
            .expect("shipped payout address must parse");
        assert_eq!(
            parsed.to_checksum(None),
            DEFAULT_FEE_EVM_ADDRESS,
            "address is not in EIP-55 checksummed form, so a typo could go unnoticed"
        );
    }

    #[test]
    fn a_default_evm_payout_means_the_evm_venues_charge_out_of_the_box() {
        let payout = FeePayout {
            evm: DEFAULT_FEE_EVM_ADDRESS.to_string(),
            ..FeePayout::default()
        };
        assert_eq!(payout.bps_for(&payout.evm), SWAP_FEE_BPS);
        // And the venues with no shipped default still charge nothing until configured.
        assert_eq!(payout.bps_for(&payout.solana), 0);
        assert_eq!(payout.bps_for(&payout.thorchain), 0);
        assert_eq!(payout.bps_for(&payout.maya), 0);
    }

    #[test]
    fn an_unregistered_lifi_integrator_is_asked_for_no_fee() {
        // Sending a fee under an unregistered integrator name would not cost a fee, it would
        // cost the venue: LI.FI rejects a quote outright from an unregistered integrator
        // (code 1011), losing the venue on every EVM pair rather than earning anything.
        // Covered generically, independent of whatever the shipped default currently is.
        let fee = FeePayout {
            evm: DEFAULT_FEE_EVM_ADDRESS.to_string(),
            lifi_integrator: String::new(),
            ..FeePayout::default()
        };
        assert_eq!(
            fee.bps_for(&fee.lifi_integrator),
            0,
            "LI.FI must be asked for no fee while the integrator is unregistered"
        );
        // KyberSwap, which needs no registration, does charge from the same config.
        assert_eq!(fee.bps_for(&fee.evm), SWAP_FEE_BPS);
    }

    #[test]
    fn the_shipped_lifi_integrator_charges_from_the_registered_name() {
        // Registered on the LI.FI Partner Portal under this exact name; a mismatch here would
        // silently lose the venue's fee rather than fail loudly, so it is pinned by name
        // rather than just checked for non-emptiness.
        assert_eq!(DEFAULT_FEE_LIFI_INTEGRATOR, "BlockWallet");
        let fee = FeePayout {
            lifi_integrator: DEFAULT_FEE_LIFI_INTEGRATOR.to_string(),
            ..FeePayout::default()
        };
        assert_eq!(fee.bps_for(&fee.lifi_integrator), SWAP_FEE_BPS);
    }
}
