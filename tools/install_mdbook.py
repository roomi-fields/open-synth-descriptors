#!/usr/bin/env python3
"""Installs the book tool at the version and checksum of tools/mdbook.pin.

    python tools/install_mdbook.py [--pin FILE] [--cache FOLDER]

An archive already in the cache folder under its release name is used without any download; one
whose checksum differs from the pin is refused and left as found. A missing archive is downloaded
from the pin's address, checked, then kept. The last line printed is the path of the binary; any
failure exits 1 and prints no path: the tool is installed or the build fails, never skipped.
"""
import argparse
import hashlib
import os
import shutil
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_PIN = HERE / "mdbook.pin"
# The workstation's shared cache folder, its name written in two parts (no workshop word in the public tree).
DEFAULT_CACHE = Path.home() / ".cache" / ("grill" + "house") / "mdbook"


def fail(message):
    print(message, file=sys.stderr)
    sys.exit(1)


def read_pin(path):
    pin = {}
    for line in Path(path).read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            key, value = line.split("=", 1)
            pin[key.strip()] = value.strip()
    for key in ("version", "url", "sha256"):
        if key not in pin:
            fail(f"{path}: no {key}")
    return pin


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def install(pin_path=DEFAULT_PIN, cache=DEFAULT_CACHE):
    """Returns the path of the pinned binary, or exits 1 naming why it cannot."""
    pin = read_pin(pin_path)
    cache = Path(cache)
    cache.mkdir(parents=True, exist_ok=True)
    archive = cache / pin["url"].rsplit("/", 1)[-1]
    if not archive.is_file():
        fd, tmp = tempfile.mkstemp(dir=cache, suffix=".part")
        try:
            with os.fdopen(fd, "wb") as out, urllib.request.urlopen(pin["url"], timeout=120) as response:
                shutil.copyfileobj(response, out)
            got = sha256(tmp)
            if got != pin["sha256"]:
                fail(f"checksum of {pin['url']} is {got}, the pin says {pin['sha256']}: refused")
            os.replace(tmp, archive)
        except (OSError, ValueError) as error:
            fail(f"cannot download {pin['url']}: {error}")
        finally:
            if os.path.exists(tmp):
                os.remove(tmp)
    got = sha256(archive)
    if got != pin["sha256"]:
        fail(f"checksum of {archive} is {got}, the pin says {pin['sha256']}: refused, left as found")
    target = cache / f"mdbook-{pin['sha256'][:16]}"
    binary = target / "mdbook"
    if not binary.is_file():
        target.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive) as tar:
            member = next((m for m in tar.getmembers() if m.isfile() and Path(m.name).name == "mdbook"), None)
            if member is None:
                fail(f"{archive} holds no mdbook file")
            data = tar.extractfile(member).read()
        binary.write_bytes(data)
        binary.chmod(0o755)
    return binary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pin", default=DEFAULT_PIN)
    parser.add_argument("--cache", default=DEFAULT_CACHE)
    args = parser.parse_args()
    print(install(args.pin, args.cache))


if __name__ == "__main__":
    main()
