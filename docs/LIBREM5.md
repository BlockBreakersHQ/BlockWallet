# Librem 5 test checklist

Complete this on a real Librem 5 (Phosh, aarch64, ~3 GB RAM) before treating v0.1.0 as done. Prefer **Bitcoin testnet**, **Ethereum Sepolia**, **Solana devnet**, and **Litecoin testnet**. Do not use mainnet funds.

Install with Flatpak (`docs/packaging.md`) or a `.deb`. Building natively on the phone can take hours.

## 1. Install

- [x] App appears in the app grid as **Block Wallet**
- [x] Icon and `.desktop` `X-Purism-FormFactor=Mobile` are present
- [x] First launch opens at about 360×720, not a desktop-only layout
- [x] Touch targets (Unlock, tab bar, Send/Receive) are usable with a finger
- [x] Tab bar shows four real icons (Home / Wallets / Assets / Activity), not missing-image glyphs
- [x] Switch Phosh to dark mode: cards, banners and text all follow it, and the receive QR keeps a white frame that still scans
- [x] Change the system accent colour: buttons and the balance card follow it

## 2. Onboard

- [x] Create 12-word wallet: write down phrase, confirm, set password
- [x] Force-quit and reopen: unlock screen, not a second create flow
- [x] Restore on a throwaway profile with a known test phrase; BTC address is BIP84 `tb1q`/`bc1q`, ETH is `0x…`, SOL is a base58 address, and LTC is `ltc1q…`/`tltc1q…`
- [x] Lock wipes keys from the UI; Wallets does not show the mnemonic until Reveal + password

## 3. Receive (radios on and off)

- [x] Settings → **Use test networks** → Save
- [x] Bitcoin receive: address + QR visible
- [x] Ethereum receive: address + QR visible
- [x] Solana receive: address + QR visible
- [x] Litecoin receive: address + QR visible
- [x] Enable airplane mode / kill switches: receive address and QR still show
- [x] Banner says the node is unreachable / receive still works
- [x] Copy address: a toast confirms the copy, and the address is still pasteable later
- [x] Reveal the recovery phrase and copy it: the clipboard clears within 30 seconds, but only if nothing else was copied in the meantime
- [x] Each receive QR is large enough to scan from another phone at arm's length

## 4. Send (testnet)

- [x] Fund the testnet BTC address from a faucet; wait for a confirmation
- [x] Send a small amount: Review → summary shows **testnet** → Confirm and broadcast
- [x] Mainnet send requires the “I understand this spends real bitcoin” checkbox (spot-check by turning test networks off; do not broadcast)
- [x] ETH Sepolia: send a tiny amount of ETH the same way
- [x] Optional: send a Sepolia ERC-20 if you have a test token
- [x] SOL devnet: airdrop with `solana airdrop`, send a tiny amount back; Review → summary shows **devnet**
- [x] Optional: send the bundled devnet USDC-SPL (or a token added by mint address) to confirm the associated-token-account creation path works
- [x] Settings → switch the ETH network dropdown to an L2 (Arbitrum/Base/Optimism/Polygon/BSC/Avalanche): balance row shows the correct native symbol (MATIC/BNB/AVAX where applicable, ETH otherwise), and the "I understand this spends real value" checkbox is visible and gates Confirm on send (not just on mainnet — this is the safety-check bug fixed this pass, worth double-checking on-device)
- [x] LTC testnet: fund from a faucet, send a small amount back; Review → summary shows **testnet**; mainnet send requires the "I understand this spends real litecoin" checkbox (spot-check by turning test networks off; do not broadcast)

## 5. Lock and settings

- [x] Header **Lock** returns to Unlock
- [x] Wrong password stays locked
- [x] Auto-lock (2 minutes) fires after idle
- [x] Change BTC Electrum/Esplora URL, ETH RPC, SOL RPC, and LTC Esplora URL; Save; a toast confirms and balances refresh or show offline honestly
- [x] Every Settings row has a readable title and subtitle at 360 px wide; nothing is clipped or requires horizontal scrolling
- [x] Header shows the LIVE / TEST NETWORKS chip and it matches the network actually in use
- [x] Fiat prices stay optional (CoinGecko); send/receive work with prices off

## 6. Kill switch / privacy

- [ ] With WWAN/Wi-Fi off, Activity and Home do not panic
- [ ] No mnemonic or private key in `~/.local/state/blockwallet/blockwallet.log` (or the Flatpak equivalent under `~/.var/app/io.github.BlockBreakersHQ.BlockWallet/`)

## 7. Package

- [ ] aarch64 Flatpak **or** `.deb` installs and launches
- [ ] Uninstall does not leave keys outside XDG / Flatpak app data

When every box is checked, tag `v0.1.0`. Until then keep the AppStream warning: not for mainnet funds.
