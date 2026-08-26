use std::collections::BTreeMap;
use std::str::FromStr;

use alloy::network::TransactionBuilder;
use alloy::primitives::{Address, B256, Bytes, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::{Filter, TransactionRequest};
use alloy::signers::local::PrivateKeySigner;
use serde::{Deserialize, Serialize};

use crate::configuration::block_error;
use crate::currencies::fees::clamp_gas_price;
use crate::currencies::tokens::Token;

const TRANSFER_TOPIC: B256 = B256::new([
    0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
    0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
]);
const SELECTOR_BALANCE_OF: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];
const SELECTOR_TRANSFER: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];
const SELECTOR_DECIMALS: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
const SELECTOR_SYMBOL: [u8; 4] = [0x95, 0xd8, 0x9b, 0x41];
const SELECTOR_NAME: [u8; 4] = [0x06, 0xfd, 0xde, 0x03];

const NATIVE_SENTINEL: &str = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const LOG_LOOKBACK: u64 = 1_500;
const HISTORY_CAP: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EthNetwork {
    Mainnet,
    Sepolia,
    ArbitrumOne,
    Base,
    Optimism,
    PolygonPos,
    BnbSmartChain,
    AvalancheCChain,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegistryToken {
    pub symbol: String,
    pub name: String,
    pub address: String,
    pub decimals: u8,
    pub native: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeeTiers {
    pub low: u128,
    pub medium: u128,
    pub high: u128,
    pub priority: u128,
}

impl Default for FeeTiers {
    fn default() -> Self {
        Self {
            low: 1_000_000_000,
            medium: 1_500_000_000,
            high: 2_500_000_000,
            priority: 100_000_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EthHistoryItem {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub symbol: String,
    pub amount: String,
    pub incoming: bool,
    pub confirmations: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EthSyncState {
    pub eth_wei: U256,
    pub receive_address: String,
    pub erc20: BTreeMap<String, String>,
    pub history: Vec<EthHistoryItem>,
    pub offline: bool,
    pub native_symbol: String,
}

impl EthSyncState {
    pub fn balance_display(&self) -> String {
        if self.offline {
            return format!("{} {} (offline)", format_units_trimmed(self.eth_wei, 18), self.native_symbol);
        }
        format!("{} {}", format_units_trimmed(self.eth_wei, 18), self.native_symbol)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSend {
    /// The account this plan was built against, and whose nonce `nonce` holds.
    ///
    /// `sign_and_broadcast` refuses a key that resolves to anything else. Without this the
    /// UI's re-read of the account dropdown at confirm time could pair a plan with a
    /// different account's key: if the two happened to share a next-nonce — routine for
    /// freshly derived accounts, which both sit at 0 — the transaction would be perfectly
    /// valid and would debit an account the user never reviewed.
    pub from: String,
    pub to: String,
    pub token_symbol: String,
    pub token_address: Option<String>,
    pub amount: U256,
    pub amount_display: String,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
    pub fee_wei: U256,
    pub fee_symbol: String,
    pub chain_id: u64,
    pub nonce: u64,
}

impl PreparedSend {
    pub fn summary(&self) -> String {
        let fee = format_units_trimmed(self.fee_wei, 18);
        if self.token_address.is_none() {
            let total = self.amount.saturating_add(self.fee_wei);
            format!(
                "Network: chain {}\nTo: {}\nAmount: {} {}\nMax fee: {} {}\nTotal (amount + max fee): {} {}",
                self.chain_id,
                self.to,
                self.amount_display,
                self.fee_symbol,
                fee,
                self.fee_symbol,
                format_units_trimmed(total, 18),
                self.fee_symbol
            )
        } else {
            format!(
                "Network: chain {}\nTo: {}\nAmount: {} {}\nMax fee: {} {} (paid in {})",
                self.chain_id,
                self.to,
                self.amount_display,
                self.token_symbol,
                fee,
                self.fee_symbol,
                self.fee_symbol
            )
        }
    }
}

pub fn parse_network(name: &str) -> EthNetwork {
    match name.trim().to_ascii_lowercase().as_str() {
        "sepolia" | "testnet" | "test" => EthNetwork::Sepolia,
        "arbitrum" | "arbitrum-one" | "arb" => EthNetwork::ArbitrumOne,
        "base" => EthNetwork::Base,
        "optimism" | "op" | "op-mainnet" => EthNetwork::Optimism,
        "polygon" | "polygon-pos" | "matic" => EthNetwork::PolygonPos,
        "bsc" | "bnb" | "bnb-smart-chain" | "binance" => EthNetwork::BnbSmartChain,
        "avalanche" | "avax" | "avalanche-c-chain" => EthNetwork::AvalancheCChain,
        _ => EthNetwork::Mainnet,
    }
}

pub fn network_name(network: EthNetwork) -> &'static str {
    match network {
        EthNetwork::Sepolia => "sepolia",
        EthNetwork::Mainnet => "mainnet",
        EthNetwork::ArbitrumOne => "arbitrum",
        EthNetwork::Base => "base",
        EthNetwork::Optimism => "optimism",
        EthNetwork::PolygonPos => "polygon",
        EthNetwork::BnbSmartChain => "bsc",
        EthNetwork::AvalancheCChain => "avalanche",
    }
}

/// True only for known real testnets. Used to gate the "this spends real value" send
/// confirmation — anything that isn't a testnet (including every L2/sidechain) must show it.
pub fn is_testnet(network: EthNetwork) -> bool {
    matches!(network, EthNetwork::Sepolia)
}

pub fn chain_id(network: EthNetwork) -> u64 {
    match network {
        EthNetwork::Mainnet => 1,
        EthNetwork::Sepolia => 11155111,
        EthNetwork::ArbitrumOne => 42161,
        EthNetwork::Base => 8453,
        EthNetwork::Optimism => 10,
        EthNetwork::PolygonPos => 137,
        EthNetwork::BnbSmartChain => 56,
        EthNetwork::AvalancheCChain => 43114,
    }
}

/// Public endpoints used when the user has not supplied their own.
///
/// Health-checked with `eth_chainId` against each chain's declared id. Two were replaced after
/// going dead: `eth.llamarpc.com` returns HTTP 521, and `polygon-rpc.com` now answers 401 and
/// requires a key. Both had made their networks completely non-functional (no balance, no
/// history, no sending, no ENS) while looking like ordinary connectivity failures. publicnode
/// serves both and was already the Sepolia default.
///
/// Worth re-checking periodically: a dead default is indistinguishable from being offline
/// unless someone looks.
pub fn default_rpc(network: EthNetwork) -> &'static str {
    match network {
        EthNetwork::Mainnet => "https://ethereum-rpc.publicnode.com",
        EthNetwork::Sepolia => "https://ethereum-sepolia-rpc.publicnode.com",
        EthNetwork::ArbitrumOne => "https://arb1.arbitrum.io/rpc",
        EthNetwork::Base => "https://mainnet.base.org",
        EthNetwork::Optimism => "https://mainnet.optimism.io",
        EthNetwork::PolygonPos => "https://polygon-bor-rpc.publicnode.com",
        EthNetwork::BnbSmartChain => "https://bsc-dataseed.binance.org",
        EthNetwork::AvalancheCChain => "https://api.avax.network/ext/bc/C/rpc",
    }
}

/// The chain's native gas token symbol. Not always "ETH" — L2s/sidechains with their own
/// native asset (Polygon, BNB Smart Chain, Avalanche C-Chain) use their own symbol.
pub fn native_symbol(network: EthNetwork) -> &'static str {
    match network {
        EthNetwork::PolygonPos => "MATIC",
        EthNetwork::BnbSmartChain => "BNB",
        EthNetwork::AvalancheCChain => "AVAX",
        _ => "ETH",
    }
}

pub fn resolve_rpc(eth_node: &str, network: EthNetwork, infura_key: &str) -> String {
    let node = eth_node.trim();
    if !node.is_empty() {
        return node.trim_end_matches('/').to_string();
    }
    let key = infura_key.trim();
    if !key.is_empty() {
        // Infura's URL scheme isn't uniform across every L2 we support (and doesn't cover all
        // of them); only use it for the two networks it's verified for here, default RPC
        // otherwise.
        match network {
            EthNetwork::Mainnet => return format!("https://mainnet.infura.io/v3/{key}"),
            EthNetwork::Sepolia => return format!("https://sepolia.infura.io/v3/{key}"),
            _ => {}
        }
    }
    default_rpc(network).to_string()
}

pub fn bundled_tokens(network: EthNetwork) -> Vec<RegistryToken> {
    let symbol = native_symbol(network);
    let name = match network {
        EthNetwork::PolygonPos => "Polygon",
        EthNetwork::BnbSmartChain => "BNB",
        EthNetwork::AvalancheCChain => "Avalanche",
        _ => "Ethereum",
    };
    let mut tokens = vec![RegistryToken {
        symbol: symbol.into(),
        name: name.into(),
        address: NATIVE_SENTINEL.into(),
        decimals: 18,
        native: true,
    }];
    match network {
        EthNetwork::Mainnet => {
            tokens.extend([
                erc20("USDC", "USD Coin", "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", 6),
                erc20("USDT", "Tether USD", "0xdac17f958d2ee523a2206206994597c13d831ec7", 6),
                erc20("DAI", "Dai Stablecoin", "0x6b175474e89094c44da98b954eedeac495271d0f", 18),
                erc20("WBTC", "Wrapped BTC", "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599", 8),
            ]);
        }
        EthNetwork::Sepolia => {
            tokens.push(erc20(
                "USDC",
                "USD Coin",
                "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238",
                6,
            ));
        }
        EthNetwork::ArbitrumOne => {
            tokens.push(erc20("USDC", "USD Coin", "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6));
        }
        EthNetwork::Base => {
            tokens.push(erc20("USDC", "USD Coin", "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", 6));
        }
        EthNetwork::Optimism => {
            tokens.push(erc20("USDC", "USD Coin", "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85", 6));
        }
        EthNetwork::PolygonPos => {
            tokens.push(erc20("USDC", "USD Coin", "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359", 6));
        }
        EthNetwork::BnbSmartChain => {
            // Circle does not issue native USDC on BSC; bundle Binance-Peg USDT instead.
            // BSC's USDT contract uses 18 decimals, unlike Ethereum's 6.
            tokens.push(erc20("USDT", "Tether USD (BSC)", "0x55d398326f99059fF775485246999027B3197955", 18));
        }
        EthNetwork::AvalancheCChain => {
            tokens.push(erc20("USDC", "USD Coin", "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E", 6));
        }
    }
    tokens
}

fn encode_address_arg(address: Address) -> Vec<u8> {
    let mut out = vec![0u8; 32];
    out[12..].copy_from_slice(address.as_slice());
    out
}

fn encode_u256_arg(amount: U256) -> [u8; 32] {
    amount.to_be_bytes::<32>()
}

fn encode_balance_of(account: Address) -> Bytes {
    let mut data = Vec::with_capacity(36);
    data.extend_from_slice(&SELECTOR_BALANCE_OF);
    data.extend_from_slice(&encode_address_arg(account));
    Bytes::from(data)
}

fn encode_transfer(to: Address, amount: U256) -> Bytes {
    let mut data = Vec::with_capacity(68);
    data.extend_from_slice(&SELECTOR_TRANSFER);
    data.extend_from_slice(&encode_address_arg(to));
    data.extend_from_slice(&encode_u256_arg(amount));
    Bytes::from(data)
}

fn encode_selector(selector: [u8; 4]) -> Bytes {
    Bytes::from(selector.to_vec())
}

fn decode_u256(bytes: &[u8]) -> U256 {
    if bytes.is_empty() {
        return U256::ZERO;
    }
    U256::from_be_slice(bytes)
}

fn decode_string(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 64 {
        return std::str::from_utf8(bytes)
            .ok()
            .map(|s| s.trim_matches('\0').trim().to_string())
            .filter(|s| !s.is_empty());
    }
    let len = U256::from_be_slice(&bytes[32..64]).try_into().ok()?;
    let start = 64usize;
    let end = start.saturating_add(len);
    if end > bytes.len() {
        return None;
    }
    String::from_utf8(bytes[start..end].to_vec()).ok()
}

fn address_topic(address: Address) -> B256 {
    let mut topic = [0u8; 32];
    topic[12..].copy_from_slice(address.as_slice());
    B256::from(topic)
}

async fn eth_call<P: Provider>(provider: &P, to: Address, data: Bytes) -> Result<Bytes, block_error::Error> {
    let tx = TransactionRequest::default().with_to(to).with_input(data);
    provider
        .call(tx)
        .await
        .map_err(|e| block_error::Error::new(format!("eth_call failed: {e}")))
}

fn erc20(symbol: &str, name: &str, address: &str, decimals: u8) -> RegistryToken {
    RegistryToken {
        symbol: symbol.into(),
        name: name.into(),
        address: address.into(),
        decimals,
        native: false,
    }
}

pub fn is_native_token(token: &Token) -> bool {
    token.symbol.eq_ignore_ascii_case("ETH")
        || token.address.trim().eq_ignore_ascii_case(NATIVE_SENTINEL)
        || token.address.trim().is_empty()
}

pub fn validate_address(address: &str) -> Result<Address, block_error::Error> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(block_error::Error::new("receive address is required".into()));
    }
    if trimmed.to_ascii_lowercase().contains(".eth") {
        return Err(block_error::Error::new("ENS names are not supported yet".into()));
    }
    Address::from_str(trimmed)
        .map_err(|e| block_error::Error::new(format!("invalid ethereum address: {e}")))
}

pub fn parse_token_amount(input: &str, decimals: u8) -> Result<U256, block_error::Error> {
    let s = crate::currencies::amount::normalize_decimal_input(input)?;
    let (whole, frac) = match s.split_once('.') {
        Some((whole, frac)) => (whole, frac),
        None => (s.as_str(), ""),
    };
    if frac.len() > decimals as usize {
        return Err(block_error::Error::new(format!(
            "amount has more than {decimals} decimal places"
        )));
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(block_error::Error::new("amount must be a number".into()));
    }
    let whole = if whole.is_empty() { "0" } else { whole };
    let mut frac = frac.to_string();
    while frac.len() < decimals as usize {
        frac.push('0');
    }
    let combined = format!("{whole}{frac}");
    let combined = combined.trim_start_matches('0');
    let combined = if combined.is_empty() { "0" } else { combined };
    U256::from_str(combined).map_err(|e| block_error::Error::new(format!("amount is too large: {e}")))
}

pub fn format_units_trimmed(amount: U256, decimals: u8) -> String {
    if amount.is_zero() {
        return "0".to_string();
    }
    let mut raw = amount.to_string();
    let decimals = decimals as usize;
    if decimals == 0 {
        return raw;
    }
    if raw.len() <= decimals {
        raw = format!("{:0>width$}", raw, width = decimals + 1);
    }
    let split = raw.len() - decimals;
    let whole = &raw[..split];
    let frac = raw[split..].trim_end_matches('0');
    if frac.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{frac}")
    }
}

/// Below this the fee-versus-amount rule is switched off: a dust-sized send legitimately
/// costs more in gas than it moves. 0.001 ETH, in wei.
const FEE_RATIO_FLOOR_WEI: u128 = 1_000_000_000_000_000;

/// The wei-denominated counterpart to [`crate::currencies::fees::check_fee_is_sane`]. Kept
/// here rather than in that module because it needs `U256`: a fee above ~18.4 ETH does not
/// fit in a `u64`, and saturating into one would let exactly the largest fees through.
fn check_native_fee_is_sane(fee_wei: U256, amount: U256) -> Result<(), block_error::Error> {
    if fee_wei > amount && fee_wei > U256::from(FEE_RATIO_FLOOR_WEI) {
        return Err(block_error::Error::new(format!(
            "network fee ({fee_wei} wei) is larger than the amount being sent ({amount} wei); \
             check the fee tier and the node this wallet is pointed at"
        )));
    }
    Ok(())
}

pub fn fee_from_tier(tiers: &FeeTiers, label: &str) -> (u128, u128) {
    let max_fee = match label.to_ascii_lowercase().as_str() {
        "low" => tiers.low,
        "high" => tiers.high,
        _ => tiers.medium,
    };
    // Both bounded: the estimate is whatever the node said, and `max_fee_per_gas` multiplied
    // by the gas limit is the ceiling on what this transaction can cost.
    (clamp_gas_price(max_fee), clamp_gas_price(tiers.priority))
}

fn block_on<T>(fut: impl std::future::Future<Output = T>) -> Result<T, block_error::Error> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| block_error::Error::new(format!("tokio runtime: {e}")))?
        .block_on(async move { Ok(fut.await) })
}

fn http_provider(rpc: &str) -> Result<impl Provider + Clone, block_error::Error> {
    let url = rpc
        .parse()
        .map_err(|e| block_error::Error::new(format!("invalid ETH RPC URL: {e}")))?;
    Ok(ProviderBuilder::new().connect_http(url))
}

fn signed_provider(
    rpc: &str,
    signer: PrivateKeySigner,
) -> Result<impl Provider + Clone, block_error::Error> {
    let url = rpc
        .parse()
        .map_err(|e| block_error::Error::new(format!("invalid ETH RPC URL: {e}")))?;
    Ok(ProviderBuilder::new().wallet(signer).connect_http(url))
}

fn signer_from_key(private_key: &str) -> Result<PrivateKeySigner, block_error::Error> {
    let mut key = private_key.trim().to_string();
    if let Some(stripped) = key.strip_prefix("0x").or_else(|| key.strip_prefix("0X")) {
        key = stripped.to_string();
    }
    PrivateKeySigner::from_str(&key)
        .or_else(|_| PrivateKeySigner::from_str(&format!("0x{key}")))
        .map_err(|e| block_error::Error::new(format!("invalid ethereum private key: {e}")))
}

pub fn sync_account(
    address: &str,
    eth_node: &str,
    network_name: &str,
    infura_key: &str,
    tokens: &[Token],
    etherscan_key: &str,
) -> Result<EthSyncState, block_error::Error> {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(eth_node, network, infura_key);
    let account = validate_address(address)?;
    match block_on(sync_account_async(
        account,
        rpc,
        network,
        tokens.to_vec(),
        etherscan_key.to_string(),
    )) {
        Ok(Ok(state)) => Ok(state),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err),
    }
}

async fn sync_account_async(
    account: Address,
    rpc: String,
    network: EthNetwork,
    tokens: Vec<Token>,
    etherscan_key: String,
) -> Result<EthSyncState, block_error::Error> {
    let symbol = native_symbol(network).to_string();
    let provider = match http_provider(&rpc) {
        Ok(provider) => provider,
        Err(_) => {
            return Ok(EthSyncState {
                eth_wei: U256::ZERO,
                receive_address: format!("{account:?}"),
                erc20: BTreeMap::new(),
                history: Vec::new(),
                offline: true,
                native_symbol: symbol,
            });
        }
    };

    let eth_wei = match provider.get_balance(account).await {
        Ok(value) => value,
        Err(_) => {
            return Ok(EthSyncState {
                eth_wei: U256::ZERO,
                receive_address: format!("{account:?}"),
                erc20: BTreeMap::new(),
                history: Vec::new(),
                offline: true,
                native_symbol: symbol,
            });
        }
    };

    let mut erc20 = BTreeMap::new();
    for token in &tokens {
        if is_native_token(token) {
            continue;
        }
        let Ok(contract_addr) = Address::from_str(token.address.trim()) else {
            continue;
        };
        if let Ok(raw) = eth_call(&provider, contract_addr, encode_balance_of(account)).await {
            let balance = decode_u256(raw.as_ref());
            let decimals = token.decimals.max(0) as u8;
            erc20.insert(token.symbol.clone(), format_units_trimmed(balance, decimals));
        }
    }

    // Native transfers move no tokens, so they emit no logs and `erc20_history` cannot see
    // them. They need an indexer. Etherscan is used when a key is configured; otherwise
    // Blockscout, which serves the same Etherscan-compatible shape without one.
    //
    // This used to be gated on the key alone, so a wallet with no Etherscan key simply had no
    // native history at all — the Activity list stayed empty however much ETH arrived or was
    // sent, which read as a bug in sending rather than a missing data source.
    let mut history = erc20_history(&provider, account, &tokens).await;
    let native = if !etherscan_key.trim().is_empty() {
        native_history_etherscan(account, chain_id(network), &etherscan_key, &symbol)
    } else {
        native_history_blockscout(account, network, &symbol)
    };
    match native {
        Ok(rows) => history.extend(rows),
        Err(why) => {
            crate::configuration::logging::warn(&format!("native history unavailable: {why}"))
        }
    }
    history.sort_by(|a, b| b.confirmations.cmp(&a.confirmations).then(b.txid.cmp(&a.txid)));
    history.truncate(HISTORY_CAP);

    Ok(EthSyncState {
        eth_wei,
        receive_address: format!("{account:?}"),
        erc20,
        history,
        offline: false,
        native_symbol: symbol,
    })
}

async fn erc20_history<P: Provider>(
    provider: &P,
    account: Address,
    tokens: &[Token],
) -> Vec<EthHistoryItem> {
    let Ok(latest) = provider.get_block_number().await else {
        return Vec::new();
    };
    let from_block = latest.saturating_sub(LOG_LOOKBACK);
    let mut items = Vec::new();
    for token in tokens {
        if is_native_token(token) {
            continue;
        }
        let Ok(contract_addr) = Address::from_str(token.address.trim()) else {
            continue;
        };
        let incoming = Filter::new()
            .address(contract_addr)
            .event_signature(TRANSFER_TOPIC)
            .from_block(from_block)
            .topic2(address_topic(account));
        let outgoing = Filter::new()
            .address(contract_addr)
            .event_signature(TRANSFER_TOPIC)
            .from_block(from_block)
            .topic1(address_topic(account));
        for (filter, incoming) in [(incoming, true), (outgoing, false)] {
            let Ok(logs) = provider.get_logs(&filter).await else {
                continue;
            };
            for log in logs {
                let Some(topics) = (log.topics().len() >= 3).then_some(log.topics()) else {
                    continue;
                };
                let from = Address::from_slice(&topics[1].as_slice()[12..]);
                let to = Address::from_slice(&topics[2].as_slice()[12..]);
                let amount = U256::from_be_slice(log.data().data.as_ref());
                let confirmations = log
                    .block_number
                    .map(|block| latest.saturating_sub(block).saturating_add(1) as u32)
                    .unwrap_or(0);
                items.push(EthHistoryItem {
                    txid: log.transaction_hash.map(|h| format!("{h:#x}")).unwrap_or_default(),
                    from: format!("{from:?}"),
                    to: format!("{to:?}"),
                    symbol: token.symbol.clone(),
                    amount: format_units_trimmed(amount, token.decimals.max(0) as u8),
                    incoming,
                    confirmations,
                });
            }
        }
    }
    items
}

/// Blockscout instance serving each network's Etherscan-compatible `txlist` API.
///
/// Used when no Etherscan key is configured. Blockscout answers unauthenticated and returns
/// the same JSON shape, so the parser below is shared. `None` where no public instance is
/// known, in which case native history is simply unavailable and says so.
pub fn blockscout_base(network: EthNetwork) -> Option<&'static str> {
    match network {
        EthNetwork::Mainnet => Some("https://eth.blockscout.com"),
        EthNetwork::Sepolia => Some("https://eth-sepolia.blockscout.com"),
        EthNetwork::ArbitrumOne => Some("https://arbitrum.blockscout.com"),
        EthNetwork::Base => Some("https://base.blockscout.com"),
        EthNetwork::Optimism => Some("https://optimism.blockscout.com"),
        EthNetwork::PolygonPos => Some("https://polygon.blockscout.com"),
        // Blockscout has no public instance this wallet can rely on for these two.
        EthNetwork::BnbSmartChain | EthNetwork::AvalancheCChain => None,
    }
}

fn native_history_blockscout(
    account: Address,
    network: EthNetwork,
    native_symbol: &str,
) -> Result<Vec<EthHistoryItem>, block_error::Error> {
    let base = blockscout_base(network).ok_or_else(|| {
        block_error::Error::new(
            "no keyless history source for this network; add an Etherscan API key in Settings"
                .to_string(),
        )
    })?;
    let url = format!(
        "{base}/api?module=account&action=txlist&address={account:?}&page=1&offset=20&sort=desc"
    );
    let text = crate::configuration::http::get_text(&url)?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    Ok(parse_txlist(&json, account, native_symbol))
}

fn native_history_etherscan(
    account: Address,
    chain_id: u64,
    api_key: &str,
    native_symbol: &str,
) -> Result<Vec<EthHistoryItem>, block_error::Error> {
    let url = format!(
        "https://api.etherscan.io/v2/api?chainid={chain_id}&module=account&action=txlist&address={account:?}&page=1&offset=20&sort=desc&apikey={api_key}"
    );
    let text = crate::configuration::http::get_text(&url)?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    Ok(parse_txlist(&json, account, native_symbol))
}

/// Parse the Etherscan-compatible `txlist` response shared by Etherscan and Blockscout.
fn parse_txlist(
    json: &serde_json::Value,
    account: Address,
    native_symbol: &str,
) -> Vec<EthHistoryItem> {
    let Some(rows) = json.get("result").and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    let account_lower = format!("{account:?}").to_ascii_lowercase();
    let mut items = Vec::new();
    for row in rows {
        let hash = row.get("hash").and_then(|v| v.as_str()).unwrap_or_default();
        let from = row.get("from").and_then(|v| v.as_str()).unwrap_or_default();
        let to = row.get("to").and_then(|v| v.as_str()).unwrap_or_default();
        let value = row
            .get("value")
            .and_then(|v| v.as_str())
            .and_then(|v| U256::from_str(v).ok())
            .unwrap_or(U256::ZERO);
        if value.is_zero() {
            continue;
        }
        let confirmations = row
            .get("confirmations")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let incoming = to.to_ascii_lowercase() == account_lower;
        items.push(EthHistoryItem {
            txid: hash.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            symbol: native_symbol.to_string(),
            amount: format_units_trimmed(value, 18),
            incoming,
            confirmations,
        });
    }
    items
}

pub fn fetch_fee_tiers(eth_node: &str, network_name: &str, infura_key: &str) -> FeeTiers {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(eth_node, network, infura_key);
    block_on(fetch_fee_tiers_async(rpc))
        .ok()
        .flatten()
        .unwrap_or_default()
}

async fn fetch_fee_tiers_async(rpc: String) -> Option<FeeTiers> {
    let provider = http_provider(&rpc).ok()?;
    if let Ok(est) = provider.estimate_eip1559_fees().await {
        let medium = est.max_fee_per_gas.max(1);
        let priority = est.max_priority_fee_per_gas.max(1);
        return Some(FeeTiers {
            low: medium.saturating_mul(80) / 100,
            medium,
            high: medium.saturating_mul(130) / 100,
            priority,
        });
    }
    let gas = provider.get_gas_price().await.ok()?;
    Some(FeeTiers {
        low: gas.saturating_mul(80) / 100,
        medium: gas,
        high: gas.saturating_mul(130) / 100,
        priority: gas / 10,
    })
}

pub fn prepare_send(
    from: &str,
    to: &str,
    amount_text: &str,
    token: &Token,
    eth_node: &str,
    network_name: &str,
    infura_key: &str,
    fee_label: &str,
) -> Result<PreparedSend, block_error::Error> {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(eth_node, network, infura_key);
    let from = validate_address(from)?;
    let to = validate_address(to)?;
    let decimals = if is_native_token(token) {
        18
    } else {
        token.decimals.max(0) as u8
    };
    let amount = parse_token_amount(amount_text, decimals)?;
    if amount.is_zero() {
        return Err(block_error::Error::new("amount must be greater than 0".into()));
    }
    let token = token.clone();
    match block_on(prepare_send_async(
        from,
        to,
        amount,
        token,
        rpc,
        chain_id(network),
        fee_label.to_string(),
        native_symbol(network).to_string(),
    )) {
        Ok(Ok(plan)) => Ok(plan),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err),
    }
}

async fn prepare_send_async(
    from: Address,
    to: Address,
    amount: U256,
    token: Token,
    rpc: String,
    chain_id: u64,
    fee_label: String,
    fee_symbol: String,
) -> Result<PreparedSend, block_error::Error> {
    let provider = http_provider(&rpc)?;
    let native = is_native_token(&token);
    let nonce = provider
        .get_transaction_count(from)
        .await
        .map_err(|e| block_error::Error::new(format!("could not read nonce: {e}")))?;
    let eth_balance = provider
        .get_balance(from)
        .await
        .map_err(|_| block_error::Error::new("ethereum node is unreachable".into()))?;

    let mut tx = TransactionRequest::default()
        .with_from(from)
        .with_chain_id(chain_id)
        .with_nonce(nonce);
    if native {
        tx = tx.with_to(to).with_value(amount);
    } else {
        let contract = Address::from_str(token.address.trim())
            .map_err(|e| block_error::Error::new(format!("invalid token contract: {e}")))?;
        tx = tx.with_to(contract).with_input(encode_transfer(to, amount));
        let raw = eth_call(&provider, contract, encode_balance_of(from)).await?;
        let token_balance = decode_u256(raw.as_ref());
        if token_balance < amount {
            return Err(block_error::Error::new(format!(
                "not enough {} to send",
                token.symbol
            )));
        }
    }

    let gas_limit = provider
        .estimate_gas(tx.clone())
        .await
        .unwrap_or(if native { 21_000 } else { 65_000 });
    let gas_limit = gas_limit.saturating_add(gas_limit / 5).max(21_000);

    let tiers = fetch_fee_tiers_async(rpc.clone()).await.unwrap_or_default();
    let (max_fee_per_gas, max_priority_fee_per_gas) = fee_from_tier(&tiers, &fee_label);
    let fee_wei = U256::from(gas_limit) * U256::from(max_fee_per_gas);
    if eth_balance < fee_wei {
        return Err(block_error::Error::new(format!(
            "not enough {fee_symbol} to cover the network fee"
        )));
    }
    if native && eth_balance < amount.saturating_add(fee_wei) {
        return Err(block_error::Error::new(format!("not enough {fee_symbol} to send")));
    }

    // A native send is directly comparable: both sides are wei. For a token send the amount
    // is in token units and the fee is in the gas token, so there is nothing meaningful to
    // compare — the gas-price ceiling in `fee_from_tier` is the guard that applies there.
    if native {
        check_native_fee_is_sane(fee_wei, amount)?;
    }

    let decimals = if native { 18 } else { token.decimals.max(0) as u8 };
    Ok(PreparedSend {
        from: format!("{from:?}"),
        to: format!("{to:?}"),
        token_symbol: token.symbol,
        token_address: if native {
            None
        } else {
            Some(token.address)
        },
        amount,
        amount_display: format_units_trimmed(amount, decimals),
        gas_limit,
        max_fee_per_gas,
        max_priority_fee_per_gas,
        fee_wei,
        fee_symbol,
        chain_id,
        nonce,
    })
}

pub fn sign_and_broadcast(
    private_key: &str,
    plan: &PreparedSend,
    eth_node: &str,
    network_name: &str,
    infura_key: &str,
) -> Result<String, block_error::Error> {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(eth_node, network, infura_key);
    let signer = signer_from_key(private_key)?;
    let plan = plan.clone();
    match block_on(sign_and_broadcast_async(signer, plan, rpc)) {
        Ok(Ok(hash)) => Ok(hash),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err),
    }
}

async fn sign_and_broadcast_async(
    signer: PrivateKeySigner,
    plan: PreparedSend,
    rpc: String,
) -> Result<String, block_error::Error> {
    let from = signer.address();
    let expected_from = validate_address(&plan.from)?;
    if from != expected_from {
        return Err(block_error::Error::new(
            "this key does not belong to the account the transaction was reviewed for".to_string(),
        ));
    }
    let to = validate_address(&plan.to)?;
    if plan.token_address.is_none() {
        check_native_fee_is_sane(plan.fee_wei, plan.amount)?;
    }
    let provider = signed_provider(&rpc, signer)?;
    let mut tx = TransactionRequest::default()
        .with_from(from)
        .with_chain_id(plan.chain_id)
        .with_nonce(plan.nonce)
        .with_gas_limit(plan.gas_limit)
        .with_max_fee_per_gas(plan.max_fee_per_gas)
        .with_max_priority_fee_per_gas(plan.max_priority_fee_per_gas);
    if let Some(contract) = &plan.token_address {
        let contract = Address::from_str(contract.trim())
            .map_err(|e| block_error::Error::new(format!("invalid token contract: {e}")))?;
        tx = tx
            .with_to(contract)
            .with_input(encode_transfer(to, plan.amount));
    } else {
        tx = tx.with_to(to).with_value(plan.amount);
    }
    let pending = provider.send_transaction(tx).await.map_err(|e| {
        block_error::Error::new(format!("ethereum broadcast failed: {e}"))
    })?;
    Ok(format!("{:#x}", pending.tx_hash()))
}

/// Resolve an ENS name to an address.
///
/// Always queried against the chain that actually hosts the registry, which for every network
/// except Sepolia is mainnet, regardless of where the wallet is currently pointed. The result
/// is a plain address and is valid on any EVM chain.
pub fn resolve_ens(
    name: &str,
    eth_node: &str,
    network_name: &str,
    infura_key: &str,
) -> Result<Address, block_error::Error> {
    use crate::currencies::ens;

    let normalized = ens::normalize(name)?;
    let node = ens::namehash(&normalized);

    let wallet_network = parse_network(network_name);
    let registry_network = ens::registry_network(wallet_network);
    // A user-supplied RPC is only reused when it is already pointed at the chain the registry
    // lives on. Otherwise the built-in endpoint for that chain is used, since querying a Base
    // RPC for a mainnet registry entry would simply find nothing.
    let rpc = if registry_network == wallet_network {
        resolve_rpc(eth_node, wallet_network, infura_key)
    } else {
        default_rpc(registry_network).to_string()
    };

    let registry = Address::from_str(ens::ENS_REGISTRY)
        .map_err(|_| block_error::Error::new("bad ENS registry address".to_string()))?;

    match block_on(async move {
        let provider = http_provider(&rpc)?;

        let resolver_raw = eth_call(&provider, registry, Bytes::from(ens::encode_resolver_call(node))).await?;
        let resolver = ens::decode_address_word(resolver_raw.as_ref()).ok_or_else(|| {
            block_error::Error::new(format!("{normalized} has no ENS resolver"))
        })?;

        let addr_raw = eth_call(&provider, resolver, Bytes::from(ens::encode_addr_call(node))).await?;
        ens::decode_address_word(addr_raw.as_ref()).ok_or_else(|| {
            block_error::Error::new(format!("{normalized} does not resolve to an address"))
        })
    }) {
        Ok(Ok(address)) => Ok(address),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err),
    }
}

/// Turn whatever was typed into a recipient address.
///
/// Accepts a raw `0x…` address unchanged, or resolves an ENS name. Returned alongside the
/// resolved address is the name it came from, when there was one, so the UI can show the user
/// what a name actually resolved to before they confirm.
pub fn resolve_recipient(
    input: &str,
    eth_node: &str,
    network_name: &str,
    infura_key: &str,
) -> Result<(Address, Option<String>), block_error::Error> {
    use crate::currencies::ens;

    if ens::looks_like_name(input) {
        let address = resolve_ens(input, eth_node, network_name, infura_key)?;
        Ok((address, Some(input.trim().to_ascii_lowercase())))
    } else {
        Ok((validate_address(input)?, None))
    }
}

/// ABI-encode `approve(address,uint256)`.
///
/// Selector is asserted against its own keccak hash in the tests, rather than trusted from
/// memory, because an approval sent to the wrong function signature either reverts or does
/// something unintended with the user's token balance.
pub fn encode_approve(spender: Address, amount: U256) -> Bytes {
    const SELECTOR_APPROVE: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
    let mut data = Vec::with_capacity(4 + 64);
    data.extend_from_slice(&SELECTOR_APPROVE);
    data.extend_from_slice(&B256::left_padding_from(spender.as_slice())[..]);
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    Bytes::from(data)
}

/// Read the current ERC-20 allowance from `owner` to `spender`.
pub fn fetch_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    eth_node: &str,
    network_name: &str,
    infura_key: &str,
) -> Result<U256, block_error::Error> {
    const SELECTOR_ALLOWANCE: [u8; 4] = [0xdd, 0x62, 0xed, 0x3e];
    let network = parse_network(network_name);
    let rpc = resolve_rpc(eth_node, network, infura_key);
    let contract = Address::from_str(token.trim())
        .map_err(|e| block_error::Error::new(format!("invalid token contract: {e}")))?;
    let owner = validate_address(owner)?;
    let spender = validate_address(spender)?;

    let mut data = Vec::with_capacity(4 + 64);
    data.extend_from_slice(&SELECTOR_ALLOWANCE);
    data.extend_from_slice(&B256::left_padding_from(owner.as_slice())[..]);
    data.extend_from_slice(&B256::left_padding_from(spender.as_slice())[..]);

    match block_on(async move {
        let provider = http_provider(&rpc)?;
        let raw = eth_call(&provider, contract, Bytes::from(data)).await?;
        Ok::<U256, block_error::Error>(decode_u256(raw.as_ref()))
    }) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err),
    }
}

/// Send an arbitrary contract call, signed locally.
///
/// This is what executes a swap: an aggregator or the THORChain router hands back calldata
/// this wallet cannot read, and the protections around it are the checks in
/// `swap::safety` plus the caller's own gas ceiling, not any inspection of `data`.
///
/// `chain_id` is taken from the plan rather than the current network setting, so a
/// transaction built for one chain can never be replayed onto another.
#[allow(clippy::too_many_arguments)]
pub fn send_contract_call(
    private_key: &str,
    to: &str,
    data: &str,
    value: U256,
    gas_limit: u64,
    chain_id: u64,
    eth_node: &str,
    network_name: &str,
    infura_key: &str,
) -> Result<String, block_error::Error> {
    let network = parse_network(network_name);
    if chain_id != self::chain_id(network) {
        return Err(block_error::Error::new(
            "this transaction was built for a different network than the wallet is on"
                .to_string(),
        ));
    }
    let rpc = resolve_rpc(eth_node, network, infura_key);
    let signer = signer_from_key(private_key)?;
    let target = validate_address(to)?;
    let payload = decode_hex_payload(data)?;

    match block_on(send_contract_call_async(
        signer, target, payload, value, gas_limit, chain_id, rpc,
    )) {
        Ok(Ok(hash)) => Ok(hash),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err),
    }
}

fn decode_hex_payload(data: &str) -> Result<Bytes, block_error::Error> {
    let text = data.trim();
    let stripped = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")).unwrap_or(text);
    if stripped.is_empty() {
        return Ok(Bytes::new());
    }
    let bytes = hex::decode(stripped)
        .map_err(|_| block_error::Error::new("provider returned unreadable calldata".to_string()))?;
    Ok(Bytes::from(bytes))
}

async fn send_contract_call_async(
    signer: PrivateKeySigner,
    to: Address,
    data: Bytes,
    value: U256,
    gas_limit: u64,
    chain_id: u64,
    rpc: String,
) -> Result<String, block_error::Error> {
    let from = signer.address();
    let provider = signed_provider(&rpc, signer)?;
    let nonce = provider
        .get_transaction_count(from)
        .await
        .map_err(|e| block_error::Error::new(format!("could not read nonce: {e}")))?;
    let tiers = fetch_fee_tiers_async(rpc.clone()).await.unwrap_or_default();
    let (max_fee_per_gas, max_priority_fee_per_gas) = fee_from_tier(&tiers, "medium");

    let tx = TransactionRequest::default()
        .with_from(from)
        .with_to(to)
        .with_chain_id(chain_id)
        .with_nonce(nonce)
        .with_gas_limit(gas_limit)
        .with_max_fee_per_gas(max_fee_per_gas)
        .with_max_priority_fee_per_gas(max_priority_fee_per_gas)
        .with_value(value)
        .with_input(data);

    let pending = provider
        .send_transaction(tx)
        .await
        .map_err(|e| block_error::Error::new(format!("broadcast failed: {e}")))?;
    Ok(format!("{:#x}", pending.tx_hash()))
}

pub fn fetch_token_metadata(
    contract: &str,
    eth_node: &str,
    network_name: &str,
    infura_key: &str,
) -> Result<RegistryToken, block_error::Error> {
    let address = validate_address(contract)?;
    let network = parse_network(network_name);
    let rpc = resolve_rpc(eth_node, network, infura_key);
    match block_on(fetch_token_metadata_async(address, rpc)) {
        Ok(Ok(token)) => Ok(token),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err),
    }
}

async fn fetch_token_metadata_async(
    address: Address,
    rpc: String,
) -> Result<RegistryToken, block_error::Error> {
    let provider = http_provider(&rpc)?;
    let decimals_raw = eth_call(&provider, address, encode_selector(SELECTOR_DECIMALS)).await?;
    let decimals: u8 = decode_u256(decimals_raw.as_ref())
        .try_into()
        .map_err(|_| block_error::Error::new("token decimals() was not a u8".into()))?;
    let symbol = eth_call(&provider, address, encode_selector(SELECTOR_SYMBOL))
        .await
        .ok()
        .and_then(|raw| decode_string(raw.as_ref()))
        .unwrap_or_else(|| format!("{address:?}")[..10].to_string());
    let name = eth_call(&provider, address, encode_selector(SELECTOR_NAME))
        .await
        .ok()
        .and_then(|raw| decode_string(raw.as_ref()))
        .unwrap_or_else(|| symbol.clone());
    Ok(RegistryToken {
        symbol,
        name,
        address: format!("{address:?}"),
        decimals,
        native: false,
    })
}

pub fn apply_bundled_tokens(tokens: &mut crate::currencies::tokens::Tokens, network: EthNetwork) {
    for item in bundled_tokens(network) {
        let logo = crate::configuration::paths::token_icon_path(&item.symbol);
        tokens.eth_tokens.insert(
            format!("eth:{}", item.symbol),
            Token {
                name: item.name,
                symbol: item.symbol,
                address: item.address,
                logo,
                decimals: item.decimals as i32,
                chain: "eth".to_string(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_networks_and_default_rpcs() {
        assert_eq!(parse_network("sepolia"), EthNetwork::Sepolia);
        assert_eq!(parse_network(""), EthNetwork::Mainnet);
        assert_eq!(chain_id(EthNetwork::Sepolia), 11155111);
        // Asserts the shape rather than one vendor's hostname; the previous version pinned
        // "llama" and so had to be edited when that endpoint died.
        for network in [EthNetwork::Mainnet, EthNetwork::Sepolia] {
            let rpc = resolve_rpc("", network, "");
            assert!(rpc.starts_with("https://"), "{network:?} default is not https: {rpc}");
        }
        assert!(resolve_rpc("", EthNetwork::Sepolia, "").contains("sepolia"));
        assert_eq!(
            resolve_rpc("https://my.node", EthNetwork::Mainnet, "abc"),
            "https://my.node"
        );
        assert!(resolve_rpc("", EthNetwork::Mainnet, "abc").contains("infura.io/v3/abc"));
    }

    #[test]
    fn l2_networks_have_correct_chain_ids_and_native_symbols() {
        let cases = [
            ("arbitrum", EthNetwork::ArbitrumOne, 42161u64, "ETH"),
            ("base", EthNetwork::Base, 8453, "ETH"),
            ("optimism", EthNetwork::Optimism, 10, "ETH"),
            ("polygon", EthNetwork::PolygonPos, 137, "MATIC"),
            ("bsc", EthNetwork::BnbSmartChain, 56, "BNB"),
            ("avalanche", EthNetwork::AvalancheCChain, 43114, "AVAX"),
        ];
        for (name, network, expected_chain_id, expected_symbol) in cases {
            assert_eq!(parse_network(name), network, "{name}");
            assert_eq!(chain_id(network), expected_chain_id, "{name}");
            assert_eq!(native_symbol(network), expected_symbol, "{name}");
            assert_eq!(network_name(network), name);
            assert!(!is_testnet(network), "{name} must not be treated as a testnet");
            assert!(!default_rpc(network).is_empty(), "{name}");
        }
        assert!(is_testnet(EthNetwork::Sepolia));
        assert!(!is_testnet(EthNetwork::Mainnet));
    }

    #[test]
    fn l2_bundled_tokens_use_native_symbol_and_no_evm_collides_with_another() {
        for network in [
            EthNetwork::ArbitrumOne,
            EthNetwork::Base,
            EthNetwork::Optimism,
            EthNetwork::PolygonPos,
            EthNetwork::BnbSmartChain,
            EthNetwork::AvalancheCChain,
        ] {
            let tokens = bundled_tokens(network);
            let native = tokens.iter().find(|t| t.native).expect("native entry");
            assert_eq!(native.symbol, native_symbol(network));
            assert_eq!(native.address, NATIVE_SENTINEL);
            // Every non-native bundled entry must be a distinct, non-native-sentinel contract.
            for t in tokens.iter().filter(|t| !t.native) {
                assert_ne!(t.address, NATIVE_SENTINEL);
            }
        }
        // BSC's bundled stablecoin uses 18 decimals (Binance-Peg USDT), unlike the 6-decimal
        // USDC bundled on the other L2s/sidechains.
        let bsc = bundled_tokens(EthNetwork::BnbSmartChain);
        let bsc_stable = bsc.iter().find(|t| !t.native).unwrap();
        assert_eq!(bsc_stable.symbol, "USDT");
        assert_eq!(bsc_stable.decimals, 18);
    }

    #[test]
    fn validates_addresses_and_rejects_ens() {
        let addr = validate_address("0x9858Eff28F61CF0aDe1AC00482789d2EF5e6d47E").unwrap();
        assert_eq!(format!("{addr:?}").len(), 42);
        assert!(validate_address("not-an-address").is_err());
        assert!(validate_address("vitalik.eth").is_err());
        assert!(validate_address("").is_err());
    }

    #[test]
    fn parses_and_formats_token_amounts() {
        assert_eq!(parse_token_amount("1", 18).unwrap(), U256::from(10).pow(U256::from(18)));
        assert_eq!(
            parse_token_amount("1.5", 6).unwrap(),
            U256::from(1_500_000u64)
        );
        assert_eq!(parse_token_amount(".5", 2).unwrap(), U256::from(50u64));
        assert!(parse_token_amount("1.2345678", 6).is_err());
        assert!(parse_token_amount("", 18).is_err());
        assert_eq!(format_units_trimmed(U256::from(1_500_000u64), 6), "1.5");
        assert_eq!(format_units_trimmed(U256::from(10).pow(U256::from(18)), 18), "1");
    }

    #[test]
    fn bundled_lists_differ_by_network() {
        let main = bundled_tokens(EthNetwork::Mainnet);
        let sepolia = bundled_tokens(EthNetwork::Sepolia);
        assert!(main.iter().any(|t| t.symbol == "DAI"));
        assert!(!sepolia.iter().any(|t| t.symbol == "DAI"));
        assert!(sepolia.iter().any(|t| t.symbol == "USDC"));
        assert!(main.iter().any(|t| t.native));
    }

    #[test]
    fn prepared_send_summary_includes_symbol_and_fee() {
        let native = PreparedSend {
            from: "0x9858EfFD232B4033E47d90003D41EC34EcaEda94".into(),
            to: "0x9858Eff28F61CF0aDe1AC00482789d2EF5e6d47E".into(),
            token_symbol: "ETH".into(),
            token_address: None,
            amount: U256::from(10).pow(U256::from(16)),
            amount_display: "0.01".into(),
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 100_000_000,
            fee_wei: U256::from(21_000) * U256::from(1_000_000_000u64),
            fee_symbol: "ETH".into(),
            chain_id: 11155111,
            nonce: 0,
        };
        let text = native.summary();
        assert!(text.contains("0.01 ETH"));
        assert!(text.contains("chain 11155111"));
        let erc20 = PreparedSend {
            token_symbol: "USDC".into(),
            token_address: Some("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into()),
            amount_display: "2.5".into(),
            ..native
        };
        assert!(erc20.summary().contains("2.5 USDC"));
        assert!(erc20.summary().contains("paid in ETH"));
    }

    #[test]
    fn transfer_calldata_is_nonempty() {
        let to = Address::from_str("0x9858Eff28F61CF0aDe1AC00482789d2EF5e6d47E").unwrap();
        let data = encode_transfer(to, U256::from(1u64));
        assert_eq!(&data[..4], &SELECTOR_TRANSFER);
        assert_eq!(data.len(), 68);
        let balance_of = encode_balance_of(to);
        assert_eq!(&balance_of[..4], &SELECTOR_BALANCE_OF);
        assert_eq!(balance_of.len(), 36);
    }

    #[test]
    fn fee_tiers_map_labels() {
        let tiers = FeeTiers {
            low: 1,
            medium: 2,
            high: 3,
            priority: 4,
        };
        assert_eq!(fee_from_tier(&tiers, "low").0, 1);
        assert_eq!(fee_from_tier(&tiers, "High").0, 3);
        assert_eq!(fee_from_tier(&tiers, "medium").1, 4);
    }
}
