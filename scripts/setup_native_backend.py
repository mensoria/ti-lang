#!/usr/bin/env python3
"""Fetch a checksum-pinned native test backend; never starts a shared database."""
import hashlib
from pathlib import Path
import platform
import tarfile
import urllib.request

ASSETS = {
 ('Darwin','arm64'):('darwin-arm64','75e37adf5f9aacfa1308df71193b2d0e0727bc6532105541c083e71271cd7fc9'),
 ('Linux','x86_64'):('linux-amd64','44aeab565f7e7e39d2d0bf0583c8aae648babc91373c70b2658288d95bbbcd55'),
}
name,digest=ASSETS[(platform.system(),platform.machine())]
root=Path(__file__).resolve().parents[1]/'.cache/native-backend'
root.mkdir(parents=True,exist_ok=True)
archive=root/f'surreal-v3.3.0.{name}.tgz'
if not archive.is_file() or hashlib.sha256(archive.read_bytes()).hexdigest()!=digest:
    data=urllib.request.urlopen(f'https://github.com/surrealdb/surrealdb/releases/download/v3.3.0/{archive.name}',timeout=60).read()
    if hashlib.sha256(data).hexdigest()!=digest:raise SystemExit('Backend checksum mismatch')
    archive.write_bytes(data)
with tarfile.open(archive) as tar:tar.extractall(root,filter='data')
print(root/'surreal')
