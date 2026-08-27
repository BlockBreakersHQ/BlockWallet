<p align="center">
  <img src="Images/Logo.png" alt="Block Wallet" width="380">
</p>

<p align="center">
  Self-custody Bitcoin, Ethereum, Solana and Litecoin wallet for the
  <a href="https://puri.sm/products/librem-5/">Librem 5</a> (PureOS / Phosh)
  and other GTK4 Linux desktops.
</p>

<p align="center">
  <img src="docs/screenshots/home-light.png" alt="Home screen" width="230">
  <img src="docs/screenshots/receive-light.png" alt="Receive address with QR code" width="230">
  <img src="docs/screenshots/home-dark.png" alt="Home screen in dark mode" width="230">
</p>

**Status:** v0.2.0 is the current release, built from a pinned tag and distributed as a Flatpak
bundle you install by hand. The last full [Librem 5 checklist](docs/LIBREM5.md) run was v0.1.0,
which passed all 41 of its boxes on real hardware in August 2026. v0.2.0 has been spot-checked
on the same device rather than re-run end to end: it installs, launches, syncs against live
nodes with the batched token read working, and renders a zero-balance wallet correctly. The
rest of the checklist is owed.

Known unproven areas, stated plainly rather than buried: cross-chain swaps have never moved
real coins, because THORChain's global trading halt was in force for most of development and
every free public THORNode gateway went dark once it lifted, while Maya reports a halt of its
own; swap quoting is only exercisable on mainnet, since no testnet has aggregator liquidity;
Token-2022 mints on Solana are excluded rather than shown as unspendable; and the Activity view
was not visually confirmed during a network outage. See the checklist for the full record.

Application ID: `io.github.BlockBreakersHQ.BlockWallet`

---

## Install

### Flatpak bundle (recommended)

Block Wallet is distributed as a `.flatpak` bundle you download and install yourself rather
than through Flathub. The bundle carries its own GNOME 50 runtime, so it does not depend on
what the distribution ships and behaves the same on every PureOS release.

| PureOS | Base | Native GTK4 / libadwaita | Native build |
| --- | --- | --- | --- |
| Byzantium | Debian bullseye | none | not possible |
| **Crimson** | Debian bookworm | GTK 4.8.3, libadwaita 1.2.2 | possible, but see [From source](#from-source) |

Verified on a Librem 5 running **PureOS 11 (Crimson)**, kernel 6.12.0-1-librem5: the aarch64
bundle installs, appears in the Phosh app grid, and runs with no errors on stderr and a flat
57 MB resident. Note that this exercises the runtime's GTK 4.22, not Crimson's own 4.8.3.
Those are separate code paths, and only the Flatpak one has been run on hardware.

**1. Download the bundle for your architecture** from the
[latest release](https://github.com/BlockBreakersHQ/BlockWallet/releases/latest), together
with `SHA256SUMS`. Use `aarch64` for the Librem 5, `x86_64` for a desktop.

**2. Check what you downloaded.** This is a wallet, so verify the file before installing it.

```sh
sha256sum --check --ignore-missing SHA256SUMS
```

That must print `OK` for the file you downloaded. If it does not, stop and do not install it.

**3. Make sure the GNOME runtime is available.** The bundle carries the app but not the
runtime underneath it, which comes from Flathub. Most systems already have this remote, and
adding it again is harmless.

```sh
flatpak remote-add --if-not-exists --user flathub \
  https://dl.flathub.org/repo/flathub.flatpakrepo
```

**4. Install and run.**

```sh
flatpak install --user ./BlockWallet-v0.2.0-aarch64.flatpak
flatpak run io.github.BlockBreakersHQ.BlockWallet
```

It then appears in the Phosh app grid, or your desktop's launcher, like any other app. The
first install also pulls the GNOME runtime, which is a few hundred MB. Later ones do not.

**Updating.** A bundle has no update channel, so nothing checks for new versions on your
behalf. Download the next release and run the same `flatpak install` command, which upgrades
in place. Your wallet lives outside the app, in
`~/.var/app/io.github.BlockBreakersHQ.BlockWallet/`, so it survives both upgrades and
uninstalls. Removing the wallet is a separate, deliberate act:

```sh
flatpak uninstall --user io.github.BlockBreakersHQ.BlockWallet   # keeps your wallet
rm -rf ~/.var/app/io.github.BlockBreakersHQ.BlockWallet          # deletes it
```

Building the bundle yourself is covered in [docs/packaging.md](docs/packaging.md).

### From source

Needs Rust 1.85+, GTK 4.6+, and libadwaita 1.2+.

```sh
# Debian bookworm / trixie, PureOS Crimson
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libssl-dev

cargo build --release
cargo test
```

**On PureOS Crimson the distribution's Rust is too old.** Crimson's `rustc` is 1.63 and this
crate needs 1.85, so install a toolchain from [rustup](https://rustup.rs) rather than
`apt install rustc cargo`. The GTK side is fine: Crimson's GTK 4.8.3 and libadwaita 1.2.2
both clear the floor, and `libgtk-4-dev` / `libadwaita-1-dev` are in its repositories.

That libadwaita floor is deliberate. `Cargo.toml` pins the `v1_2` feature and
`ui::add_switch_row` hand-builds what `AdwSwitchRow` would give, precisely so this still
compiles against the 1.2.2 that Crimson ships. Raising it to `v1_4` would break the native
build on the device this app targets.

Building on the phone itself is slow: 4 cores and 3 GB of RAM against roughly 580 crates.
Prefer building on a faster machine and copying the result over.

Windows (MSYS2 mingw64 + GNU rustc). Plain `cargo run` uses MSVC and has no
`pkg-config`/GTK, so use the wrapper:

```powershell
.\scripts\run-windows.ps1
```

## What it is

Block Wallet is a wallet you actually hold the keys to, built for a Linux phone.

**One recovery phrase, four chains.** A single BIP39 phrase derives every account: Bitcoin
(BIP84 `bc1q…`), Ethereum (BIP44), Solana (SLIP-0010 ed25519) and Litecoin. There is no
account to create, no email, no KYC, and nothing to sign up for. Write the phrase down and
that is the whole backup.

**Your keys never leave the device.** The store is a single encrypted file (Argon2id at
64 MiB for the password, ChaCha20-Poly1305 for the contents), written owner-only under your
XDG data directory, and replaced atomically so an interrupted save cannot corrupt it.
Anyone who copies that file can attack it offline at their own pace, so how strong a password
you pick is what protects it. Every copy of your keys is wiped from memory when it goes out of
scope, not just the one the lock button reaches. The Flatpak sandbox is *not* granted access
to your home directory, so even the packaged build can only see its own data. Logs are short
context strings that never contain a phrase, key or password.

**Your nodes, not ours.** Every chain talks to an endpoint you choose: an Electrum or
Esplora server for Bitcoin, any JSON-RPC for Ethereum and Solana, an Esplora-style server
for Litecoin. The defaults are public endpoints so it works out of the box, but those
endpoints see which addresses you ask about. Point it at your own server in Settings and
that stops. Remote endpoints must use TLS; plaintext `http://` is accepted only for a node
running on the device itself. Nothing a node says is taken on trust: fee estimates are
capped, and a fee larger than the amount being sent is refused rather than shown.

**It asks a node as little as it can.** A sync costs the same whether the wallet is tracking
four tokens or three hundred: Ethereum balances are read in a single `eth_call` through
Multicall3, token history in two `eth_getLogs` queries covering every contract at once, and
Solana holdings in one `getTokenAccountsByOwner`. This is not only politeness to a free public
endpoint. A wallet that hammers one gets rate-limited, and a rate-limited node is
indistinguishable from being offline, which is exactly how Bitcoin appeared broken for the
whole of this project's early life. Multicall3 is used for reads only, never for anything
signed, so it can affect what you see but never what you spend.

**Receiving works with the radios off.** Addresses and QR codes are derived locally, so
the receive screen is fully usable in airplane mode or when a node is unreachable. The app
says so plainly rather than showing a spinner.

**It tries hard not to let you lose money.** Any network that spends real value requires an
explicit "I understand" checkbox before the Confirm button becomes active, and that gate
keys off *"is this a known testnet"*, not *"is this literally mainnet"*, so a newly added
L2 defaults to protected rather than unprotected. Consent is per send, not per screen: the
box is unticked again for every transaction. What you confirm is always what you reviewed:
changing the recipient, amount, fee or account tears the review card down rather than
leaving Confirm wired to the previous plan, and each plan is bound to the account it was
built for, so it cannot be signed by another. The header carries a permanent LIVE or TEST
NETWORKS chip so the answer is never more than a glance away.

**It is shaped like a phone app.** 360×720, touch-sized targets, a bottom tab bar, and
libadwaita's own patterns throughout: boxed lists, preference groups, status pages,
toasts. It follows the system accent colour, and **Settings → Appearance** either follows the
system light/dark theme or pins one of them. That choice is stored outside the encrypted
store, so the lock screen is themed correctly before you have typed anything.

## Screens

| | | |
|:--:|:--:|:--:|
| ![Unlock](docs/screenshots/unlock-light.png) | ![Home](docs/screenshots/home-light.png) | ![Wallets](docs/screenshots/wallets-light.png) |
| Unlock | Home | Accounts, one phrase behind all of them |
| ![Receive](docs/screenshots/receive-light.png) | ![Send](docs/screenshots/send-light.png) | ![Unlock dark](docs/screenshots/unlock-dark.png) |
| Receive, works offline | Send | Dark mode |
| ![Home dark](docs/screenshots/home-dark.png) | ![Detail dark](docs/screenshots/detail-dark.png) | |
| Home in dark mode | Asset detail in dark mode | |

Captured at 360×720, the Librem 5's logical resolution.

## Supported chains

| Chain | Derivation | Networks | Notes |
| --- | --- | --- | --- |
| **Bitcoin** | BIP84 `m/84'/0'/0'/0/0` | mainnet, testnet | BDK 1.x, Electrum or Esplora, fee tiers, RBF-ready PSBT flow |
| **Ethereum** | BIP44 `m/44'/60'/0'/0/0` | mainnet, Sepolia, **Arbitrum One, Base, Optimism, Polygon PoS, BNB Smart Chain, Avalanche C-Chain** | Alloy. One address across all of them. Native gas token follows the chain (ETH / POL / BNB / AVAX) |
| **Solana** | SLIP-0010 ed25519 `m/44'/501'/0'/0'` | mainnet, devnet | SOL and SPL tokens, hand-rolled transaction format, no `solana-sdk` dependency |
| **Litecoin** | BIP84-style `m/84'/2'/0'/0/0` | mainnet, testnet | Hand-rolled: no Litecoin fork of `bitcoin`/`bdk_wallet` exists on crates.io, so this reuses the project's BIP32/secp256k1/BIP143 code and adds Litecoin's own bech32 and WIF encoding |

Tokens: about 315 bundled entries, covering roughly 275 ERC-20s across the seven EVM networks
and the top 40 SPL tokens on Solana. Every contract address and mint was verified on-chain
before it went in, by reading `symbol()` and `decimals()` from the contract or the mint
account itself rather than trusting a listing file. That check earns its keep: the curated
source list had FLUX at 18 decimals where all three of its contracts report 8, which would
have misreported the balance by ten orders of magnitude. You can add any other ERC-20 by
contract or any SPL token by mint address.

Where a token's on-chain symbol differs from the one commonly used, the on-chain value is
what the wallet shows. Most of those are Avalanche bridge assets, whose contracts genuinely
report `WETH.e`, `LINK.e` and so on: if you hold the bridged asset, the wallet says so rather
than implying you hold the native one. The two contracts whose symbol cannot be displayed at
all fall back to the conventional name rather than being mangled into something that looks
right but is not.

Solana coverage is classic SPL only. Token-2022 mints are deliberately left out: the wallet
derives associated token accounts and builds transfers against the classic program id, so
bundling one would show a balance that could not be spent, which is worse than not listing it.

Because the list is long, the swap screen picks tokens through a searchable list rather than a
dropdown. Typing filters on the whole label, so a chain name narrows it as well as a symbol.

Home shows all four chains while the wallet is empty, so a new wallet looks like a wallet
rather than a blank page, and narrows to just what you hold the moment a balance lands. It
needs no setting: the rule flips itself.

The Assets screen only lists tokens you actually hold, plus each chain's own asset so there is
always a way in to receive. **Settings -> Hide empty assets** hides those too, with a button on
the list to bring them back for a visit. A balance that is still syncing, or that could not be
fetched because a node is unreachable, is never hidden: both read as zero, and treating either
as empty would hide something you own at the exact moment you cannot check.

Import from a mnemonic, a WIF, a raw private key, or an existing encrypted `.dic`.

## Swaps

Swaps are non-custodial. No company ever holds your coins, and every transaction is signed on
the device.

Several venues are asked at once and their offers ranked by what you actually receive:

| Venue | Covers | Custody |
| --- | --- | --- |
| **LI.FI** | Same-chain swaps on Ethereum and every supported L2 | Settles in one transaction |
| **KyberSwap** | Same-chain swaps on Ethereum and every supported L2, quoted alongside LI.FI | Settles in one transaction |
| **Jupiter** | Same-chain swaps on Solana, SOL and SPL tokens | Settles in one transaction |
| **THORChain** | Cross-chain: BTC, LTC and ETH-family assets | A protocol vault holds the inbound side until the outbound side settles |
| **Maya Protocol** | Cross-chain: BTC and ETH-family assets | A protocol vault holds the inbound side until the outbound side settles |

Two aggregators are asked for every EVM pair rather than one, because they disagree, sometimes
by a lot, and an outage at one no longer means "no route" for every pair. The same reasoning
applies to running Maya alongside THORChain: they are separate networks with separate pools
and separate halt states, so the better price genuinely varies, and one being down does not
take cross-chain swapping with it.

Bitcoin and Litecoin have no on-chain DEX, so a vault-based venue is the only route for them.
That means the funds are briefly out of your control between the two legs, which the app says
plainly on the offer and again on the review screen before you can confirm. Litecoin goes
through THORChain specifically: Maya has no LTC pool, and says so before making a request.

### Why aggregators rather than individual exchanges

There is no Uniswap entry in that table, nor PancakeSwap, Curve or SushiSwap, and that is
deliberate: **the wallet already swaps on all of them.** LI.FI and KyberSwap are aggregators.
They quote across dozens of venues and pick whichever is best for that pair at that moment, so
a single EVM swap routinely executes across several at once. The review card names the ones it
used.

Adding a DEX directly could only ever match what an aggregator already returns, never beat it,
because the aggregator can always choose that DEX too. It would also move the construction of
swap calldata out from behind a bounded interface and into this codebase, where a mistake in
path encoding or router arguments is a mistake that loses money. That trade is not worth making
for a price that is, at best, identical.

The one thing this costs is resilience: if both aggregators are unreachable, every same-chain
EVM pair loses its route. A minimal direct-to-Uniswap fallback would fix that, and is the only
form in which adding a single DEX makes sense here.

Hyperliquid is a different case and is deliberately absent. It is an order book on its own
chain rather than an AMM, it requires depositing assets into its system first, and it deals
mainly in leveraged perpetuals. None of those fit a wallet whose whole model is one signed
transaction, a guaranteed minimum output, and never handing custody to anyone.

Block Wallet asks each venue for a 1% affiliate fee, deducted by the venue itself rather than
by an extra transaction. It is shown as a single Swap fee line on every offer and again on
the review card before anything can be confirmed. Where a venue reports its own total, that
figure already includes this fee, so the two are never added together. Only the EVM aggregators
have a payout address built in, so only they are asked for a fee.

What the wallet checks before it will sign, on every offer:

- the proceeds are addressed to your own wallet, verified against the THORChain memo itself
  rather than the provider's claim about it;
- a minimum output exists and implies no more slippage than you allowed;
- the amount clears the provider's own minimum, since THORChain forfeits rather than refunds
  anything below it;
- the quote has not expired;
- ERC-20 approvals are for exactly the swap amount and only to the contract being called, so
  no standing allowance is left behind;
- a Solana transaction built elsewhere is payable only by your own account.

Providers that decline say why rather than quietly disappearing, and the reason is worth
reading, because the two cross-chain venues are currently unavailable for entirely different
reasons.

**THORChain the network is healthy.** It settled around $24M of swaps in the last 24 hours and
its 30-day volume is up roughly 83% month over month. What has gone is the free public API
layer: at the time of writing the long-standing default gateway has no DNS record at all, one
alternative sits behind a Cloudflare challenge, and a third serves a certificate that expired
in February 2024 and fronts a node frozen at a single block height, which refuses to quote
and says so. The wallet tries each in turn and then reports the failure honestly rather than
presenting it as your connection being down. **Maya** answers normally and reports its own
trading halt.

So this is an infrastructure-access problem with an ordinary fix, not a dead dependency. If
you run your own node, set it in **Settings -> Swaps** and it is tried first, ahead of the
public list. The cross-chain paths are unit-tested against captured responses but have not
moved real coins.

## Trying it safely

`BLOCKWALLET_HOME` relocates the entire profile, so a test run cannot touch an existing
wallet:

```sh
BLOCKWALLET_HOME=~/blockwallet-test RUST_LOG=debug block_wallet
```

Turn on **Settings → Use test networks** for Bitcoin testnet, Ethereum Sepolia, Solana
devnet and Litecoin testnet in one switch. The header chip turns green.

## Data locations

User files follow the XDG Base Directory spec. Override the root with `BLOCKWALLET_HOME`.

| What | Linux default | Flatpak |
| --- | --- | --- |
| Encrypted wallet (`Config.dic`) | `~/.local/share/blockwallet/` | `~/.var/app/io.github.BlockBreakersHQ.BlockWallet/data/blockwallet/` |
| Network settings (`network.yml`) | `~/.config/blockwallet/` | `…/config/blockwallet/` |
| Log (`blockwallet.log`) | `~/.local/state/blockwallet/` | `…/data/blockwallet/` |
| Backups | `~/.local/share/blockwallet/backups/` | `…/data/blockwallet/backups/` |

## Packaging and device QA

- Flatpak manifests: `data/io.github.BlockBreakersHQ.BlockWallet.json` (release, offline
  vendored build) and `…Devel.json` (local iteration)
- Debian/PureOS sketch: `packaging/debian/`
- How to build and install: [docs/packaging.md](docs/packaging.md)
- Pre-submission checks: `./scripts/validate-packaging.sh`
- Librem 5 manual checklist: [docs/LIBREM5.md](docs/LIBREM5.md)

Progress and remaining work: [docs/ROADMAP.md](docs/ROADMAP.md).

`v0.1.0` is tagged, and release builds are produced for both `aarch64` (the Librem 5) and
`x86_64` (desktop). Work since that tag is unreleased and is being re-tested against the
expanded checklist.

## License

[GNU General Public License v3.0 or later](LICENSE).
