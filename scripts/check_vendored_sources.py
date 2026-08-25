#!/usr/bin/env python3
"""Check data/generated-sources.json really matches Cargo.lock.

A mismatch here does not fail until deep into an offline Flatpak build, where the error is
opaque, so it is worth checking cheaply up front. Compares by exact URL rather than by
parsing crate filenames: versions carrying semver build metadata (e.g. "1.1.4+spec-1.1.0")
contain a second "-...", which naive parsing splits in the wrong place.
"""
import json, os, re, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
GEN = os.path.join(ROOT, "data", "generated-sources.json")


def locked_crates(lock_text):
    out = {}
    for blk in lock_text.split("[[package]]")[1:]:
        name = re.search(r'^name\s*=\s*"([^"]+)"', blk, re.M)
        ver = re.search(r'^version\s*=\s*"([^"]+)"', blk, re.M)
        src = re.search(r'^source\s*=\s*"([^"]+)"', blk, re.M)
        sha = re.search(r'^checksum\s*=\s*"([^"]+)"', blk, re.M)
        if name and ver and src and "registry+" in src.group(1) and sha:
            out[(name.group(1), ver.group(1))] = sha.group(1)
    return out


def main():
    lock = open(os.path.join(ROOT, "Cargo.lock"), encoding="utf-8").read()
    gen = json.load(open(GEN, encoding="utf-8"))
    locked = locked_crates(lock)

    archives = {e["url"]: e for e in gen if e.get("type") == "archive"}
    sums = {e["dest"]: e for e in gen
            if e.get("type") == "inline" and e.get("dest-filename") == ".cargo-checksum.json"}
    config = [e for e in gen
              if e.get("type") == "inline" and e.get("dest-filename") == "config"]

    errs = []
    expected = set()
    for (name, ver), sha in sorted(locked.items()):
        url = "https://static.crates.io/crates/{0}/{0}-{1}.crate".format(name, ver)
        expected.add(url)
        entry = archives.get(url)
        if entry is None:
            errs.append("missing archive: %s-%s" % (name, ver))
            continue
        if entry.get("sha256") != sha:
            errs.append("sha256 mismatch: %s-%s" % (name, ver))
        dest = "cargo/vendor/%s-%s" % (name, ver)
        if entry.get("dest") != dest:
            errs.append("wrong dest for %s-%s: %s" % (name, ver, entry.get("dest")))
        got = sums.get(dest)
        if got is None:
            errs.append("missing .cargo-checksum.json: %s-%s" % (name, ver))
        elif json.loads(got["contents"]).get("package") != sha:
            errs.append("checksum file disagrees: %s-%s" % (name, ver))

    for url in sorted(set(archives) - expected):
        errs.append("orphan archive not in Cargo.lock: %s" % url)

    if len(config) != 1:
        errs.append("expected exactly one cargo config inline, found %d" % len(config))

    print("  %d registry crates in Cargo.lock, %d archive entries" % (len(locked), len(archives)))
    for e in errs[:20]:
        print("  FAIL " + e)
    if len(errs) > 20:
        print("  ... and %d more" % (len(errs) - 20))
    if errs:
        print("  -> regenerate with scripts/flatpak-gen-sources.sh")
        return 1
    print("  OK: every locked crate vendored with a matching checksum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
