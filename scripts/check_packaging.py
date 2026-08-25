#!/usr/bin/env python3
"""Cross-file consistency checks for the Flatpak packaging metadata.

The app id appears in six places (manifest, desktop file, metainfo, icon filename,
StartupWMClass, launchable). Flathub rejects submissions where any of them disagree, and
the failure messages are unhelpful, so check them here instead.
"""
import json, os, re, struct, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
APP = "io.github.BlockBreakersHQ.BlockWallet"

passed, failed = [], []


def check(cond, msg):
    (passed if cond else failed).append(msg)


def main():
    data = lambda *p: os.path.join(ROOT, "data", *p)

    # --- release manifest ---
    try:
        m = json.load(open(data(APP + ".json"), encoding="utf-8"))
    except Exception as exc:
        print("  FAIL release manifest is not valid JSON: %s" % exc)
        return 1

    check(m["app-id"] == APP, "release app-id matches")
    module = m["modules"][0]
    top_args = m.get("build-options", {}).get("build-args", [])
    mod_args = module.get("build-options", {}).get("build-args", [])
    check("--share=network" not in top_args + mod_args,
          "no network access during build (Flathub requirement)")
    src = module["sources"][0]
    check(src.get("type") == "git", "first source is a git checkout")
    check("tag" in src, "git source pins a tag")
    check("commit" in src, "git source pins a commit (Flathub wants tag AND commit)")
    check(m["build-options"]["env"]["CARGO_HOME"]
          == "/run/build/%s/cargo" % module["name"],
          "CARGO_HOME matches the module name")
    check(any(s == "generated-sources.json" for s in module["sources"]),
          "manifest includes generated-sources.json")

    # --- desktop entry ---
    desktop = open(data(APP + ".desktop"), encoding="utf-8").read()
    kv = dict(re.findall(r"^([A-Za-z\-\[\]@]+)=(.*)$", desktop, re.M))
    check(desktop.startswith("[Desktop Entry]"), "desktop file has the right header")
    check(kv.get("Icon") == APP, "desktop Icon= is the app id")
    check(kv.get("Exec") == m["command"], "desktop Exec= matches manifest command")
    check(kv.get("StartupWMClass") == APP, "StartupWMClass is the app id")
    check("Mobile" in kv.get("X-Purism-FormFactor", ""),
          "declares the Mobile form factor for the Librem 5")

    # --- metainfo ---
    mi = open(data(APP + ".metainfo.xml"), encoding="utf-8").read()
    check("<id>%s</id>" % APP in mi, "metainfo id matches")
    check('<launchable type="desktop-id">%s.desktop</launchable>' % APP in mi,
          "metainfo launchable points at the desktop file")
    check('type="oars-1.1"' in mi, "metainfo carries an OARS rating")
    check("<metadata_license>" in mi and "<project_license>" in mi,
          "metainfo declares both licences")
    check("<screenshots>" in mi, "metainfo declares screenshots")
    check("<releases>" in mi, "metainfo declares releases")

    # --- icon ---
    icon = data("icons", "hicolor", "256x256", "apps", APP + ".png")
    if os.path.exists(icon):
        blob = open(icon, "rb").read()
        check(blob[:8] == b"\x89PNG\r\n\x1a\n", "icon is a PNG")
        width, height = struct.unpack(">II", blob[16:24])
        check((width, height) == (256, 256),
              "icon is 256x256, matching its hicolor directory")
    else:
        check(False, "icon exists at the hicolor path")

    # --- assets the build-commands copy ---
    check(os.path.isdir(os.path.join(ROOT, "Images")), "Images/ exists")
    check(os.path.exists(os.path.join(ROOT, "Images", "Logo.png")), "Images/Logo.png exists")

    for msg in passed:
        print("  ok   " + msg)
    for msg in failed:
        print("  FAIL " + msg)
    print("  %d passed, %d failed" % (len(passed), len(failed)))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
