"""Reads the publication list (tools/site.list), the one place that says what the site holds."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def read(list_name):
    """Returns (folders, home page source) from the list file next to this module."""
    folders, home = [], None
    for line in (Path(__file__).resolve().parent / list_name).read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.endswith("/"):
            folders.append(line.rstrip("/"))
        else:
            home = line
    if home is None or not folders:
        raise SystemExit(f"{list_name}: needs a folder and a home page source")
    return folders, home


def ids(folders):
    """Every `$id` of the published schemas, as (id, site path) under the w3id rule."""
    out = []
    for folder in folders:
        for path in sorted((ROOT / folder).rglob("*.json")):
            doc = json.loads(path.read_text())
            if isinstance(doc, dict) and isinstance(doc.get("$id"), str):
                tail = doc["$id"].split("/osd/", 1)[1]
                out.append((doc["$id"], f"{folder}/{tail}"))
    return out


def files(folder):
    return sorted(p for p in (ROOT / folder).rglob("*") if p.is_file())
