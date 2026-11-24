#!/usr/bin/env python3
"""Writes the site folder: python tools/build_site.py <folder>. What it holds is in tools/site.list."""
import html
import shutil
import sys
from pathlib import Path

import site_list

LIST = "site.list"


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: build_site.py <site folder>")
    out = Path(sys.argv[1])
    if out.exists():
        shutil.rmtree(out)
    folders, home = site_list.read(LIST)
    links = []
    for folder in folders:
        for src in site_list.files(folder):
            rel = src.relative_to(site_list.ROOT).as_posix()
            dest = out / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dest)
            links.append(f'<li><a href="{html.escape(rel)}">{html.escape(rel)}</a></li>')
    page = (site_list.ROOT / home).read_text().replace("<!-- links -->", "\n".join(links))
    (out / Path(home).name).write_text(page)


if __name__ == "__main__":
    main()
