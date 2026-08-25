# Block Wallet

Self-custody Bitcoin, Ethereum, Solana, and Litecoin wallet for the [Librem 5](https://puri.sm/products/librem-5/) (PureOS / Phosh) and other GTK4 Linux desktops.

**Status:** v0.1.0 packaged snapshot. Do not put mainnet funds in it until the [Librem 5 checklist](docs/LIBREM5.md) is complete.

Application ID: `org.BlockBreakers.Wallet`

## What works today

- GTK4 / libadwaita phone-sized window (360×720) with Home / Wallets / Assets / Activity
- Password-gated encrypted store (XDG data dir)
- Bitcoin BIP84 native SegWit (`bc1q…`) via BDK 1.x (Electrum/Esplora)
- Ethereum BIP44 HD via Alloy: ETH and ERC-20 send/receive against a user RPC — mainnet, Sepolia, or an L2/sidechain (Arbitrum One, Base, Optimism, Polygon PoS, BNB Smart Chain, Avalanche C-Chain), same address across all of them
- Solana SLIP-0010 ed25519 HD (`m/44'/501'/0'/0'`): SOL and SPL token send/receive against a user RPC (mainnet or devnet)
- Litecoin BIP84-style native SegWit (`ltc1q…`), hand-rolled (no Litecoin fork of `bitcoin`/`bdk_wallet` exists on crates.io): reuses this project's BIP32/secp256k1/transaction-signing code, adds Litecoin's own bech32 address and WIF encoding, and talks to a Litecoin Esplora-style RPC (mainnet or testnet)
- Bundled token list (ETH, USDC, USDT, DAI, WBTC, SOL, USDC-SPL, LTC, plus each L2's native token and a bundled stablecoin) plus add-by-contract / add-by-mint
- Import from mnemonic / WIF / private key / encrypted `.dic`

Swap is **not** in this release. Do not put mainnet funds in it until device QA is done. Default public RPCs see your addresses; set your own BTC Electrum/Esplora and ETH RPC in settings.

## Run

```sh
cargo run
```

Windows (MSYS2 mingw64 + GNU rustc). Do **not** use plain `cargo run` here — that is the MSVC toolchain and has no `pkg-config`/GTK:

```powershell
.\scripts\run-windows.ps1
```

Already built: `target/debug/block_wallet` or `target/debug/block_wallet.exe`.

## Build

Needs Rust 1.85+, GTK 4, and libadwaita development packages.

```sh
# Debian / PureOS
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev

cargo build
cargo test
```

On Windows, GTK must be on `PKG_CONFIG_PATH` (MSYS2 or gvsbuild). The Librem 5 is the target device.

Progress vs leftover work: [docs/ROADMAP.md](docs/ROADMAP.md).

## Data locations

User files follow the XDG Base Directory spec (`directories` crate). Override the root with `BLOCKWALLET_HOME`.

| What | Linux default |
| --- | --- |
| Encrypted wallet (`Config.dic`) | `~/.local/share/blockwallet/` |
| Network settings (`network.yml`) | `~/.config/blockwallet/` |
| Log (`blockwallet.log`) | `~/.local/state/blockwallet/` |
| Backups | `~/.local/share/blockwallet/backups/` |
| Cached token list / icons | `~/.cache/blockwallet/` |

Logs are short context strings. They must not contain mnemonics, private keys, or passwords.

```sh
RUST_LOG=debug cargo run
```

## Desktop / AppStream

- `data/org.BlockBreakers.Wallet.desktop`
- `data/org.BlockBreakers.Wallet.metainfo.xml`
- `data/icons/hicolor/256x256/apps/org.BlockBreakers.Wallet.png`

## Packaging and device QA

- Flatpak manifest: `data/org.BlockBreakers.Wallet.json` (no home filesystem access; keys stay in XDG / Flatpak app data)
- Debian/PureOS sketch: `packaging/debian/`
- How to build and install: [docs/packaging.md](docs/packaging.md)
- Librem 5 manual checklist: [docs/LIBREM5.md](docs/LIBREM5.md)

After that checklist passes on a phone:

```sh
git tag v0.1.0
```

## License

[GNU General Public License v3.0 or later](LICENSE).
