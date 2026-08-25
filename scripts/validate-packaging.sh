#!/usr/bin/env bash
# Pre-submission checks for the Flatpak / Flathub packaging metadata.
#
# Run this before opening a Flathub PR, and again after adding screenshots and tagging.
# Needs: appstreamcli, python3. Optional: desktop-file-validate, flatpak-builder.
#
#   ./scripts/validate-packaging.sh
set -uo pipefail
cd "$(dirname "$0")/.."

APP="io.github.BlockBreakersHQ.BlockWallet"
fail=0

echo "== AppStream =="
if command -v appstreamcli >/dev/null 2>&1; then
    appstreamcli validate --explain --strict "data/$APP.metainfo.xml" || fail=1
else
    echo "  SKIP: appstreamcli not installed (apt install appstream)"
fi

echo
echo "== Desktop entry =="
if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "data/$APP.desktop" && echo "  OK" || fail=1
else
    echo "  SKIP: desktop-file-validate not installed (apt install desktop-file-utils)"
fi

echo
echo "== Flathub linter (authoritative - this is what Flathub CI runs) =="
if flatpak info org.flatpak.Builder >/dev/null 2>&1; then
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder         manifest "data/$APP.json" || fail=1
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder         appstream "data/$APP.metainfo.xml" || fail=1
else
    echo "  SKIP: flatpak install --user flathub org.flatpak.Builder"
fi

echo
echo "== Cross-file consistency =="
python3 scripts/check_packaging.py || fail=1

echo
echo "== Vendored cargo sources vs Cargo.lock =="
python3 scripts/check_vendored_sources.py || fail=1

echo
if [ "$fail" -ne 0 ]; then
    echo "FAILED - fix the above before submitting."
else
    echo "All packaging checks passed."
fi
exit "$fail"
