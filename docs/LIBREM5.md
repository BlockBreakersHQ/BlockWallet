# Librem 5 test checklist

A **per-release** gate, not a one-off. Reset the boxes for each version under test and work
through it again on real hardware; completed runs are recorded at the bottom.

Run it on a real Librem 5 (Phosh, aarch64, ~3 GB RAM). Prefer **Bitcoin testnet**, **Ethereum
Sepolia**, **Solana devnet**, and **Litecoin testnet**. Do not use mainnet funds.

Install with Flatpak (`docs/packaging.md`) or a `.deb`. Building natively on the phone takes
hours, and Crimson's own `rustc` is too old for this crate; see the packaging notes.

**Currently under test:** the untagged tree after v0.1.0 — the swap feature, the security
hardening, and the fixes that followed the v0.1.0 run. The crate version is still `0.1.0`;
the next version number has not been decided.

## 1. Install

- [ ] App appears in the app grid as **Block Wallet**
- [ ] Icon and `.desktop` `X-Purism-FormFactor=Mobile` are present
- [ ] First launch opens at about 360×720, not a desktop-only layout
- [ ] Touch targets (Unlock, tab bar, Send/Receive) are usable with a finger
- [ ] Tab bar shows five real icons (Home / Wallets / Assets / Swap / Activity), not missing-image glyphs
- [ ] Switch Phosh to dark mode: cards, banners and text all follow it, and the receive QR keeps a white frame that still scans
- [ ] **Settings → Display → Appearance**: Light and Dark apply immediately, and "Follow system" tracks a Phosh theme change while the app is open
- [ ] The chosen appearance survives a restart, including on the unlock screen itself
- [ ] Change the system accent colour: buttons and the balance card follow it

## 2. Onboard

- [ ] Create 12-word wallet: write down phrase, confirm, set password
- [ ] Force-quit and reopen: unlock screen, not a second create flow
- [ ] Restore on a throwaway profile with a known test phrase; BTC address is BIP84 `tb1q`/`bc1q`, ETH is `0x…`, SOL is a base58 address, and LTC is `ltc1q…`/`tltc1q…`
- [ ] Lock wipes keys from the UI; Wallets does not show the mnemonic until Reveal + password

## 3. Receive (radios on and off)

- [ ] Settings → **Use test networks** → Save
- [ ] Bitcoin receive: address + QR visible
- [ ] Ethereum receive: address + QR visible
- [ ] Solana receive: address + QR visible
- [ ] Litecoin receive: address + QR visible
- [ ] Enable airplane mode / kill switches: receive address and QR still show
- [ ] Banner says the node is unreachable / receive still works
- [ ] Copy an address, then copy something else in another app: the second value is still on the clipboard a minute later (addresses must not auto-clear)
- [ ] Reveal the recovery phrase and copy it: the clipboard clears within 30 seconds, but only if nothing else was copied in the meantime
- [ ] Paste into a Send recipient field works
- [ ] Each receive QR is large enough to scan from another phone at arm's length

## 4. Send (testnet)

- [ ] Fund the testnet BTC address from a faucet; wait for a confirmation
- [ ] Send a small amount: Review → summary shows **testnet** → Confirm and broadcast
- [ ] The sent transaction appears in **Activity** with no Etherscan or other API key configured
- [ ] Mainnet send requires the “I understand this spends real bitcoin” checkbox (spot-check by turning test networks off; do not broadcast)
- [ ] Tap Review, then change the recipient or amount: the review card disappears rather than staying armed with the old plan
- [ ] Send twice in one visit to the screen: the acknowledgement checkbox is unticked again for the second send
- [ ] ETH Sepolia: send a tiny amount of ETH the same way
- [ ] ETH recipient accepts an **ENS name**; the review card shows the resolved `0x…` address, not the name
- [ ] Optional: send a Sepolia ERC-20 if you have a test token
- [ ] SOL devnet: airdrop with `solana airdrop`, send a tiny amount back; Review → summary shows **devnet**
- [ ] Optional: send the bundled devnet USDC-SPL (or a token added by mint address) to confirm the associated-token-account creation path works
- [ ] Settings → switch the ETH network dropdown to an L2 (Arbitrum/Base/Optimism/Polygon/BSC/Avalanche): balance row shows the correct native symbol (MATIC/BNB/AVAX where applicable, ETH otherwise), and the "I understand this spends real value" checkbox is visible and gates Confirm on send
- [ ] LTC testnet: fund from a faucet, send a small amount back; Review → summary shows **testnet**; mainnet send requires the "I understand this spends real litecoin" checkbox (spot-check by turning test networks off; do not broadcast)

## 5. Swap

Be clear about what is testable where. On testnets every provider correctly declines: LI.FI has
no Sepolia liquidity, Jupiter none on devnet, and THORChain refuses while its network-wide
`HALTTRADING` mimir is set. The testnet rows therefore verify the *refusal* path, which is
where the v0.1.0 UI freeze lived. Real quoting needs mainnet and a live THORChain.

- [ ] Swap tab opens and lists the wallet's assets in both dropdowns
- [ ] On testnet, **Find the best rate** returns promptly and does **not** hang the app
- [ ] It reports that no provider could route the swap, and lists why each declined
- [ ] Changing the amount or either asset clears any shown offer
- [ ] Mainnet, small amount: at least one provider quotes, and offers are ordered best-output-first
- [ ] A THORChain offer is labelled as vault-held; a LI.FI or Jupiter offer is labelled as settling in one transaction
- [ ] The review card shows the minimum received, and Confirm is gated behind the acknowledgement
- [ ] Optional (mainnet, real value): execute a small same-chain swap and confirm it lands

## 6. Lock and settings

- [ ] Header **Lock** returns to Unlock
- [ ] Wrong password stays locked
- [ ] Auto-lock fires after idle
- [ ] Auto-lock does **not** fire while the app is in use by touch alone (tap around for longer than the timeout)
- [ ] When auto-lock fires while another app is in front, Block Wallet does **not** raise itself
- [ ] Change BTC Electrum/Esplora URL, ETH RPC, SOL RPC, and LTC Esplora URL; Save; a toast confirms and balances refresh or show offline honestly
- [ ] A plaintext `http://` endpoint is rejected unless it points at localhost
- [ ] Every Settings row has a readable title and subtitle at 360 px wide; nothing is clipped or requires horizontal scrolling
- [ ] Header shows the LIVE / TEST NETWORKS chip and it matches the network actually in use
- [ ] Fiat prices stay optional (CoinGecko); send/receive work with prices off

## 7. Kill switch / privacy

- [ ] With WWAN/Wi-Fi off, **Home** does not panic and shows the unreachable-node banner
- [ ] With WWAN/Wi-Fi off, **Activity** does not panic (bring it to the foreground during the outage)
- [ ] Balances show the last known figure marked offline rather than blanking to nothing
- [ ] No mnemonic or private key in `~/.local/state/blockwallet/blockwallet.log` (or the Flatpak equivalent under `~/.var/app/io.github.BlockBreakersHQ.BlockWallet/`)

## 8. Package

- [ ] aarch64 Flatpak **or** `.deb` installs and launches
- [ ] Uninstall does not leave keys outside XDG / Flatpak app data

---

## Completed runs

### v0.1.0 — 25 to 26 August 2026, PureOS 11 (Crimson)

All 41 boxes of the then-current checklist passed on a Librem 5 running Crimson
(`/etc/debian_version` 12.0, kernel 6.12.0-1-librem5), installed as an aarch64 Flatpak and
driven over SSH with the device attached by USB. `grim` was used for screen capture, which
works on Crimson's compositor where it did not on Byzantium.

Two caveats from that run, recorded rather than smoothed over:

- **Activity was never displayed while offline.** The app was proven to survive 75+ seconds
  fully offline (`wlan0` down, no route) with zero panics, and Home was captured rendering its
  unreachable-node banner correctly. But Activity itself was never brought to the foreground
  during the outage. It reads the same history `Arc` that the balance path writes and makes no
  network calls of its own, so a panic there but not in Home is unlikely — that is reasoning,
  not observation. It is now its own row in section 7 so the next run covers it explicitly.
- **Swaps were not covered at all.** The checklist predated the feature, which is why a UI
  freeze on the swap screen reached a human tester instead of a test. Section 5 exists now.

Bugs found *after* that run finished, none of which the checklist as it then stood would have
caught: the swap-screen freeze, Bitcoin rate-limiting itself into a permanent "offline",
Ethereum history silently requiring an API key, auto-lock firing during touch-only use, the
clipboard wiping addresses copied in other apps, and two dead default RPC endpoints. Rows in
sections 3 through 7 were added to cover each of them.
