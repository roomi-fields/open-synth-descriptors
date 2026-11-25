#!/usr/bin/env python3
"""Writes the site folder: python tools/build_site.py <folder> [--candidates FILE]

What it holds is in tools/site.list, less the schemas of the versions that the list of candidates
(tools/candidates.list, or the file given) names, and the files their lines name.
"""
import argparse
import html
import shutil
from pathlib import Path

import candidates
import site_list

LIST = "site.list"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("site")
    parser.add_argument("--candidates")
    args = parser.parse_args()
    listed = candidates.read(args.candidates)
    out = Path(args.site)
    if out.exists():
        shutil.rmtree(out)
    folders, home = site_list.read(LIST)
    links = []
    for folder in folders:
        for src in site_list.files(folder):
            rel = src.relative_to(site_list.ROOT).as_posix()
            if candidates.withholds(listed, rel):
                continue
            dest = out / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dest)
            links.append(f'<li><a href="{html.escape(rel)}">{html.escape(rel)}</a></li>')
    page = (site_list.ROOT / home).read_text().replace("<!-- links -->", "\n".join(links))
    (out / Path(home).name).write_text(page)


if __name__ == "__main__":
    main()
