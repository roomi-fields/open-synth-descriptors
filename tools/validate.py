#!/usr/bin/env python3
"""Check descriptors against the OSD schemas.

    python tools/validate.py <device|profile|sound|firmware|fingerprint> FILE_OR_DIR... [--candidates FILE]

Checked against the newest version in schemas/ that is not a candidate: the list of candidates
(tools/candidates.list, or the file given) names those. OSD_VERSION=0.1 picks another version,
a candidate included.

A directory is read for every *.json in it; files whose name starts with _
are maker profiles (checked as profiles) or other notes, never devices.
A file inside a conformance case whose case.json lists it in `schema_refused`
({file, pointer}) is expected refused, with one error at that pointer; any other
file is expected valid.
Needs the jsonschema package. Exit status 1 when a file is not valid.
"""
import argparse
import json
import os
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

import candidates

SCHEMAS = Path(__file__).resolve().parent.parent / "schemas"


def validators(version):
    schemas = {p.name.split(".")[0]: json.loads(p.read_text(encoding="utf-8")) for p in (SCHEMAS / version).glob("*.schema.json")}
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


def pointer_of(error):
    return "".join("/" + str(k).replace("~", "~0").replace("/", "~1") for k in error.path)


def expected_refusal(p):
    """The pointer the case.json of the case holding p says the schema refuses p at, or None.

    A case folder holds `case.json` and `files/<maker>/<name>.json`; its `schema_refused`
    is a list of {file, pointer}, each file by its catalogue id (`acme/synth`).
    """
    for folder in p.resolve().parents:
        case = folder / "case.json"
        if case.is_file() and (folder / "files") in p.resolve().parents:
            listed = json.loads(case.read_text(encoding="utf-8")).get("schema_refused", [])
            fid = p.resolve().relative_to(folder / "files").with_suffix("").as_posix()
            return next((r["pointer"] for r in listed if r["file"] == fid), None)
    return None


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("kind", nargs="?")
    parser.add_argument("paths", nargs="*")
    parser.add_argument("--candidates")
    args = parser.parse_args(argv)
    if not args.paths:
        sys.exit(__doc__)
    version = os.environ.get("OSD_VERSION") or candidates.newest(SCHEMAS, candidates.read(args.candidates))
    if version is None:
        sys.exit("schemas/: no version folder outside the candidates")
    kind, check = args.kind, validators(version)[args.kind]
    n = bad = refused = 0
    for p in files(args.paths, kind):
        n += 1
        errors = sorted(check.iter_errors(json.loads(p.read_text(encoding="utf-8"))), key=lambda e: list(e.path))
        said = expected_refusal(p)
        if said is None:
            if errors:
                bad += 1
                for e in errors[:5]:
                    print(f"{p}: {'/'.join(map(str, e.path)) or '(top)'}: {e.message[:200]}")
        elif not errors:
            bad += 1
            print(f"{p}: expected refused at {said}, but valid")
        elif len(errors) > 1 or pointer_of(errors[0]) != said:
            bad += 1
            print(f"{p}: expected refused at {said} with one error, got {len(errors)}:")
            for e in errors[:5]:
                print(f"{p}: {pointer_of(e) or '(top)'}: {e.message[:200]}")
        else:
            refused += 1
    print(f"{n - bad - refused}/{n} valid {kind} descriptors, {refused} refused as expected")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
