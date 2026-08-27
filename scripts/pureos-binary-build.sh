#!/usr/bin/env bash
# Build the block_wallet binary the PureOS Store manifest ships, reproducibly.
#
# sm.puri.Sdk 44's own rust-stable extension is 22.08 (rustc ~1.69), far below this crate's
# real 1.91 floor, so this does not use it. Instead it runs an upstream Rust toolchain inside
# the sandbox, linking against sm.puri.Platform 44's own GTK4 4.10.1 / libadwaita 1.3.1 /
# glibc 2.35 exactly as the shipped app will at runtime. Needs the pureos flatpak remote and
# sm.puri.Sdk/Platform 44 installed for the target arch:
#
#   flatpak remote-add --if-not-exists --user pureos https://store.puri.sm/repo/stable/pureos.flatpakrepo
#   flatpak install --user pureos runtime/sm.puri.Sdk/$ARCH/44 runtime/sm.puri.Platform/$ARCH/44
#
#   ./scripts/pureos-binary-build.sh aarch64   # or x86_64
set -uo pipefail
cd "$(dirname "$0")/.."

ARCH="${1:-x86_64}"
ID=io.github.BlockBreakersHQ.BlockWallet
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

RUST_VERSION=1.98.0
RUST_URL="https://static.rust-lang.org/dist/rust-${RUST_VERSION}-${ARCH}-unknown-linux-gnu.tar.xz"
echo "== fetching Rust ${RUST_VERSION} for ${ARCH} =="
curl -sL -o "$WORK/rust.tar.xz" "$RUST_URL"
tar -xf "$WORK/rust.tar.xz" -C "$WORK"
"$WORK/rust-${RUST_VERSION}-${ARCH}-unknown-linux-gnu/install.sh" \
    --prefix="$WORK/rust" \
    --components=rustc,cargo,"rust-std-${ARCH}-unknown-linux-gnu" \
    --disable-ldconfig

BD="$WORK/appdir"
flatpak build-init --arch="$ARCH" "$BD" "$ID" sm.puri.Sdk sm.puri.Platform 44

echo "== cargo build (release) started $(date -u +%H:%M:%S) =="
flatpak build --share=network --filesystem=host "$BD" sh -c "
    cd '$PWD' &&
    PATH='$WORK/rust/bin:/usr/bin:/bin' CARGO_TARGET_DIR='$WORK/target' \
    cargo build --release --locked
"
echo "== done $(date -u +%H:%M:%S) =="

OUT="block_wallet-pureos-${ARCH}"
cp "$WORK/target/release/block_wallet" "$OUT"
sha256sum "$OUT"
echo "Binary at ./$OUT — sha256 above is what data/$ID.PureOS.json pins."
