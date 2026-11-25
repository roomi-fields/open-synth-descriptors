"""Reads the list of the candidate versions (tools/candidates.list), the one place that says them."""
import re
from pathlib import Path

DEFAULT_LIST = Path(__file__).resolve().parent / "candidates.list"


def read(path=None):
    """{version: [paths it brings outside its folder]} from the list given, else the repository's."""
    out = {}
    for line in Path(path or DEFAULT_LIST).read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            version, *paths = line.split()
            out[version] = paths
    return out


def newest(schemas, listed):
    """The newest version folder of `schemas` that `listed` does not name, by number; None without one."""
    versions = [
        p.name for p in schemas.iterdir()
        if p.is_dir() and re.fullmatch(r"\d+(\.\d+)*", p.name) and p.name not in listed
    ]
    return max(versions, key=lambda v: tuple(map(int, v.split("."))), default=None)


def withholds(listed, path):
    """Whether `path`, a path of the repository, is a schema file of a candidate of `listed`."""
    return any(path.startswith(f"schemas/{version}/") or path in paths for version, paths in listed.items())
