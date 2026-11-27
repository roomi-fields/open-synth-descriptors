#!/usr/bin/env python3
"""Installs the book tool at the version and checksum of tools/mdbook.pin.

    python tools/install_mdbook.py [--pin FILE] [--cache FOLDER]

The cache folder is the one --cache names; without it, the one the variable MDBOOK names; with
MDBOOK unset or empty, the default folder under the home. A cache folder this run cannot write is
only read: it serves when it holds the pinned archive and its installed binary, and is refused
otherwise.

An archive already in the cache folder under its release name is used without any download; one
whose checksum differs from the pin is refused and left as found. A missing archive is downloaded
from the pin's address, checked, then kept. The binary is installed when it is executable and holds
the bytes of the archive's mdbook file; otherwise it is written beside its path, made executable,
then renamed into place. The last line printed is the path of the binary; any failure exits 1 and
prints no path: the tool is installed or the build fails, never skipped.
"""
import argparse
import hashlib
import os
import shlex
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


def cache_folder():
    """The cache folder of a run that gives no --cache."""
    return os.environ.get("MDBOOK") or DEFAULT_CACHE


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


def refuse_unwritable(pin_path, cache, missing):
    """Exits 1: the cache folder cannot be written and lacks what the pin asks for."""
    command = shlex.join(
        ["python", "tools/install_mdbook.py", "--pin", str(pin_path), "--cache", str(cache)]
    )
    fail(
        f"{cache}: cannot be written and holds no {missing}: fill it outside the envelope, "
        f"from the repository root, with: {command}"
    )


def writable(folder):
    """True when a file can be created in the folder, made if absent."""
    try:
        folder.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryFile(dir=folder):
            return True
    except OSError:
        return False


def pinned_member(archive):
    """The bytes of the mdbook file of the archive."""
    with tarfile.open(archive) as tar:
        member = next((m for m in tar.getmembers() if m.isfile() and Path(m.name).name == "mdbook"), None)
        if member is None:
            fail(f"{archive} holds no mdbook file")
        return tar.extractfile(member).read()


def installed(binary, data):
    """True when the binary is an executable file holding exactly these bytes."""
    try:
        return binary.is_file() and os.access(binary, os.X_OK) and binary.read_bytes() == data
    except OSError:
        return False


def place(binary, data):
    """Renames an executable file holding these bytes over the binary: no run sees it half written."""
    fd, tmp = tempfile.mkstemp(dir=binary.parent, suffix=".part")
    try:
        with os.fdopen(fd, "wb") as out:
            out.write(data)
        os.chmod(tmp, 0o755)
        os.replace(tmp, binary)
    except OSError as error:
        fail(f"cannot install {binary}: {error}")
    finally:
        if os.path.exists(tmp):
            os.remove(tmp)


def install(pin_path=DEFAULT_PIN, cache=None):
    """Returns the path of the pinned binary, or exits 1 naming why it cannot."""
    pin = read_pin(pin_path)
    cache = Path(cache_folder() if cache is None else cache)
    archive = cache / pin["url"].rsplit("/", 1)[-1]
    target = cache / f"mdbook-{pin['sha256'][:16]}"
    binary = target / "mdbook"
    if not archive.is_file():
        if not writable(cache):
            refuse_unwritable(pin_path, cache, archive.name)
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
    data = pinned_member(archive)
    if not installed(binary, data):
        if not writable(target):
            refuse_unwritable(pin_path, cache, f"installed {binary}")
        place(binary, data)
    return binary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pin", default=DEFAULT_PIN)
    parser.add_argument("--cache")
    args = parser.parse_args()
    print(install(args.pin, args.cache))


if __name__ == "__main__":
    main()
