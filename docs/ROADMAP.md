# Roadmap

Self-custody GTK4/libadwaita wallet for the Librem 5. MVP is **BTC + ETH L1 send/receive/lock**, packaged, then QA on the phone. Solana (native + SPL, Phase 6), EVM L2s/sidechains (Phase 7), and Litecoin (Phase 8) all landed ahead of the original post-MVP schedule. Phase 9 rebuilt the UI on libadwaita's own patterns.

## Run

**Linux / PureOS** (from the repo root, after GTK 4 and libadwaita dev packages):

```sh
cargo run
```

Release binary:

```sh
cargo run --release
```

**Windows** (MSYS2 mingw64 + GNU rustc). Plain `cargo run` uses MSVC and fails looking for pkg-config:

```powershell
.\scripts\run-windows.ps1
```

If the binary is already built: `target/debug/block_wallet` (or `block_wallet.exe`).

Override wallet files with `BLOCKWALLET_HOME`. Flatpak: `flatpak run org.BlockBreakers.Wallet`.

---

## MVP phases

### Phase 0 — Project — **done**

- gtk4 0.10 / libadwaita 0.8; `ethers` replaced with Alloy; BTC path is **BDK 1.x** (`bdk_wallet` 1.2), not a bitcoin 0.29 bump
- Tests moved from `src/Tests` to `#[cfg(test)]` and `tests/`
- XDG paths (`directories`), `tracing` logs without secrets, hardcoded `/Users/andy/...` removed
- README, GPL-3.0-or-later, `.desktop`, AppStream
- First-launch 1inch download / 3-second sleep removed
- CSS class selectors fixed (`.label-error`, etc.)

### Phase 1 — Wallet core and security — **done**

- Encrypted store schema v1: Argon2id + ChaCha20-Poly1305 JSON (`Config.dic`)
- One BIP39 mnemonic → BTC BIP84 `bc1q`/`tb1q` + ETH BIP44 `m/44'/60'/0'/0/0`
- Onboarding: create 12/24, confirm phrase, restore, set password
- Unlock decrypts; lock wipes keys in memory and increments sync epoch
- Wallets tab hides mnemonic/private key unless reveal + password
- Clipboard timeout for secrets (later also addresses)

### Phase 2 — Bitcoin — **done**

- BDK in-memory wallet; Esplora default (`blockstream.info`) or Electrum (`ssl://` / `tcp://`)
- Sync, confirmed + pending balance, receive address + QR (works offline)
- PSBT prepare/sign/broadcast, fee tiers, bech32 validation
- History from the wallet, not `blockchain.info/getreceivedbyaddress`
- Review → confirm send UI; settings persist `btc_network` and `btc_node`

Desktop GTK send path is implemented. **Librem 5 testnet send** is still the device QA item in Phase 5.

### Phase 3 — Ethereum — **done**

- Alloy HTTP provider against user RPC (Infura optional; public default if empty)
- Native ETH transfer signed locally; ERC-20 `balanceOf` / `transfer` (not Etherscan `tokentx`)
- Bundled list: ETH, USDC, USDT, DAI, WBTC; Sepolia USDC; add-by-contract
- History via `eth_getLogs` Transfer events; optional Etherscan v2 if a key is set
- Wallet address is never overwritten
- Review → confirm send with fee and token symbol; `eth_network` + RPC persist

### Phase 4 — UI and settings — **done** (layout sized at 360×720; not yet tapped on the L5)

- Named list → detail → send stacks (no widget-tree walking)
- Home/Assets from node balances, not 1inch quotes
- Optional CoinGecko fiat (off by default)
- Trade tab replaced with **Activity**
- Settings persist: auto-lock, units, fiat, nodes
- Offline / node-unreachable banners; receive still works
- Touch-sized rows and tab bar; content clamp for a docked window

### Phase 5 — Package and harden — **done** (installable in tree; **not** tagged until phone QA)

- Flatpak manifest: network for RPC, no `--filesystem=home`
- Debian/PureOS sketch under `packaging/debian/`
- Panic/unwrap audit on wallet startup, QR, extra-account generation
- Mainnet send requires an extra checkbox; testnet/Sepolia does not
- Clipboard timeout on addresses and keys
- Settings: **Use test networks** (BTC testnet + ETH Sepolia)
- Checklist: [LIBREM5.md](LIBREM5.md)
- Images resolved from `/app/share/blockwallet` and `/usr/share/blockwallet` when installed
- Crate version **0.1.0**; tag after the phone checklist

**MVP leftover (hard gate):** run [LIBREM5.md](LIBREM5.md) on the real Librem 5, then `git tag v0.1.0`.

### Phase 6 — Solana (native + SPL) — **done** (desktop; not yet tapped on the L5)

- One BIP39 mnemonic also derives Solana via SLIP-0010 ed25519, hardened-only `m/44'/501'/0'/0'`
- Key material: `ed25519-dalek` + `slip10_ed25519`, addresses/keys in base58 (`bs58`) — no `solana-sdk` dependency
- Raw JSON-RPC over `reqwest` (`getBalance`, `getLatestBlockhash`, `getTokenAccountBalance`, `sendTransaction`, `getSignaturesForAddress`, `getTransaction`) — same hand-rolled-protocol style as `eth_chain.rs`'s ERC-20 ABI encoding
- Hand-built legacy `Message`/`Transaction` wire format (compact-u16 arrays, fee-payer-first account ordering) signed with a single signer
- Native SOL send/receive/lock, balance sync, and best-effort Activity history (native transfers + tracked SPL transfers, derived from `preBalances`/`postBalances` and `preTokenBalances`/`postTokenBalances` deltas)
- SPL Token Program transfers, with idempotent Associated Token Account creation (hand-rolled PDA derivation) when the recipient has none yet
- Bundled: native SOL + mainnet USDC-SPL; add any other SPL token by mint address
- Settings: SOL RPC node, mainnet/devnet toggle (folded into the existing **Use test networks** switch)
- Mainnet send requires the same "I understand this spends real…" checkbox as BTC/ETH; devnet does not
- Extra accounts increment the hardened account index (`m/44'/501'/n'/0'`), matching Phantom/Solflare
- Token registry keys became chain-namespaced (`"eth:USDC"`, `"sol:USDC"`, …) so same-symbol tokens on different chains (e.g. USDC as ERC-20 vs SPL) don't collide in the token map — `Token.chain` is now the dispatch key used across Views instead of bare symbol matching
- Not yet added: fiat pricing for SPL tokens beyond native SOL, an SPL history parser for tokens outside the configured registry, a bundled SOL icon asset
- Librem 5 device QA (create/restore, receive, devnet send of SOL and the bundled SPL token) is a new checklist section in [LIBREM5.md](LIBREM5.md), same gate as BTC/ETH

### Phase 7 — EVM L2s/sidechains — **done** (desktop; not yet tapped on the L5)

- Same ETH key/address (`m/44'/60'/0'/0/0`), same Alloy provider code — added as more `eth_network` values: Arbitrum One, Base, Optimism, Polygon PoS, BNB Smart Chain, Avalanche C-Chain (chain IDs and RPCs verified against each network's own docs)
- Native gas token generalized: `eth_chain::native_symbol(network)` (ETH / MATIC / BNB / AVAX) now drives balance, fee, and history display instead of a hardcoded `"ETH"`
- Bundled stablecoin per network (Circle-issued native USDC on Arbitrum/Base/Optimism/Polygon/Avalanche; Binance-Peg USDT, 18 decimals, on BSC — Circle doesn't issue native USDC there)
- Switching networks now clears the previous network's bundled tokens and each wallet's ERC-20 balance cache, so a stale contract address/amount from the prior network can't linger under a reused symbol key
- Safety-check fix: the mainnet spend-confirmation checkbox was gated on `network == "mainnet"`, which hid it (and left Confirm enabled) for *any* other network name, including a new L2 — now gated on `eth_chain::is_testnet`, correct for every real-value network
- Settings: network dropdown extended to all eight networks; "Use test networks" still only touches BTC testnet / ETH Sepolia / SOL devnet — L2s are a separate, explicit choice since they're real-value networks, not test networks
- Not yet added: bridging UI (deliberately out of scope, per the original P2 note), per-L2 fiat pricing beyond ETH/BTC/SOL, Etherscan-v2-history verified end-to-end for each new chain (URL is chain-id-parameterized and should work, not independently confirmed per chain)

### Phase 8 — Litecoin — **done** (desktop; not yet tapped on the L5)

- Hand-rolled, not BDK-backed: `bitcoin`/`bdk_wallet` (this project's BTC dependency) has no Litecoin variant in its `Network` enum, and the only BDK-quality alternative — LitecoinDevKit's fork — isn't on crates.io (git-only, pinned-commit). Chose to hand-roll rather than take this project's first non-released dependency.
- Reuses `bitcoin`'s network-agnostic pieces directly: BIP32 derivation (`m/84'/2'/0'/0/0` mainnet, `m/84'/1'/0'/0/0` testnet — SLIP-44 coin type 2, testnet convention 1), `Transaction`/`TxIn`/`TxOut`/`Witness` wire types, BIP143 sighash, secp256k1 signing — none of these depend on the `Network` enum for byte-level correctness, only address/WIF *text encoding* do
- Hand-written where Litecoin actually differs: bech32 `ltc1…`/`tltc1…` addresses (new `bech32` crate) and WIF with Litecoin's version byte (`0xB0` mainnet / `0xEF` testnet — reuses `sha2`/`bs58`, already dependencies from the Solana work, no new crate needed there)
- Signing correctness verified against BIP143's published "Native P2WPKH" test vector: an independent ECDSA verification (not a hardcoded "expected sighash" string) confirms the sighash computation, and RFC6979 determinism confirms the exact published signature is reproduced
- Backend: `litecoinspace.org` (Litecoin Foundation's mempool.space-fork explorer, Esplora/electrs-ltc compatible) — hand-rolled HTTP JSON calls, same style as `eth_chain.rs`/`sol_chain.rs`, not the `bdk_esplora`/`esplora-client` crates (those are built around `bdk_wallet::Wallet`, which isn't used here)
- Single reused address per wallet (mirrors how `BitcoinWallet` already behaves in practice — `peek_address(...,0)` is deterministic, no cross-session rotation), simple largest-first coin selection, exact vsize via `Transaction::vsize()` rather than an approximation formula
- Native LTC send/receive/lock, balance sync, Activity history; mainnet send gated behind the same "I understand this spends real…" checkbox as BTC/ETH/SOL
- Not yet added: RBF, an Electrum-protocol backend option (Esplora-only, unlike BTC's Esplora-or-Electrum choice), any fungible/token concept on Litecoin

### Phase 9 — Visual design pass — **done** (desktop; not yet tapped on the L5)

The screens worked but were built from bare `GtkBox`/`GtkLabel`/`GtkEntry` stacks, so the app
read as assembled rather than designed. This pass rebuilt the presentation layer on
libadwaita's own patterns without changing any wallet, signing or network behaviour.

- New `src/Views/ui.rs`: single source of truth for gutters, buttons, labels, chain
  identity and toasts, so spacing no longer drifts per screen
- `src/style.css` rewritten as a design system on libadwaita's named colours
  (`@card_bg_color`, `@accent_bg_color`, …), so light mode, dark mode and the user's accent
  colour all follow the system with no second stylesheet
- Home: portfolio hero card, chain-grouped asset rows with press feedback and fiat
  sublines; Assets grouped per chain; Activity rows split into direction badge, amount and
  monospace metadata instead of one run-on string
- Settings rebuilt as an `AdwPreferencesPage`: every control now has a title and a
  subtitle. Previously it was an unlabelled column of dropdowns and entries whose only
  clue was placeholder text that vanished on first keystroke
- Send screens share one chrome (`SendChrome`) so the four chains cannot drift apart, with
  labelled `AdwEntryRow`s and a review card whose network strip is green for testnet and
  amber for real value
- Wallets shows all four chains at once in titled groups instead of hiding every account
  behind a per-chain toggle button
- Onboarding and unlock: centred lockups, step headers, seed words as numbered chips,
  `AdwPasswordEntryRow` throughout
- Toasts (`AdwToastOverlay`) confirm copies, saves and broadcasts — these were previously
  silent, with no way to tell whether a tap had registered

Fixed along the way:

- **Tab bar and account icons were broken glyphs.** `"home"`, `"wallet"`, `"assets"` and
  `"BTC"`/`"ETH"` were passed as icon-theme names; none of them resolve. All icon names are
  now checked against the shipped Adwaita theme.
- **Missing token logos rendered GTK's broken-image icon.** LTC has no bundled PNG, and
  add-by-contract tokens never will. `ui::coin_mark` falls back to a chain-coloured monogram.
- **QR codes could not scan in dark mode.** A QR is dark modules on a light field; on a dark
  background it had no quiet zone. It now sits on a deliberately white `.qr-frame` in both themes.
- **Enter did not submit the unlock password.** `connect_activate` on an `AdwPasswordEntryRow`
  compiles but binds `GtkListBoxRow`'s own activate signal, not the text field's;
  `connect_entry_activated` is the correct one.
- **Same-symbol tokens on different chains looked identical.** Rows now carry a chain tag,
  suppressed where it would only repeat the coin's own name.

Dependency: `libadwaita` gains the `v1_2` feature, for `AdwEntryRow`/`AdwPasswordEntryRow`.
Deliberately not higher — the only API above that this design wanted was `AdwSwitchRow`
(1.4), and `ui::add_switch_row` hand-builds the same thing from `AdwActionRow` +
`GtkSwitch` (including `set_activatable_widget`, so the whole row stays a hit target).
That keeps Debian bookworm (libadwaita 1.2) buildable; `packaging/debian/control` carries
the matching `>= 1.2` floor.

Not done: a bundled LTC/BNB/AVAX icon asset (the monogram covers it), an `AdwNavigationView`
migration for the in-tab stacks, and any on-device tap-through — that stays owed to
[LIBREM5.md](LIBREM5.md).

---

## Extended set (after MVP)

L2s shipped ahead of schedule — see Phase 7 above. Solana shipped ahead of schedule — see Phase 6 above.

### P1 — still the core wallet

- Extra accounts from the same seed; rename; hide empty
- Address book + recent recipients
- Fiat source you choose, or hide prices
- BIP39 passphrase (25th word)
- Custom ERC-20s + hide spam
- BTC RBF / cancel-replace; ETH speed-up
- Export xpub (watch-only later)
- Tor SOCKS for Electrum/RPC
- Seed reveal + encrypted backup to microSD

### P2 — remaining items

L2s (see Phase 7) and Solana (see Phase 6) already shipped. Still open: Lightning (unless BTC-only users demand it first), BIP86 Taproot, WalletConnect v2, watch-only, and an in-app bridging UI for the L2s (deliberately not built yet — Phase 7 is send/receive only, no bridge).

### P3 — swap / DeFi, no KYC

- Trade as **local-signed** swap (user RPC + a router that is not 1inch portal KYC)
- Only tokens the user holds, on the selected chain
- SOL swaps only after SOL send/receive
- Do **not** restore `api.1inch.io/v5.0`

### P4 — hardware / platform

- USB-C hardware wallet
- Librem Key / OpenPGP unlock 2FA if the SKU has a reader
- Phosh receive notifications
- Docked sidebar vs handheld bottom bar

### P5 — maybe never

NFT gallery, fiat on-ramp, social recovery, built-in full node (3 GB / 32 GB).
