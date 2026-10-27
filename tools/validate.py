#!/usr/bin/env python3
"""Check descriptors against the OSD schemas.

    python tools/validate.py <device|profile|sound|firmware|fingerprint> FILE_OR_DIR...

Checked against the newest version in schemas/ (OSD_VERSION=0.1 picks another).

A directory is read for every *.json in it; files whose name starts with _
are maker profiles (checked as profiles) or other notes, never devices.
Needs the jsonschema package. Exit status 1 when a file is not valid.
"""
import json
import os
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

SCHEMAS = Path(__file__).resolve().parent.parent / "schemas"
VERSION = os.environ.get("OSD_VERSION") or max((p.name for p in SCHEMAS.iterdir()), key=lambda v: tuple(map(int, v.split("."))))
HERE = SCHEMAS / VERSION


def validators():
    schemas = {p.name.split(".")[0]: json.loads(p.read_text(encoding="utf-8")) for p in HERE.glob("*.schema.json")}
    registry = Registry().with_resources((s["$id"], Resource.from_contents(s)) for s in schemas.values())
    return {name: Draft202012Validator(s, registry=registry) for name, s in schemas.items()}


def files(args, kind):
    for a in map(Path, args):
        if a.is_dir():
            for p in sorted(a.rglob("*.json")):
                profile = p.name.startswith("_profile")
                if (kind == "profile") == profile and (profile or not p.name.startswith("_")):
                    yield p
        else:
            yield a


def main(argv):
    if len(argv) < 2:
        sys.exit(__doc__)
    kind, check = argv[0], validators()[argv[0]]
    n = bad = 0
    for p in files(argv[1:], kind):
        n += 1
        errors = sorted(check.iter_errors(json.loads(p.read_text(encoding="utf-8"))), key=lambda e: list(e.path))
        if errors:
            bad += 1
            for e in errors[:5]:
                print(f"{p}: {'/'.join(map(str, e.path)) or '(top)'}: {e.message[:200]}")
    print(f"{n - bad}/{n} valid {kind} descriptors")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
