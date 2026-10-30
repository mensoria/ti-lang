#!/usr/bin/env python3
"""Run implemented project regressions; never count unimplemented PC rows as passing."""
from pathlib import Path
import datetime
import hashlib
import json
import re
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[1]
stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
out=ROOT/'.cache/project-qualification'/stamp
out.mkdir(parents=True)
command=['cargo','test','--locked','--test','projects']
run=subprocess.run(command,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
(out/'projects.log').write_text(run.stdout)
tests=re.findall(r'^test (\w+) \.\.\. (ok|FAILED)$',run.stdout,re.M)
cases={f'PC-{i:03}':[] for i in range(1,31)}
for test,status in tests:
    if test.startswith('pc_'):
        for n in test.split('_')[1:]:
            if not n.isdigit():break
            cases[f'PC-{int(n):03}'].append({'test':test,'status':status})
report={'schema':'ti.project-qualification/1','time_utc':stamp,'edition':'ti/0.2',
        'status':'passed' if run.returncode==0 else 'failed','tests':len(tests),
        'claim':'Implemented project regression checks, not full PC acceptance or 0.2 host qualification',
        'command':command,'exit_code':run.returncode,'source_sha256':{},
        'cases':[{'id':case,'status':('regression-covered' if all(c['status']=='ok' for c in checks) else 'failed') if checks else 'not-executed','checks':checks} for case,checks in cases.items()]}
for p in sorted(set(ROOT.glob('crates/*/src/*'))|set(ROOT.glob('crates/*/Cargo.toml'))|{ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'tests/projects.rs'}):
    if p.is_file():report['source_sha256'][str(p.relative_to(ROOT))]=hashlib.sha256(p.read_bytes()).hexdigest()
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':report['status'],'tests':len(tests),'report':str(out/'report.json')},indent=2))
sys.exit(run.returncode)
