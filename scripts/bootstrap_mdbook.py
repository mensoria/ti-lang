#!/usr/bin/env python3
"""Download a pinned official mdBook release into an ignored, local cache."""
from pathlib import Path
import hashlib
import io
import platform
import tarfile
import urllib.request
import urllib.error

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.5.4"
ASSETS = {
    ("Darwin", "arm64"): ("aarch64-apple-darwin", "03e8a6d8b13a2971e0b3280affd03b388373c1485e26f73407c3a76b0b1838df"),
    ("Darwin", "x86_64"): ("x86_64-apple-darwin", "a47d7bf0d5d670cff9ee6cce95537cbeb62dc10704d9e7131ffbd13e2b59a5de"),
    ("Linux", "x86_64"): ("x86_64-unknown-linux-musl", "5222beabd3e37dc5be0d18ff99b79058469354db5c220153a1b92db5ba12be89"),
    ("Linux", "aarch64"): ("aarch64-unknown-linux-musl", "753e5c5c363ee8a56972344dcf91466f005a51db84a7aeffe427ae3ef83d6d44"),
}

def main():
    key = (platform.system(), platform.machine())
    if key not in ASSETS:
        raise SystemExit(f"Unsupported bootstrap platform {key}; install mdbook {VERSION} manually.")
    target, expected = ASSETS[key]
    name = f"mdbook-v{VERSION}-{target}.tar.gz"
    url = f"https://github.com/rust-lang/mdBook/releases/download/v{VERSION}/{name}"
    cache = ROOT / ".cache/tools"
    cache.mkdir(parents=True, exist_ok=True)
    archive_path = cache / name
    data = archive_path.read_bytes() if archive_path.is_file() else None
    if data is None or hashlib.sha256(data).hexdigest() != expected:
        data = None
        for attempt in range(3):
            try:
                request = urllib.request.Request(url)
                with urllib.request.urlopen(request, timeout=30) as response:
                    candidate = response.read()
                actual = hashlib.sha256(candidate).hexdigest()
                if actual != expected:
                    raise SystemExit(f"Archive SHA-256 mismatch for {name}: {actual}")
                data = candidate
                break
            except (urllib.error.URLError, TimeoutError, ConnectionError) as error:
                print(f"Official download attempt {attempt + 1}/3 failed: {error}")
        if data is None:
            raise SystemExit("Official archive download failed; existing tool was preserved. Retry later.")
        pending_archive = archive_path.with_suffix(".pending")
        pending_archive.write_bytes(data)
        pending_archive.replace(archive_path)
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        members = [m for m in archive.getmembers() if Path(m.name).name == "mdbook" and m.isfile()]
        if len(members) != 1:
            raise SystemExit("Official archive did not contain one mdbook executable.")
        content = archive.extractfile(members[0]).read()
    destination = ROOT / ".cache/tools/mdbook"
    destination.parent.mkdir(parents=True, exist_ok=True)
    pending = destination.with_suffix(".download")
    pending.write_bytes(content)
    pending.chmod(0o755)
    pending.replace(destination)
    print(f"Installed verified mdbook {VERSION}: {destination}")

if __name__ == "__main__":
    main()
