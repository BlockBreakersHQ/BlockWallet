# Packaging

Application ID: `org.BlockBreakers.Wallet`  
Binary: `block_wallet`  
Keys live in XDG data (`~/.local/share/blockwallet/` on a normal install, or the Flatpak app data dir). The sandbox does **not** need `--filesystem=home`.

## Flatpak (primary)

Runtime: GNOME 47. Network is required so Electrum/Esplora and Ethereum RPC can work; there is no `--filesystem=home`.

From the repo root:

```sh
flatpak-builder --user --install --force-clean build-dir data/org.BlockBreakers.Wallet.json
flatpak run org.BlockBreakers.Wallet
```

The manifest uses a local `dir` source (`..` from `data/`). For a published Flathub build, replace that source with a git tag and vendor crates (`cargo vendor` + `--offline`) instead of `build-args: --share=network`.

On aarch64 (Librem 5):

```sh
flatpak-builder --arch=aarch64 --user --install --force-clean build-dir data/org.BlockBreakers.Wallet.json
```

Expect a long compile on the phone. Cross-building with GNOME Builder or a faster aarch64 host is recommended.

## Debian / PureOS sketch

Files under `packaging/debian/` are a starting point, not a complete source package.

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libssl-dev cargo rustc
cargo build --release
sudo install -Dm755 target/release/block_wallet /usr/bin/block_wallet
sudo install -Dm644 data/org.BlockBreakers.Wallet.desktop /usr/share/applications/org.BlockBreakers.Wallet.desktop
sudo install -Dm644 data/org.BlockBreakers.Wallet.metainfo.xml /usr/share/metainfo/org.BlockBreakers.Wallet.metainfo.xml
sudo install -Dm644 data/icons/hicolor/256x256/apps/org.BlockBreakers.Wallet.png /usr/share/icons/hicolor/256x256/apps/org.BlockBreakers.Wallet.png
sudo mkdir -p /usr/share/blockwallet
sudo cp -a Images /usr/share/blockwallet/Images
```

To turn this into a `.deb`, copy `packaging/debian/` to a `debian/` directory in a source tree and run `dpkg-buildpackage -us -uc`.

Installed assets are loaded from `/usr/share/blockwallet/Images` or `/app/share/blockwallet/Images`. Wallet files stay under XDG.

## Version

Current crate version is `0.1.0`. After device QA:

```sh
git tag v0.1.0
git push origin v0.1.0
```
