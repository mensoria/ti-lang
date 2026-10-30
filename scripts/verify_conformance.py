#!/usr/bin/env python3
"""Execute the 44 numbered scenarios and retain both failed and successful runs."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
STAMP = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
OUT = ROOT / '.cache/qualification' / STAMP
OUT.mkdir(parents=True)
report = {'schema': 'ti.qualification/1', 'time_utc': STAMP,
          'claim': '44 numbered regression scenarios, not full implementation or platform certification',
          'platform': platform.platform(), 'rust': subprocess.check_output(['rustc','--version'], text=True).strip(),
          'backend': 'SurrealDB 3.3.0; isolated loopback memory database per native test',
          'source_sha256': {}, 'runs': [], 'cases': []}
paths = sorted(set(ROOT.glob('src/*')) | set(ROOT.glob('tests/*.rs')) | set(ROOT.glob('tests/support/*.rs')) | {ROOT/'Cargo.lock', ROOT/'Cargo.toml'})
for path in paths:
    if path.is_file(): report['source_sha256'][str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
report['fixture'] = json.loads((ROOT/'tests/fixtures/mensoria/manifest.json').read_text())
binary = Path(os.environ.get('TI_SURREAL_BIN', ROOT/'../mensoria/prototypes/surrealdb-rust-ai-baas/.p0/bin/surreal'))
if not binary.is_file():
    raise SystemExit('TI_SURREAL_BIN must point to the pinned 3.3.0 native server; native tests are not skipped.')
report['server_binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
passed, failed, commands_ok = set(), set(), True
for name, command in [
    ('core', ['cargo','test','--locked','--test','language','--test','conformance']),
    ('native', ['cargo','test','--locked','--features','host-qualification','--test','native_host','--','--test-threads=2']),
]:
    run = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (OUT/f'{name}.log').write_text(run.stdout)
    commands_ok &= run.returncode == 0
    report['runs'].append({'suite':name,'command':command,'exit_code':run.returncode,'log_sha256':hashlib.sha256(run.stdout.encode()).hexdigest()})
    for test, status in re.findall(r'^test (\w+) \.\.\. (ok|FAILED)$', run.stdout, re.M):
        (passed if status == 'ok' else failed).add(test)
for number in range(1,45):
    prefix = f'tc_{number:03d}'
    names = sorted(name for name in passed | failed if name == prefix or name.startswith(prefix+'_'))
    status = 'passed' if names and not failed.intersection(names) else 'failed-or-missing'
    report['cases'].append({'id':f'TC-{number:03d}','status':status,'tests':names})
report['passed_cases'] = sum(c['status']=='passed' for c in report['cases'])
report['passed_tests'] = len(passed)
report['status'] = 'passed' if commands_ok and report['passed_cases']==44 else 'failed'
(OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':report['status'],'cases':f"{report['passed_cases']}/44",'tests':len(passed),'report':str(OUT/'report.json')},indent=2))
sys.exit(0 if report['status']=='passed' else 1)
