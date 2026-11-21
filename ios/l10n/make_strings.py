#!/usr/bin/env python3
"""The app's translations: l10n/<language>.json (English text -> translation,
the English texts being the keys SwiftUI looks up) into
App/<language>.lproj/Localizable.strings.

  python3 l10n/make_strings.py          (in ios/)

Checks that every language has every key of en.json and the same %@ / %lld
in the same order; a missing or bad one stays English (and is listed).
"""
import json
import pathlib
import re
import sys

here = pathlib.Path(__file__).resolve().parent
app = here.parent / "App"
keys = json.loads((here / "en.json").read_text(encoding="utf-8"))
spec = re.compile(r"%(?:@|lld)")


def esc(s):
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")


bad = 0
for f in sorted(here.glob("*.json")):
    if f.stem == "en":
        continue
    tr = json.loads(f.read_text(encoding="utf-8"))
    lines = [f"/* Mandelbulb3D for iPad: {f.stem} (made by l10n/make_strings.py from l10n/{f.name}) */", ""]
    for k in keys:
        v = tr.get(k)
        if not v or spec.findall(v) != spec.findall(k):
            print(f"{f.stem}: {'missing' if not v else 'format differs'}: {k}", file=sys.stderr)
            bad += 1
            continue
        lines.append(f'"{esc(k)}" = "{esc(v)}";')
    d = app / f"{f.stem}.lproj"
    d.mkdir(exist_ok=True)
    (d / "Localizable.strings").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{f.stem}: {len(lines) - 2} of {len(keys)}")
sys.exit(1 if bad else 0)
