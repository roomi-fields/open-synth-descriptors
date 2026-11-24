#!/usr/bin/env python3
"""Checks the links of the built book.

    python tools/check_book.py <book folder> --site <site folder> [--public-list FILE]

The book sits beside the schemas, at <site>/book/. Each link of each page must resolve to a page of
the book (and its anchor to an id of that page), to a file of the site (a schema), or, as an address of
the public repository on main, to a file of the public tree: a path covered by the public list (without
the option, a file of this repository). Exit 1, naming the page and the target of each failure.
"""
import argparse
import posixpath
import re
import sys
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPO = "https://github.com/roomi-fields/open-synth-descriptors"
SKIPPED = {"print.html", "404.html"}


class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links, self.ids = [], set()

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get("id"):
            self.ids.add(attrs["id"])
        if tag == "a" and attrs.get("name"):
            self.ids.add(attrs["name"])
        if tag == "a" and attrs.get("href") is not None:
            self.links.append(attrs["href"])


def parse(path):
    page = Page()
    page.feed(path.read_text(encoding="utf-8"))
    return page


def read_list(path):
    exact, folders = set(), []
    for line in Path(path).read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.endswith("/"):
            folders.append(line)
        else:
            exact.add(line)
    return exact, folders


def covered(path, exact, folders):
    return path in exact or any((path + "/").startswith(f) for f in folders)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("book")
    parser.add_argument("--site", required=True)
    parser.add_argument("--public-list")
    args = parser.parse_args()
    book, site = Path(args.book), Path(args.site)
    listing = read_list(args.public_list) if args.public_list else None
    pages = {p.relative_to(book).as_posix(): p for p in book.rglob("*.html") if p.name not in SKIPPED}
    parsed = {rel: parse(p) for rel, p in pages.items()}
    failures = []
    for rel, page in sorted(parsed.items()):
        for href in page.links:
            target = href.replace("&amp;", "&")
            problem = judge(rel, target, book, site, parsed, listing)
            if problem:
                failures.append(f"{rel}: {target}: {problem}")
    for failure in failures:
        print(failure)
    if failures:
        sys.exit(1)
    print(f"ok: {sum(len(p.links) for p in parsed.values())} links in {len(parsed)} pages")


def judge(rel, target, book, site, parsed, listing):
    """Returns why the link resolves nowhere, or None."""
    if target.startswith(REPO + "/"):
        m = re.match(r"(?:blob|tree)/main/([^#?]*)", target[len(REPO) + 1 :])
        if not m:
            return None
        path = m.group(1).rstrip("/")
        if listing is not None:
            ok = covered(path, *listing)
        else:
            ok = (ROOT / path).exists()
        return None if ok else f"{path} is not a file of the public tree"
    if re.match(r"[a-z][a-z0-9+.-]*:", target, re.I) or target.startswith("//"):
        return None
    path, _, fragment = target.partition("#")
    path = path.partition("?")[0]
    if not path:
        page = rel
    else:
        here = posixpath.dirname(rel)
        resolved = posixpath.normpath(posixpath.join("book", here, path))
        if resolved.startswith("../"):
            return "leaves the site"
        if resolved.startswith("book/") or resolved == "book":
            page = resolved[len("book/") :] if resolved != "book" else "index.html"
            if path.endswith("/"):
                page = posixpath.join(page, "index.html")
            if not (book / page).is_file():
                return "no page of the book"
        else:
            if not (site / resolved).is_file():
                return "no schema or file of the site"
            return None
    if fragment and page in parsed and fragment not in parsed[page].ids:
        return f"no anchor #{fragment} in {page}"
    return None


if __name__ == "__main__":
    main()
