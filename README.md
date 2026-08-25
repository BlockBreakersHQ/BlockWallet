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

**Status:** v0.1.0 packaged snapshot. Do not put mainnet funds in it until the
[Librem 5 checklist](docs/LIBREM5.md) is complete.

Application ID: `io.github.BlockBreakersHQ.BlockWallet`

---

## What it is

Block Wallet is a wallet you actually hold the keys to, built for a Linux phone.

**One recovery phrase, four chains.** A single BIP39 phrase derives every account: Bitcoin
(BIP84 `bc1q…`), Ethereum (BIP44), Solana (SLIP-0010 ed25519) and Litecoin. There is no
account to create, no email, no KYC, and nothing to sign up for. Write the phrase down and
that is the whole backup.

**Your keys never leave the device.** The store is a single encrypted file — Argon2id at
64 MiB for the password, ChaCha20-Poly1305 for the contents — written owner-only under your
XDG data directory, and replaced atomically so an interrupted save cannot corrupt it.
Passwords must be at least 12 characters, because anyone who copies the file can attack it
offline at their own pace. Every copy of your keys is wiped from memory when it goes out of
scope, not just the one the lock button reaches. The Flatpak sandbox is *not* granted access
to your home directory, so even the packaged build can only see its own data. Logs are short
context strings that never contain a phrase, key or password.

**Your nodes, not ours.** Every chain talks to an endpoint you choose: an Electrum or
Esplora server for Bitcoin, any JSON-RPC for Ethereum and Solana, an Esplora-style server
for Litecoin. The defaults are public endpoints so it works out of the box, but those
endpoints see which addresses you ask about — point it at your own server in Settings and
that stops. Remote endpoints must use TLS; plaintext `http://` is accepted only for a node
running on the device itself. Nothing a node says is taken on trust: fee estimates are
capped, and a fee larger than the amount being sent is refused rather than shown.

**Receiving works with the radios off.** Addresses and QR codes are derived locally, so
the receive screen is fully usable in airplane mode or when a node is unreachable. The app
says so plainly rather than showing a spinner.

**It tries hard not to let you lose money.** Any network that spends real value requires an
explicit "I understand" checkbox before the Confirm button becomes active — and that gate
keys off *"is this a known testnet"*, not *"is this literally mainnet"*, so a newly added
L2 defaults to protected rather than unprotected. Consent is per send, not per screen: the
box is unticked again for every transaction. What you confirm is always what you reviewed —
changing the recipient, amount, fee or account tears the review card down rather than
leaving Confirm wired to the previous plan, and each plan is bound to the account it was
built for, so it cannot be signed by another. The header carries a permanent LIVE or TEST
NETWORKS chip so the answer is never more than a glance away.

**It is shaped like a phone app.** 360×720, touch-sized targets, a bottom tab bar, and
libadwaita's own patterns throughout — boxed lists, preference groups, status pages,
toasts. It follows the system light/dark theme and accent colour.

## Screens

| | | |
|:--:|:--:|:--:|
| ![Unlock](docs/screenshots/unlock-light.png) | ![Home](docs/screenshots/home-light.png) | ![Wallets](docs/screenshots/wallets-light.png) |
| Unlock | Home | Accounts, one phrase behind all of them |
| ![Receive](docs/screenshots/receive-light.png) | ![Send](docs/screenshots/send-light.png) | ![Unlock dark](docs/screenshots/unlock-dark.png) |
| Receive — works offline | Send | Dark mode |
| ![Home dark](docs/screenshots/home-dark.png) | ![Detail dark](docs/screenshots/detail-dark.png) | |
| Home in dark mode | Asset detail in dark mode | |

Captured at 360×720, the Librem 5's logical resolution.

## Supported chains

| Chain | Derivation | Networks | Notes |
| --- | --- | --- | --- |
| **Bitcoin** | BIP84 `m/84'/0'/0'/0/0` | mainnet, testnet | BDK 1.x, Electrum or Esplora, fee tiers, RBF-ready PSBT flow |
| **Ethereum** | BIP44 `m/44'/60'/0'/0/0` | mainnet, Sepolia, **Arbitrum One, Base, Optimism, Polygon PoS, BNB Smart Chain, Avalanche C-Chain** | Alloy. One address across all of them. Native gas token follows the chain (ETH / MATIC / BNB / AVAX) |
| **Solana** | SLIP-0010 ed25519 `m/44'/501'/0'/0'` | mainnet, devnet | SOL and SPL tokens, hand-rolled transaction format — no `solana-sdk` dependency |
| **Litecoin** | BIP84-style `m/84'/2'/0'/0/0` | mainnet, testnet | Hand-rolled: no Litecoin fork of `bitcoin`/`bdk_wallet` exists on crates.io, so this reuses the project's BIP32/secp256k1/BIP143 code and adds Litecoin's own bech32 and WIF encoding |

Tokens: a bundled list (USDC, USDT, DAI, WBTC, SPL USDC, plus each L2's native token and a
stablecoin), and you can add any ERC-20 by contract or any SPL token by mint address.

Import from a mnemonic, a WIF, a raw private key, or an existing encrypted `.dic`.

Swapping is **not** in this release.

## Install

### Flatpak (recommended, and the only option on PureOS Byzantium)

Byzantium is Debian bullseye-based and has no GTK4 or libadwaita, so a native build cannot
work there — the Flatpak brings its own runtime.

From a built bundle:

```sh
flatpak install --user ./blockwallet-aarch64.flatpak
flatpak run io.github.BlockBreakersHQ.BlockWallet
```

From a hosted repo:

```sh
flatpak remote-add --if-not-exists --user blockwallet https://example.org/repo/blockwallet.flatpakrepo
flatpak install --user blockwallet io.github.BlockBreakersHQ.BlockWallet
flatpak run io.github.BlockBreakersHQ.BlockWallet
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

Windows (MSYS2 mingw64 + GNU rustc) — plain `cargo run` uses MSVC and has no
`pkg-config`/GTK, so use the wrapper:

```powershell
.\scripts\run-windows.ps1
```

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

After the device checklist passes:

```sh
git tag v0.1.0
```

## License

[GNU General Public License v3.0 or later](LICENSE).
