# Store screenshots

`data/io.github.BlockBreakersHQ.BlockWallet.metainfo.xml` references three files in this directory by
raw GitHub URL. **They must exist on the default branch before submitting to any store** —
Flathub requires at least one reachable https screenshot and will reject the submission
otherwise.

| File | Shows |
| --- | --- |
| `home.png` | Home tab with balances across the configured chains |
| `receive.png` | A receive address with its QR code |
| `send.png` | The send review card before broadcasting |

Capture them **on the Librem 5** during the [device checklist](../../docs/LIBREM5.md), not
on a desktop:

- Real device size (360x720), so the store preview matches what a phone user gets.
- Use **test networks** and a throwaway profile (`BLOCKWALLET_HOME=~/blockwallet-test`).
  Never publish a screenshot showing a mainnet address or a real balance — a receive
  address in a store listing is public forever and links the app to a real wallet.
- No window decorations or desktop background; crop to the app.
- Take one set in light and check they are legible; the store shows them on a light card.

On Phosh, `Print` screen or `grim` will capture the window.
