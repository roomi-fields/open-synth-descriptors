#!/usr/bin/env python3
"""Checks the public site against the publication list (tools/site.list).

    python tools/check_site.py --site <built folder> [--candidates FILE]
    python tools/check_site.py <base address> [--candidates FILE]
                                  (a local server, the Pages address, the w3id address)

A published schema is one of tools/site.list that no candidate of the list of candidates
(tools/candidates.list, or the file given) withholds. Every `$id` of a published schema must answer at its path (the w3id rule maps
https://w3id.org/osd/<version>/<name> to <Pages root>/schemas/<version>/<name>), the home page
must answer, and the workshop's files must answer 404. Exit 1, naming each failure.
"""
import argparse
import sys
import urllib.error
import urllib.request
from pathlib import Path

import candidates
import site_list

LIST = "site.list"
# Workshop files, written in two parts so that no file of the public tree holds a workshop word.
MUST_BE_ABSENT = ["CLAUDE" + ".md", ".bea" + "ds/interactions.jsonl"]


def status(url):
    try:
        with urllib.request.urlopen(url, timeout=30) as response:
            return response.status
    except urllib.error.HTTPError as error:
        return error.code


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("base", nargs="?")
    parser.add_argument("--site")
    parser.add_argument("--candidates")
    args = parser.parse_args()
    if (args.base is None) == (args.site is None):
        raise SystemExit(__doc__)
    folders, home = site_list.read(LIST)
    home_name = Path(home).name
    listed = candidates.read(args.candidates)
    ids = [(id_, path) for id_, path in site_list.ids(folders) if not candidates.withholds(listed, path)]
    failures = []
    if args.site is not None:
        site = Path(args.site)
        if not (site / home_name).is_file():
            failures.append(f"home page {home_name} is missing")
        for id_, path in ids:
            if not (site / path).is_file():
                failures.append(f"{id_}: {path} is not a file of the site folder")
        for path in MUST_BE_ABSENT:
            if (site / path).exists():
                failures.append(f"{path} is in the site folder")
    else:
        base = args.base if args.base.endswith("/") else args.base + "/"
        # Behind the w3id rule the schemas answer at <host>/osd/<version>/<name>.
        w3id = "w3id.org" in base
        urls = [base if w3id else base + home_name]
        for _, path in ids:
            urls.append(base + (path.removeprefix("schemas/") if w3id else path))
        for url in urls:
            code = status(url)
            if code != 200:
                failures.append(f"{url}: expected 200, got {code}")
        for path in MUST_BE_ABSENT:
            code = status(base + path)
            if code != 404:
                failures.append(f"{base + path}: expected 404, got {code}")
    for failure in failures:
        print(failure)
    if failures:
        sys.exit(1)
    print(f"ok: {len(ids)} schema addresses and the home page")


if __name__ == "__main__":
    main()
