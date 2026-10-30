#!/usr/bin/env python3
"""Check actual Cargo ownership edges and the default consumer dependency closure."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version','1'],cwd=ROOT,text=True))
allowed = {
    'ti-lang': {'ti-language','ti-artifact','ti-cli','ti-host-mensoria','ti-project'},
    'ti-language': set(),
    'ti-artifact': {'ti-language'},
    'ti-cli': {'ti-language','ti-artifact','ti-project'},
    'ti-project': {'ti-language'},
    'ti-host-mensoria': {'ti-language','ti-artifact'},
}
packages = {p['name']:p for p in metadata['packages'] if p['id'] in metadata['workspace_members']}
assert set(packages)==set(allowed),set(packages)
for name,package in packages.items():
    actual={d['name'] for d in package['dependencies'] if d['name'] in packages}
    assert actual==allowed[name],(name,actual)
    if name in {'ti-language','ti-artifact','ti-cli','ti-project'}:
        assert not {'surrealdb','tokio'}.intersection(d['name'] for d in package['dependencies']),name
binaries=[(p['name'],t['name']) for p in packages.values() for t in p['targets'] if 'bin' in t['kind']]
assert binaries==[('ti-lang','ti')],binaries
closure=subprocess.check_output(['cargo','tree','--locked','-p','ti-lang','--no-default-features','--prefix','none'],cwd=ROOT,text=True)
assert not any(line.startswith(('surrealdb ','tokio ','ti-host-mensoria ')) for line in closure.splitlines())
print('Workspace: six packages, declared acyclic ownership edges, one ti binary, no native host in the default consumer closure.')
