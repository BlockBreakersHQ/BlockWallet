#!/bin/sh
# Regenerate data/generated-sources.json from Cargo.lock.
#
# Stores (Flathub, and any curated repo) build with no network access, so every crate has
# to be declared as a source with a checksum up front. This wraps the upstream generator
# rather than reimplementing it — it handles registry crates, git dependencies and the
# vendored-sources cargo config correctly, and it is the version Flathub itself expects.
#
# Run this on a Linux host whenever Cargo.lock changes, then commit the result.
#
# Needs: python3, python3-aiohttp, python3-toml
set -eu

REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT="$REPO_ROOT/data/generated-sources.json"
TOOL_DIR="${FLATPAK_BUILDER_TOOLS_DIR:-$REPO_ROOT/.flatpak-builder-tools}"
GENERATOR="$TOOL_DIR/cargo/flatpak-cargo-generator.py"

if [ ! -f "$GENERATOR" ]; then
    echo "Fetching flatpak-builder-tools into $TOOL_DIR"
    git clone --depth 1 https://github.com/flatpak/flatpak-builder-tools.git "$TOOL_DIR"
fi

echo "Generating $OUT from Cargo.lock"
python3 "$GENERATOR" "$REPO_ROOT/Cargo.lock" -o "$OUT"

echo "Done. Commit data/generated-sources.json alongside Cargo.lock so the two stay in step."
