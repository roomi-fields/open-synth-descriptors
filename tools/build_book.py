#!/usr/bin/env python3
"""Writes the book of the specification: python tools/build_book.py <book folder> [--pin FILE] [--cache FOLDER]

The source of the book is assembled from the texts of spec/ (their words unchanged) and the pages of
`osd-reference build schemas/<newest version>`; the book tool is the one tools/mdbook.pin names,
installed by tools/install_mdbook.py. The book is written into its own folder, never into the site
folder. Meant to sit beside the schemas at <site>/book/: a link to a schema file is rendered as the
path of that file in the site, a link to a repository file the book does not hold as its address in
the public repository on main. Without the tool the build fails and writes nothing.
"""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath

import install_mdbook

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
REPO = "https://github.com/roomi-fields/open-synth-descriptors"
LINK = re.compile(r"\]\(([^)\s]+)((?:\s+\"[^\"]*\")?)\)")


def fail(message):
    print(message, file=sys.stderr)
    sys.exit(1)


def newest_schemas():
    versions = [p for p in (ROOT / "schemas").iterdir() if p.is_dir() and re.fullmatch(r"\d+(\.\d+)*", p.name)]
    if not versions:
        fail("schemas/: no version folder")
    return max(versions, key=lambda p: tuple(int(n) for n in p.name.split(".")))


def spec_pages():
    """The texts of spec/, as paths under spec/, the specification of the newest version first."""
    spec = ROOT / "spec"
    pages = sorted(p.relative_to(spec).as_posix() for p in spec.rglob("*.md"))
    return sorted(pages, key=lambda r: (re.fullmatch(r"\d+(\.\d+)*\.md", r) is None, r))


def render_links(text, source):
    """Rewrites the destination of each relative link that leaves spec/; the words stay."""
    depth = len(PurePosixPath(source).parts) - 1
    here = PurePosixPath("spec") / PurePosixPath(source).parent

    def one(match):
        target, title = match.group(1), match.group(2)
        if re.match(r"[a-z][a-z0-9+.-]*:|#", target):
            return match.group(0)
        path, _, fragment = target.partition("#")
        repo_path = PurePosixPath(os.path.normpath(here / path))
        if repo_path.parts[:1] == ("spec",):
            return match.group(0)
        tail = "#" + fragment if fragment else ""
        if repo_path.parts[:1] == ("schemas",) and repo_path.suffix == ".json":
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
    parser.add_argument("--cache", default=install_mdbook.DEFAULT_CACHE)
    args = parser.parse_args()
    mdbook = install_mdbook.install(args.pin, args.cache)
    schemas = newest_schemas()
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
        for rel in spec_pages():
            text = (ROOT / "spec" / rel).read_text()
            dest = src / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text(render_links(text, rel))
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
