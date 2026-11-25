#!/usr/bin/env python3
"""Writes the book of the specification:
python tools/build_book.py <book folder> [--pin FILE] [--cache FOLDER] [--candidates FILE]

The source of the book is assembled from the texts of spec/ (their words unchanged) and the pages of
`osd-reference build schemas/<newest version>`. A version that the list of candidates
(tools/candidates.list, or the file given) names has no page spec/<version>.md in the book, and the
newest version is the newest the list does not name. The book tool is the one tools/mdbook.pin names,
installed by tools/install_mdbook.py. The book is written into its own folder, never into the site
folder. Meant to sit beside the schemas at <site>/book/: a link to a schema file is rendered as the
path of that file in the site, a link to a repository file the book does not hold, or to a page or a
schema withheld for a candidate, as its address in the public repository on main. Without the tool the build fails and writes nothing.
"""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath

import candidates
import install_mdbook

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
REPO = "https://github.com/roomi-fields/open-synth-descriptors"
LINK = re.compile(r"\]\(([^)\s]+)((?:\s+\"[^\"]*\")?)\)")


def fail(message):
    print(message, file=sys.stderr)
    sys.exit(1)


def spec_pages(listed):
    """The texts of spec/ the book holds, as paths under spec/, the specifications of the versions first."""
    spec = ROOT / "spec"
    withheld = {f"{version}.md" for version in listed}
    pages = sorted(p.relative_to(spec).as_posix() for p in spec.rglob("*.md"))
    pages = [p for p in pages if p not in withheld]
    return sorted(pages, key=lambda r: (re.fullmatch(r"\d+(\.\d+)*\.md", r) is None, r))


def render_links(text, source, listed):
    """Rewrites the destination of each relative link that leaves the book; the words stay."""
    depth = len(PurePosixPath(source).parts) - 1
    here = PurePosixPath("spec") / PurePosixPath(source).parent

    def one(match):
        target, title = match.group(1), match.group(2)
        if re.match(r"[a-z][a-z0-9+.-]*:|#", target):
            return match.group(0)
        path, _, fragment = target.partition("#")
        repo_path = PurePosixPath(os.path.normpath(here / path))
        withheld_page = str(repo_path) in {f"spec/{version}.md" for version in listed}
        if repo_path.parts[:1] == ("spec",) and not withheld_page:
            return match.group(0)
        tail = "#" + fragment if fragment else ""
        in_site = not candidates.withholds(listed, str(repo_path))
        if repo_path.parts[:1] == ("schemas",) and repo_path.suffix == ".json" and in_site:
            return f"]({'../' * (depth + 1)}{repo_path}{tail}{title})"
        kind = "tree" if (ROOT / repo_path).is_dir() else "blob"
        return f"]({REPO}/{kind}/main/{repo_path}{tail}{title})"

    out, in_code = [], False
    for line in text.split("\n"):
        if line.startswith("```"):
            in_code = not in_code
        out.append(line if in_code else LINK.sub(one, line))
    return "\n".join(out)


def title_of(text, fallback):
    for line in text.splitlines():
        if line.startswith("# "):
            return line[2:].strip().replace("`", "")
    return fallback


def reference_binary():
    given = os.environ.get("OSD_REFERENCE")
    return [given] if given else ["cargo", "run", "--quiet", "-p", "reference", "--"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("book")
    parser.add_argument("--pin", default=install_mdbook.DEFAULT_PIN)
    parser.add_argument("--cache")
    parser.add_argument("--candidates")
    args = parser.parse_args()
    mdbook = install_mdbook.install(args.pin, args.cache)
    listed = candidates.read(args.candidates)
    version = candidates.newest(ROOT / "schemas", listed)
    if version is None:
        fail("schemas/: no version folder outside the candidates")
    schemas = ROOT / "schemas" / version
    with tempfile.TemporaryDirectory() as work:
        work = Path(work)
        src = work / "src"
        src.mkdir()
        summary = ["# Summary", "", "[Open Synth Descriptors](intro.md)", "", "# Specification", ""]
        (src / "intro.md").write_text(
            "# Open Synth Descriptors\n\nOpen Synth Descriptors (OSD) is an open format that describes "
            "synthesizers, drum machines, controllers and music software beyond what MIDI and OSC say. "
            "This book holds the text of the specification and, for each kind of descriptor, the reference "
            "of its schema.\n"
        )
        for rel in spec_pages(listed):
            text = (ROOT / "spec" / rel).read_text()
            dest = src / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text(render_links(text, rel, listed))
            summary.append(f"- [{title_of(text, rel)}]({rel})")
        reference = work / "reference-pages"
        run = subprocess.run(
            reference_binary() + ["build", str(schemas), str(reference)], cwd=ROOT, capture_output=True, text=True
        )
        if run.returncode != 0:
            fail(f"osd-reference build {schemas} failed:\n{run.stdout}{run.stderr}")
        summary += ["", f"# Reference {schemas.name}", ""]
        (src / "reference").mkdir()
        for page in sorted(reference.glob("*.md")):
            text = page.read_text()
            shutil.copyfile(page, src / "reference" / page.name)
            summary.append(f"- [{title_of(text, page.stem)}](reference/{page.name})")
        (src / "SUMMARY.md").write_text("\n".join(summary) + "\n")
        shutil.copyfile(HERE / "book.toml", work / "book.toml")
        out = Path(args.book).resolve()
        run = subprocess.run([str(mdbook), "build", str(work), "-d", str(out)], capture_output=True, text=True)
        if run.returncode != 0:
            fail(f"mdbook build failed:\n{run.stdout}{run.stderr}")


if __name__ == "__main__":
    main()
