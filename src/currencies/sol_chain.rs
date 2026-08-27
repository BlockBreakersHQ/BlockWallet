use std::collections::BTreeMap;

use curve25519_dalek::edwards::CompressedEdwardsY;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::configuration::block_error;
use crate::currencies::sol::{sign_message, signing_key_from_base58};
use crate::currencies::tokens::Token;

pub const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const FEE_LAMPORTS_PER_SIGNATURE: u64 = 5_000;
const HISTORY_CAP: usize = 40;
const HISTORY_LOOKBACK: u32 = 20;

pub type Pubkey = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolNetwork {
    Mainnet,
    Devnet,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegistryToken {
    pub symbol: String,
    pub name: String,
    pub address: String,
    pub decimals: u8,
    pub native: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolHistoryItem {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub symbol: String,
    pub amount: String,
    pub incoming: bool,
    pub confirmations: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolSyncState {
    pub lamports: u64,
    pub receive_address: String,
    pub spl: BTreeMap<String, String>,
    pub history: Vec<SolHistoryItem>,
    pub offline: bool,
}

impl SolSyncState {
    pub fn balance_display(&self) -> String {
        if self.offline {
            return format!("{} SOL (offline)", format_units_trimmed(self.lamports, 9));
        }
        format!("{} SOL", format_units_trimmed(self.lamports, 9))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSend {
    pub from: String,
    pub to: String,
    pub token_symbol: String,
    pub token_mint: Option<String>,
    pub amount: u64,
    pub amount_display: String,
    pub fee_lamports: u64,
    pub create_destination_ata: bool,
}

impl PreparedSend {
    pub fn summary(&self) -> String {
        let fee = format_units_trimmed(self.fee_lamports, 9);
        if self.token_mint.is_none() {
            let total = self.amount.saturating_add(self.fee_lamports);
            format!(
                "To: {}\nAmount: {} SOL\nNetwork fee: {} SOL\nTotal (amount + fee): {} SOL",
                self.to,
                self.amount_display,
                fee,
                format_units_trimmed(total, 9)
            )
        } else {
            let mut summary = format!(
                "To: {}\nAmount: {} {}\nNetwork fee: {} SOL (paid in SOL)",
                self.to, self.amount_display, self.token_symbol, fee
            );
            if self.create_destination_ata {
                summary.push_str("\n(Recipient has no token account yet; this send creates one.)");
            }
            summary
        }
    }
}

pub fn parse_network(name: &str) -> SolNetwork {
    match name.trim().to_ascii_lowercase().as_str() {
        "devnet" | "testnet" | "test" => SolNetwork::Devnet,
        _ => SolNetwork::Mainnet,
    }
}

pub fn network_name(network: SolNetwork) -> &'static str {
    match network {
        SolNetwork::Devnet => "devnet",
        SolNetwork::Mainnet => "mainnet",
    }
}

pub fn default_rpc(network: SolNetwork) -> &'static str {
    match network {
        SolNetwork::Mainnet => "https://api.mainnet-beta.solana.com",
        SolNetwork::Devnet => "https://api.devnet.solana.com",
    }
}

pub fn resolve_rpc(sol_node: &str, network: SolNetwork) -> String {
    let node = sol_node.trim();
    if !node.is_empty() {
        return node.trim_end_matches('/').to_string();
    }
    default_rpc(network).to_string()
}

/// Every mint below was checked on-chain with `getAccountInfo` (jsonParsed) before being
/// bundled: the account must exist, be owned by the SPL token program, and report the
/// decimals claimed here. A wrong mint would have the wallet display and spend the wrong
/// asset, so no address goes in on the strength of a listing site alone.
fn spl(symbol: &str, name: &str, mint: &str, decimals: u8) -> RegistryToken {
    RegistryToken {
        symbol: symbol.to_string(),
        name: name.to_string(),
        address: mint.to_string(),
        decimals,
        native: false,
    }
}

pub fn bundled_tokens(network: SolNetwork) -> Vec<RegistryToken> {
    let mut tokens = vec![RegistryToken {
        symbol: "SOL".into(),
        name: "Solana".into(),
        address: SYSTEM_PROGRAM_ID.into(),
        decimals: 9,
        native: true,
    }];
    match network {
        // Every mint below was verified on-chain before it was bundled: the account exists,
        // is owned by the classic SPL token program, is of type "mint", and reports these
        // decimals. Ranked by holder count with a liquidity floor, which is a better proxy
        // for "a wallet user will actually hold this" than liquidity alone: ranking purely by
        // liquidity produced a list that was eighteen near-identical liquid-staking SOL
        // derivatives and omitted JUP, BONK, PYTH and WIF entirely.
        //
        // Classic SPL only. Token-2022 mints are excluded because `find_associated_token_address`
        // and the transfer instruction both use the classic program id, and Token-2022 uses its
        // own as an ATA seed: bundling one would show a balance that could not be spent.
        SolNetwork::Mainnet => tokens.extend([
            spl("ANTFUN", "AntFun", "CWZ6BsdnjkDVTGkmL6bGbJXXig6ceef12KvyGQW14cMt", 6),
            spl("BOME", "BOOK OF MEME", "ukHH6c7mMyiWCf1b9pnWe25TSpkDDt3H5pQZgZ74J82", 6),
            spl("Bonk", "Bonk", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", 5),
            spl("CHILLGUY", "Just a chill guy", "Df6yfrKC8kZE3KNkrHERKzAetSxbrWeniQfyJY4Jpump", 6),
            spl("DBR", "deBridge", "DBRiDgJAMsM95moTzJs7M9LnkGErpbv9v6CUR1DXnUu5", 6),
            spl("DOOD", "Doodles", "DvjbEsdca43oQcw2h3HW1CT7N3x5vRcr3QrvTUHnXvgV", 9),
            spl("ETH", "Ether (Portal)", "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs", 8),
            spl("Fartcoin", "Fartcoin", "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump", 6),
            spl("GIGA", "GIGACHAD", "63LfDmNb3MQ8mw9MtZ2To9bEA2M71kZUUGq5tiJxcqj9", 5),
            spl("GOAT", "Goatseus Maximus", "CzLSujWBLFsSjncfkh59rUFqvafWcY5tzedWJSuypump", 6),
            spl("GRASS", "Grass", "Grass7B4RdKfBCjTKgSqnXkqjwiGvQyFbuSCUJr3XXjs", 9),
            spl("HNT", "Helium Network Token", "hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux", 8),
            spl("JitoSOL", "Jito Staked SOL", "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", 9),
            spl("JLP", "Jupiter Perps", "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4", 6),
            spl("JTO", "JITO", "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL", 9),
            spl("JUP", "Jupiter", "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", 6),
            spl("ME", "Magic Eden", "MEFNBXixkEbait3xn9bkm8WsJzXtVsaJEn4c8Sam21u", 6),
            spl("MELANIA", "Melania Meme", "FUAfBo2jgks6gB4Z4LfZkqSZgzNucisEHqnNebaRxM1P", 6),
            spl("MEW", "cat in a dogs world", "MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5", 5),
            spl("MOODENG", "Moo Deng", "ED5nyyWEzpPPiWimP8vYm7sD7TD3LAt3Q3gRTWHzPJBY", 6),
            spl("mSOL", "Marinade staked SOL (mSOL)", "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", 9),
            spl("ORCA", "Orca", "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE", 6),
            spl("PENGU", "Pudgy Penguins", "2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv", 6),
            spl("Pnut", "Peanut the Squirrel", "2qEHjDLDLbuBgRYvsxhc5D6uDWAivNFZGan56P1tpump", 6),
            spl("POPCAT", "Popcat", "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr", 9),
            spl("PSOL", "Phantom Staked SOL", "pSo1f9nQXWgXibFtKf7NWYxb5enAM4qfP6UJSiXRQfL", 9),
            spl("PYTH", "Pyth Network", "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3", 6),
            spl("RAY", "Raydium", "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R", 6),
            spl("RENDER", "Render Token", "rndrizKT3MK1iimdxRdWabcF7Zg7AR5T4nud4EkHBof", 8),
            spl("SLIM", "Solanium", "xxxxa1sKNGwFtw2kFn8XauW9xq8hBZ5kVtcSesTT9fW", 6),
            spl("SOLCEX", "SolCex ", "AMjzRn1TBQwQfNAjHFeBb7uGbbqbJB7FzXAnGgdFPk6K", 6),
            spl("TRUMP", "OFFICIAL TRUMP", "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN", 6),
            spl("USA", "American Coin", "69kdRLyP5DTRkpHraaSZAQbWmAwzF9guKjZfzMXzcbAs", 6),
            spl("USDC", "USD Coin", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 6),
            spl("USDT", "USDT", "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", 6),
            spl("VINE", "Vine Coin", "6AJcP7wuLwmRYLBNbi825wgguaPsWzPBEHcHndpRpump", 6),
            spl("W", "Wormhole Token", "85VBFQZC9TZkfaptBWjvUw7YbZjy52A6mjtPGjstQAmQ", 6),
            spl("WEN", "Wen", "WENWENvqqNya429ubCdR81ZmD69brwQaaBYY6p3LCpk", 5),
            spl("WIF", "dogwifhat", "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm", 6),
            spl("wSOL", "Wrapped SOL", "So11111111111111111111111111111111111111112", 9),
        ]),
        SolNetwork::Devnet => tokens.extend([
            spl("USDC", "USD Coin (devnet)", "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU", 6),
            spl("wSOL", "Wrapped SOL", "So11111111111111111111111111111111111111112", 9),
        ]),
    }
    tokens
}

pub fn apply_bundled_tokens(tokens: &mut crate::currencies::tokens::Tokens, network: SolNetwork) {
    for item in bundled_tokens(network) {
        let logo = crate::configuration::paths::token_icon_path(&item.symbol);
        tokens.eth_tokens.insert(
            format!("sol:{}", item.symbol),
            Token {
                name: item.name,
                symbol: item.symbol,
                address: item.address,
                logo,
                decimals: item.decimals as i32,
                chain: "sol".to_string(),
            },
        );
    }
}

pub fn is_native_token(token: &Token) -> bool {
    token.chain == "sol"
        && (token.symbol.eq_ignore_ascii_case("SOL")
            || token.address.trim() == SYSTEM_PROGRAM_ID
            || token.address.trim().is_empty())
}

pub fn parse_derivation_indexes(path: &str) -> Result<Vec<u32>, block_error::Error> {
    let trimmed = path.trim();
    let rest = trimmed
        .strip_prefix("m/")
        .or_else(|| trimmed.strip_prefix("M/"))
        .ok_or_else(|| block_error::Error::new(format!("invalid derivation path: {trimmed}")))?;
    rest.split('/')
        .map(|segment| {
            let segment = segment.trim();
            let digits = segment
                .strip_suffix('\'')
                .or_else(|| segment.strip_suffix('h'))
                .unwrap_or(segment);
            digits
                .parse::<u32>()
                .map_err(|e| block_error::Error::new(format!("invalid path segment {segment:?}: {e}")))
        })
        .collect()
}

pub fn parse_pubkey(address: &str) -> Result<Pubkey, block_error::Error> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(block_error::Error::new("address is required".into()));
    }
    let bytes = bs58::decode(trimmed)
        .into_vec()
        .map_err(|e| block_error::Error::new(format!("invalid Solana address: {e}")))?;
    bytes
        .try_into()
        .map_err(|_| block_error::Error::new("invalid Solana address length".into()))
}

pub fn encode_pubkey(pubkey: &Pubkey) -> String {
    bs58::encode(pubkey).into_string()
}

pub fn validate_address(address: &str) -> Result<Pubkey, block_error::Error> {
    parse_pubkey(address)
}

pub fn parse_token_amount(input: &str, decimals: u8) -> Result<u64, block_error::Error> {
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
    combined
        .parse::<u64>()
        .map_err(|e| block_error::Error::new(format!("amount is too large: {e}")))
}

pub fn format_units_trimmed(amount: u64, decimals: u8) -> String {
    if amount == 0 {
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

fn compact_u16_encode(mut value: u16, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
            out.push(byte);
        } else {
            out.push(byte);
            break;
        }
    }
}

fn rpc_call(rpc: &str, method: &str, params: Value) -> Result<Value, block_error::Error> {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let text = crate::configuration::http::post_json(rpc, &body)?;
    let response: Value = serde_json::from_str(&text)
        .map_err(|e| block_error::Error::new(format!("invalid response from the solana rpc: {e}")))?;
    if let Some(error) = response.get("error") {
        return Err(block_error::Error::new(format!("solana rpc error: {error}")));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| block_error::Error::new("solana rpc response missing result".into()))
}

fn get_balance(rpc: &str, address: &str) -> Result<u64, block_error::Error> {
    let result = rpc_call(rpc, "getBalance", json!([address]))?;
    result
        .get("value")
        .and_then(Value::as_u64)
        .ok_or_else(|| block_error::Error::new("unexpected getBalance response".into()))
}

fn get_latest_blockhash(rpc: &str) -> Result<[u8; 32], block_error::Error> {
    let result = rpc_call(rpc, "getLatestBlockhash", json!([{"commitment": "finalized"}]))?;
    let blockhash = result
        .get("value")
        .and_then(|v| v.get("blockhash"))
        .and_then(Value::as_str)
        .ok_or_else(|| block_error::Error::new("unexpected getLatestBlockhash response".into()))?;
    let bytes = bs58::decode(blockhash)
        .into_vec()
        .map_err(|e| block_error::Error::new(format!("invalid blockhash: {e}")))?;
    bytes
        .try_into()
        .map_err(|_| block_error::Error::new("invalid blockhash length".into()))
}

fn get_token_account_balance(rpc: &str, ata: &str) -> Option<u64> {
    let result = rpc_call(rpc, "getTokenAccountBalance", json!([ata])).ok()?;
    result
        .get("value")?
        .get("amount")?
        .as_str()?
        .parse::<u64>()
        .ok()
}

/// Every SPL balance the account holds, in one call.
///
/// `getTokenAccountsByOwner` returns all of the owner's token accounts at once, so the cost
/// of syncing does not grow with the bundled token list. The per-mint
/// `getTokenAccountBalance` this replaces meant one RPC call per listed token on every sync
/// cycle, which puts a wallet on a free public RPC into rate limiting as soon as the list is
/// more than a handful of entries long.
///
/// Keyed by mint address. A token the account has never held simply has no entry, which is
/// the same outcome the per-mint version reached by failing its call.
fn get_all_token_balances(rpc: &str, owner: &str) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    let params = json!([
        owner,
        {"programId": TOKEN_PROGRAM_ID},
        {"encoding": "jsonParsed"}
    ]);
    let Ok(result) = rpc_call(rpc, "getTokenAccountsByOwner", params) else {
        return out;
    };
    let Some(entries) = result.get("value").and_then(Value::as_array) else {
        return out;
    };
    for entry in entries {
        let Some(info) = entry
            .get("account")
            .and_then(|a| a.get("data"))
            .and_then(|d| d.get("parsed"))
            .and_then(|p| p.get("info"))
        else {
            continue;
        };
        let Some(mint) = info.get("mint").and_then(Value::as_str) else {
            continue;
        };
        let Some(amount) = info
            .get("tokenAmount")
            .and_then(|t| t.get("amount"))
            .and_then(Value::as_str)
            .and_then(|a| a.parse::<u64>().ok())
        else {
            continue;
        };
        // An owner can hold several accounts for one mint. The wallet spends from the
        // associated account, but the balance shown should be everything it holds, so they
        // are summed rather than the last one winning.
        *out.entry(mint.to_string()).or_insert(0) += amount;
    }
    out
}

/// SHA-256 loop over bump seeds 255..=0 until an off-curve (no known private key) address is found,
/// per Solana's `find_program_address` PDA derivation.
pub fn find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Result<(Pubkey, u8), block_error::Error> {
    for bump in (0u8..=255).rev() {
        let mut hasher = Sha256::new();
        for seed in seeds {
            hasher.update(seed);
        }
        hasher.update([bump]);
        hasher.update(program_id);
        hasher.update(b"ProgramDerivedAddress");
        let hash: [u8; 32] = hasher.finalize().into();
        if CompressedEdwardsY(hash).decompress().is_none() {
            return Ok((hash, bump));
        }
    }
    Err(block_error::Error::new("unable to find a valid program address".into()))
}

pub fn find_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Result<(Pubkey, u8), block_error::Error> {
    let token_program = parse_pubkey(TOKEN_PROGRAM_ID)?;
    let ata_program = parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?;
    find_program_address(&[wallet, &token_program, mint], &ata_program)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct AccountMeta {
    pubkey: Pubkey,
    is_signer: bool,
    is_writable: bool,
}

struct Instruction {
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
}

fn system_transfer_instruction(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&lamports.to_le_bytes());
    Instruction {
        program_id: parse_pubkey(SYSTEM_PROGRAM_ID).expect("well-known system program id parses"),
        accounts: vec![
            AccountMeta { pubkey: *from, is_signer: true, is_writable: true },
            AccountMeta { pubkey: *to, is_signer: false, is_writable: true },
        ],
        data,
    }
}

fn spl_transfer_instruction(
    source: &Pubkey,
    destination: &Pubkey,
    owner: &Pubkey,
    token_program: &Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(9);
    data.push(3u8);
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: *token_program,
        accounts: vec![
            AccountMeta { pubkey: *source, is_signer: false, is_writable: true },
            AccountMeta { pubkey: *destination, is_signer: false, is_writable: true },
            AccountMeta { pubkey: *owner, is_signer: true, is_writable: false },
        ],
        data,
    }
}

fn create_associated_token_account_idempotent_instruction(
    funding: &Pubkey,
    wallet: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Result<Instruction, block_error::Error> {
    let (ata, _bump) = find_associated_token_address(wallet, mint)?;
    let ata_program = parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?;
    let system_program = parse_pubkey(SYSTEM_PROGRAM_ID)?;
    Ok(Instruction {
        program_id: ata_program,
        accounts: vec![
            AccountMeta { pubkey: *funding, is_signer: true, is_writable: true },
            AccountMeta { pubkey: ata, is_signer: false, is_writable: true },
            AccountMeta { pubkey: *wallet, is_signer: false, is_writable: false },
            AccountMeta { pubkey: *mint, is_signer: false, is_writable: false },
            AccountMeta { pubkey: system_program, is_signer: false, is_writable: false },
            AccountMeta { pubkey: *token_program, is_signer: false, is_writable: false },
        ],
        data: vec![1u8],
    })
}

/// Compiles a legacy Solana `Message`: merges each instruction's account metas (deduped by
/// pubkey, signer/writable flags OR'd together), orders them fee-payer-first then
/// signer+writable, signer+readonly, writable, readonly, and serializes the compact-array wire
/// format. Returns the message bytes plus the final account key ordering.
fn compile_message(fee_payer: Pubkey, instructions: &[Instruction], recent_blockhash: [u8; 32]) -> (Vec<u8>, Vec<Pubkey>) {
    let mut metas: Vec<AccountMeta> = vec![AccountMeta { pubkey: fee_payer, is_signer: true, is_writable: true }];
    for ix in instructions {
        for meta in &ix.accounts {
            if let Some(existing) = metas.iter_mut().find(|m| m.pubkey == meta.pubkey) {
                existing.is_signer |= meta.is_signer;
                existing.is_writable |= meta.is_writable;
            } else {
                metas.push(*meta);
            }
        }
        if !metas.iter().any(|m| m.pubkey == ix.program_id) {
            metas.push(AccountMeta { pubkey: ix.program_id, is_signer: false, is_writable: false });
        }
    }

    let (fee_payer_meta, rest) = metas.split_first().expect("fee payer meta always present");
    let mut signer_writable = Vec::new();
    let mut signer_readonly = Vec::new();
    let mut writable = Vec::new();
    let mut readonly = Vec::new();
    for m in rest {
        match (m.is_signer, m.is_writable) {
            (true, true) => signer_writable.push(*m),
            (true, false) => signer_readonly.push(*m),
            (false, true) => writable.push(*m),
            (false, false) => readonly.push(*m),
        }
    }
    let mut ordered = vec![*fee_payer_meta];
    ordered.extend(signer_writable);
    ordered.extend(signer_readonly);
    ordered.extend(writable);
    ordered.extend(readonly);

    let num_required_signatures = ordered.iter().filter(|m| m.is_signer).count() as u8;
    let num_readonly_signed = ordered.iter().filter(|m| m.is_signer && !m.is_writable).count() as u8;
    let num_readonly_unsigned = ordered.iter().filter(|m| !m.is_signer && !m.is_writable).count() as u8;
    let account_keys: Vec<Pubkey> = ordered.iter().map(|m| m.pubkey).collect();

    let mut out = Vec::new();
    out.push(num_required_signatures);
    out.push(num_readonly_signed);
    out.push(num_readonly_unsigned);
    compact_u16_encode(account_keys.len() as u16, &mut out);
    for key in &account_keys {
        out.extend_from_slice(key);
    }
    out.extend_from_slice(&recent_blockhash);
    compact_u16_encode(instructions.len() as u16, &mut out);
    for ix in instructions {
        let program_index = account_keys.iter().position(|k| *k == ix.program_id).expect("program id is an account key") as u8;
        out.push(program_index);
        compact_u16_encode(ix.accounts.len() as u16, &mut out);
        for a in &ix.accounts {
            let idx = account_keys.iter().position(|k| *k == a.pubkey).expect("instruction account is a message account key") as u8;
            out.push(idx);
        }
        compact_u16_encode(ix.data.len() as u16, &mut out);
        out.extend_from_slice(&ix.data);
    }
    (out, account_keys)
}

fn finalize_transaction(
    signing_key: &SigningKey,
    fee_payer: Pubkey,
    instructions: Vec<Instruction>,
    recent_blockhash: [u8; 32],
) -> Result<Vec<u8>, block_error::Error> {
    let (message, account_keys) = compile_message(fee_payer, &instructions, recent_blockhash);
    let required_signers = message[0] as usize;
    if required_signers != 1 || account_keys.first() != Some(&fee_payer) {
        // Every flow this wallet builds (native transfer, SPL transfer, SPL transfer + ATA
        // creation) uses the fee payer as the only signer; anything else means a bug upstream
        // rather than a transaction we know how to sign.
        return Err(block_error::Error::new(
            "unsupported transaction shape: expected exactly one signer (the sender)".into(),
        ));
    }
    let signature = sign_message(signing_key, &message);
    let mut out = Vec::new();
    compact_u16_encode(1, &mut out);
    out.extend_from_slice(&signature);
    out.extend_from_slice(&message);
    Ok(out)
}

pub fn sync_account(address: &str, sol_node: &str, network_name: &str, tokens: &[Token]) -> Result<SolSyncState, block_error::Error> {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(sol_node, network);
    // Validated up front so a malformed address fails loudly rather than reading as an
    // account with no balance and no history.
    validate_address(address)?;

    let lamports = match get_balance(&rpc, address) {
        Ok(value) => value,
        Err(_) => {
            return Ok(SolSyncState {
                lamports: 0,
                receive_address: address.to_string(),
                spl: BTreeMap::new(),
                history: Vec::new(),
                offline: true,
            });
        }
    };

    let held = get_all_token_balances(&rpc, address);
    let mut spl = BTreeMap::new();
    for token in tokens {
        if is_native_token(token) {
            continue;
        }
        if let Some(amount) = held.get(token.address.trim()) {
            spl.insert(
                token.symbol.clone(),
                format_units_trimmed(*amount, token.decimals.max(0) as u8),
            );
        }
    }

    let history = fetch_history(&rpc, address, tokens).unwrap_or_default();

    Ok(SolSyncState { lamports, receive_address: address.to_string(), spl, history, offline: false })
}

fn fetch_history(rpc: &str, address: &str, tokens: &[Token]) -> Result<Vec<SolHistoryItem>, block_error::Error> {
    let result = rpc_call(rpc, "getSignaturesForAddress", json!([address, {"limit": HISTORY_LOOKBACK}]))?;
    let entries = result.as_array().cloned().unwrap_or_default();
    let mut items = Vec::new();
    for entry in entries {
        let failed = entry.get("err").map(|e| !e.is_null()).unwrap_or(false);
        if failed {
            continue;
        }
        let Some(signature) = entry.get("signature").and_then(Value::as_str) else { continue };
        let confirmations = match entry.get("confirmationStatus").and_then(Value::as_str) {
            Some("finalized") => 32,
            Some("confirmed") => 1,
            _ => 0,
        };
        if let Ok(mut parsed) = fetch_transaction_deltas(rpc, signature, address, tokens) {
            for item in &mut parsed {
                item.confirmations = confirmations;
            }
            items.extend(parsed);
        }
        if items.len() >= HISTORY_CAP {
            break;
        }
    }
    items.truncate(HISTORY_CAP);
    Ok(items)
}

fn fetch_transaction_deltas(
    rpc: &str,
    signature: &str,
    address: &str,
    tokens: &[Token],
) -> Result<Vec<SolHistoryItem>, block_error::Error> {
    let result = rpc_call(
        rpc,
        "getTransaction",
        json!([signature, {"encoding": "json", "maxSupportedTransactionVersion": 0}]),
    )?;
    let mut items = Vec::new();

    let account_keys: Vec<String> = result
        .get("transaction")
        .and_then(|t| t.get("message"))
        .and_then(|m| m.get("accountKeys"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    if let Some(index) = account_keys.iter().position(|k| k == address) {
        let pre = result.get("meta").and_then(|m| m.get("preBalances")).and_then(Value::as_array);
        let post = result.get("meta").and_then(|m| m.get("postBalances")).and_then(Value::as_array);
        if let (Some(pre), Some(post)) = (pre, post) {
            let before = pre.get(index).and_then(Value::as_u64).unwrap_or(0) as i64;
            let after = post.get(index).and_then(Value::as_u64).unwrap_or(0) as i64;
            let delta = after - before;
            if delta != 0 {
                let counterparty = account_keys.iter().find(|k| k.as_str() != address).cloned().unwrap_or_default();
                items.push(SolHistoryItem {
                    txid: signature.to_string(),
                    from: if delta < 0 { address.to_string() } else { counterparty.clone() },
                    to: if delta < 0 { counterparty } else { address.to_string() },
                    symbol: "SOL".to_string(),
                    amount: format_units_trimmed(delta.unsigned_abs(), 9),
                    incoming: delta > 0,
                    confirmations: 0,
                });
            }
        }
    }

    let pre_tokens = result
        .get("meta")
        .and_then(|m| m.get("preTokenBalances"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let post_tokens = result
        .get("meta")
        .and_then(|m| m.get("postTokenBalances"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for token in tokens {
        if is_native_token(token) {
            continue;
        }
        let (Some(before), Some(after)) = (
            token_ui_amount(&pre_tokens, &token.address, address),
            token_ui_amount(&post_tokens, &token.address, address),
        ) else {
            continue;
        };
        let delta = after as i128 - before as i128;
        if delta != 0 {
            items.push(SolHistoryItem {
                txid: signature.to_string(),
                from: if delta < 0 { address.to_string() } else { String::new() },
                to: if delta < 0 { String::new() } else { address.to_string() },
                symbol: token.symbol.clone(),
                amount: format_units_trimmed(delta.unsigned_abs() as u64, token.decimals.max(0) as u8),
                incoming: delta > 0,
                confirmations: 0,
            });
        }
    }

    Ok(items)
}

fn token_ui_amount(balances: &[Value], mint: &str, owner: &str) -> Option<u64> {
    balances
        .iter()
        .find(|b| {
            b.get("mint").and_then(Value::as_str) == Some(mint)
                && b.get("owner").and_then(Value::as_str) == Some(owner)
        })
        .and_then(|b| b.get("uiTokenAmount"))
        .and_then(|u| u.get("amount"))
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u64>().ok())
}

pub fn prepare_send(
    from: &str,
    to: &str,
    amount_text: &str,
    token: &Token,
    sol_node: &str,
    network_name: &str,
) -> Result<PreparedSend, block_error::Error> {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(sol_node, network);
    let from_pk = validate_address(from)?;
    let to_pk = validate_address(to)?;
    let native = is_native_token(token);
    let decimals = if native { 9 } else { token.decimals.max(0) as u8 };
    let amount = parse_token_amount(amount_text, decimals)?;
    if amount == 0 {
        return Err(block_error::Error::new("amount must be greater than 0".into()));
    }

    let lamports = get_balance(&rpc, from).map_err(|_| block_error::Error::new("solana node is unreachable".into()))?;
    let fee_lamports = FEE_LAMPORTS_PER_SIGNATURE;
    if lamports < fee_lamports {
        return Err(block_error::Error::new("not enough SOL to cover the network fee".into()));
    }

    if native {
        if lamports < amount.saturating_add(fee_lamports) {
            return Err(block_error::Error::new("not enough SOL to send".into()));
        }
        return Ok(PreparedSend {
            from: from.to_string(),
            to: to.to_string(),
            token_symbol: token.symbol.clone(),
            token_mint: None,
            amount,
            amount_display: format_units_trimmed(amount, 9),
            fee_lamports,
            create_destination_ata: false,
        });
    }

    let mint = validate_address(&token.address)?;
    let (source_ata, _) = find_associated_token_address(&from_pk, &mint)?;
    let source_amount = get_token_account_balance(&rpc, &encode_pubkey(&source_ata))
        .ok_or_else(|| block_error::Error::new(format!("no {} balance to send", token.symbol)))?;
    if source_amount < amount {
        return Err(block_error::Error::new(format!("not enough {} to send", token.symbol)));
    }
    let (dest_ata, _) = find_associated_token_address(&to_pk, &mint)?;
    let create_destination_ata = get_token_account_balance(&rpc, &encode_pubkey(&dest_ata)).is_none();

    Ok(PreparedSend {
        from: from.to_string(),
        to: to.to_string(),
        token_symbol: token.symbol.clone(),
        token_mint: Some(token.address.clone()),
        amount,
        amount_display: format_units_trimmed(amount, decimals),
        fee_lamports,
        create_destination_ata,
    })
}

pub fn sign_and_broadcast(
    private_key: &str,
    plan: &PreparedSend,
    sol_node: &str,
    network_name: &str,
) -> Result<String, block_error::Error> {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(sol_node, network);
    let signing_key = signing_key_from_base58(private_key)?;
    let from_pk = validate_address(&plan.from)?;
    // The UI re-reads the account dropdown at confirm time, so the key handed in here is not
    // guaranteed to be the one the plan was built for. A mismatch would produce a signature
    // that does not verify against the declared fee payer, which the cluster would reject —
    // catching it here turns a confusing broadcast failure into a precise message, and stops
    // a stale plan from ever being paired with the wrong account.
    if signing_key.verifying_key().to_bytes() != from_pk {
        return Err(block_error::Error::new(
            "this key does not belong to the account the transaction was reviewed for".to_string(),
        ));
    }
    let to_pk = validate_address(&plan.to)?;
    let recent_blockhash = get_latest_blockhash(&rpc)?;

    let instructions = if let Some(mint_str) = &plan.token_mint {
        let mint = validate_address(mint_str)?;
        let token_program = parse_pubkey(TOKEN_PROGRAM_ID)?;
        let (source_ata, _) = find_associated_token_address(&from_pk, &mint)?;
        let (dest_ata, _) = find_associated_token_address(&to_pk, &mint)?;
        let mut ixs = Vec::new();
        if plan.create_destination_ata {
            ixs.push(create_associated_token_account_idempotent_instruction(
                &from_pk, &to_pk, &mint, &token_program,
            )?);
        }
        ixs.push(spl_transfer_instruction(&source_ata, &dest_ata, &from_pk, &token_program, plan.amount));
        ixs
    } else {
        vec![system_transfer_instruction(&from_pk, &to_pk, plan.amount)]
    };

    let tx_bytes = finalize_transaction(&signing_key, from_pk, instructions, recent_blockhash)?;
    let encoded = bs58::encode(tx_bytes).into_string();
    let result = rpc_call(&rpc, "sendTransaction", json!([encoded]))?;
    result
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| block_error::Error::new("unexpected sendTransaction response".into()))
}

pub fn fetch_token_metadata(mint: &str, sol_node: &str, network_name: &str) -> Result<RegistryToken, block_error::Error> {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(sol_node, network);
    validate_address(mint)?;
    let result = rpc_call(&rpc, "getAccountInfo", json!([mint, {"encoding": "jsonParsed"}]))?;
    let info = result
        .get("value")
        .and_then(|v| v.get("data"))
        .and_then(|d| d.get("parsed"))
        .and_then(|p| p.get("info"))
        .ok_or_else(|| block_error::Error::new("could not read that mint account".into()))?;
    let decimals = info
        .get("decimals")
        .and_then(Value::as_u64)
        .ok_or_else(|| block_error::Error::new("mint account has no decimals field".into()))? as u8;
    let label_len = mint.len().min(6);
    Ok(RegistryToken {
        symbol: mint[..label_len].to_uppercase(),
        name: format!("SPL token {}", &mint[..mint.len().min(10)]),
        address: mint.to_string(),
        decimals,
        native: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same clerical guard as the ERC-20 list: a duplicate symbol silently overwrites its twin
    /// in the registry, and a malformed mint would be dropped at sync time without saying so.
    /// Whether each mint is the *right* one was established separately, by reading the mint
    /// account and checking it is owned by the token program with the decimals claimed here.
    #[test]
    fn every_bundled_mint_is_internally_consistent() {
        for network in [SolNetwork::Mainnet, SolNetwork::Devnet] {
            let tokens = bundled_tokens(network);
            let name = network_name(network);
            assert_eq!(
                tokens.iter().filter(|t| t.native).count(),
                1,
                "{name} must bundle exactly one native asset"
            );

            let mut symbols: Vec<&str> = tokens.iter().map(|t| t.symbol.as_str()).collect();
            symbols.sort_unstable();
            let before = symbols.len();
            symbols.dedup();
            assert_eq!(before, symbols.len(), "{name} bundles a duplicate symbol");

            let mut mints: Vec<&str> = Vec::new();
            for token in &tokens {
                if token.native {
                    continue;
                }
                assert!(
                    validate_address(&token.address).is_ok(),
                    "{name} bundles {} with an invalid mint {}",
                    token.symbol,
                    token.address
                );
                assert!(
                    token.decimals <= 18,
                    "{name} bundles {} with implausible decimals {}",
                    token.symbol,
                    token.decimals
                );
                assert!(
                    !mints.contains(&token.address.as_str()),
                    "{name} bundles two symbols at mint {}",
                    token.address
                );
                mints.push(&token.address);
            }
        }
    }

    #[test]
    fn parses_networks_and_default_rpcs() {

        assert_eq!(parse_network("devnet"), SolNetwork::Devnet);
        assert_eq!(parse_network(""), SolNetwork::Mainnet);
        assert!(resolve_rpc("", SolNetwork::Mainnet).contains("mainnet"));
        assert!(resolve_rpc("", SolNetwork::Devnet).contains("devnet"));
        assert_eq!(resolve_rpc("https://my.node/", SolNetwork::Mainnet), "https://my.node");
    }

    #[test]
    fn validates_addresses() {
        // A well-formed 32-byte base58 pubkey (the system program id).
        assert!(validate_address(SYSTEM_PROGRAM_ID).is_ok());
        assert!(validate_address("not-base58!!!").is_err());
        assert!(validate_address("").is_err());
    }

    #[test]
    fn parses_and_formats_token_amounts() {
        assert_eq!(parse_token_amount("1", 9).unwrap(), 1_000_000_000u64);
        assert_eq!(parse_token_amount("1.5", 6).unwrap(), 1_500_000u64);
        assert_eq!(parse_token_amount(".5", 2).unwrap(), 50u64);
        assert!(parse_token_amount("1.2345678", 6).is_err());
        assert!(parse_token_amount("", 9).is_err());
        assert_eq!(format_units_trimmed(1_500_000, 6), "1.5");
        assert_eq!(format_units_trimmed(1_000_000_000, 9), "1");
    }

    #[test]
    fn bundled_lists_differ_by_network() {
        let main = bundled_tokens(SolNetwork::Mainnet);
        let dev = bundled_tokens(SolNetwork::Devnet);
        assert!(main.iter().any(|t| t.native && t.symbol == "SOL"));
        assert_ne!(
            main.iter().find(|t| t.symbol == "USDC").unwrap().address,
            dev.iter().find(|t| t.symbol == "USDC").unwrap().address
        );
    }

    #[test]
    fn compact_u16_matches_known_encodings() {
        let mut out = Vec::new();
        compact_u16_encode(0, &mut out);
        assert_eq!(out, vec![0]);
        out.clear();
        compact_u16_encode(127, &mut out);
        assert_eq!(out, vec![127]);
        out.clear();
        compact_u16_encode(128, &mut out);
        assert_eq!(out, vec![0x80, 0x01]);
        out.clear();
        compact_u16_encode(300, &mut out);
        assert_eq!(out, vec![0xAC, 0x02]);
    }

    #[test]
    fn pda_derivation_is_deterministic_and_mint_sensitive() {
        let wallet = parse_pubkey(SYSTEM_PROGRAM_ID).unwrap();
        let mint_a = parse_pubkey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let mint_b = parse_pubkey("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
        let (ata_a1, bump_a1) = find_associated_token_address(&wallet, &mint_a).unwrap();
        let (ata_a2, bump_a2) = find_associated_token_address(&wallet, &mint_a).unwrap();
        let (ata_b, _) = find_associated_token_address(&wallet, &mint_b).unwrap();
        assert_eq!(ata_a1, ata_a2);
        assert_eq!(bump_a1, bump_a2);
        assert_ne!(ata_a1, ata_b);
        // A valid PDA must be off-curve.
        assert!(CompressedEdwardsY(ata_a1).decompress().is_none());
    }

    #[test]
    fn system_transfer_instruction_layout() {
        let from = parse_pubkey(SYSTEM_PROGRAM_ID).unwrap();
        let to = parse_pubkey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let ix = system_transfer_instruction(&from, &to, 42);
        assert_eq!(&ix.data[..4], &2u32.to_le_bytes());
        assert_eq!(&ix.data[4..], &42u64.to_le_bytes());
        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.accounts[0].is_signer && ix.accounts[0].is_writable);
        assert!(!ix.accounts[1].is_signer && ix.accounts[1].is_writable);
    }

    #[test]
    fn spl_transfer_instruction_layout() {
        let source = parse_pubkey(SYSTEM_PROGRAM_ID).unwrap();
        let dest = parse_pubkey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let owner = parse_pubkey("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
        let token_program = parse_pubkey(TOKEN_PROGRAM_ID).unwrap();
        let ix = spl_transfer_instruction(&source, &dest, &owner, &token_program, 7);
        assert_eq!(ix.data[0], 3u8);
        assert_eq!(&ix.data[1..], &7u64.to_le_bytes());
        assert_eq!(ix.program_id, token_program);
        assert!(ix.accounts[2].is_signer && !ix.accounts[2].is_writable);
    }

    #[test]
    fn compiled_message_has_single_required_signer_for_native_transfer() {
        // `from` must differ from the system program id itself (SYSTEM_PROGRAM_ID), or the fee
        // payer account and the instruction's program-id account collide into one entry.
        let from = parse_pubkey(TOKEN_PROGRAM_ID).unwrap();
        let to = parse_pubkey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let ix = system_transfer_instruction(&from, &to, 1);
        let (message, keys) = compile_message(from, &[ix], [0u8; 32]);
        assert_eq!(message[0], 1); // num_required_signatures
        assert_eq!(keys[0], from); // fee payer stays first
        assert_eq!(keys.len(), 3); // from, to, system program
    }

    #[test]
    fn prepared_send_summary_includes_symbol_and_fee() {
        let native = PreparedSend {
            from: SYSTEM_PROGRAM_ID.to_string(),
            to: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            token_symbol: "SOL".into(),
            token_mint: None,
            amount: 10_000_000,
            amount_display: "0.01".into(),
            fee_lamports: 5_000,
            create_destination_ata: false,
        };
        assert!(native.summary().contains("0.01 SOL"));
        let spl = PreparedSend {
            token_symbol: "USDC".into(),
            token_mint: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into()),
            amount_display: "2.5".into(),
            create_destination_ata: true,
            ..native
        };
        let text = spl.summary();
        assert!(text.contains("2.5 USDC"));
        assert!(text.contains("creates one"));
    }
}

/// Broadcast an already-signed transaction.
///
/// Used by the swap path, where the transaction was built by an aggregator and signed by
/// `swap::solana_tx` rather than assembled here. Kept separate from `sign_and_broadcast` so
/// that function keeps its guarantee of only ever sending transactions this wallet built.
pub fn broadcast_signed(
    signed_tx: &[u8],
    sol_node: &str,
    network_name: &str,
) -> Result<String, block_error::Error> {
    let network = parse_network(network_name);
    let rpc = resolve_rpc(sol_node, network);
    let encoded = bs58::encode(signed_tx).into_string();
    let result = rpc_call(&rpc, "sendTransaction", json!([encoded]))?;
    result
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| block_error::Error::new("unexpected sendTransaction response".into()))
}
