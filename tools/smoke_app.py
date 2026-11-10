#!/usr/bin/env python3
"""Verify a packaged app renders its dashboard and graph, then exits cleanly."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[1]
app = root / 'build/desktop/HuntLog' / ('HuntLog.exe' if sys.platform == 'win32' else 'HuntLog')
with tempfile.TemporaryDirectory() as folder:
    folder = Path(folder)
    journal = folder / 'sample.jsonl'
    rows = [dict(event='start', t=0),
        dict(event='sample', t=0, players=[dict(id=1, name='Example hunter', weapon_type=3)], monsters=[dict(id=1, name='Zinogre', hp=100, max_hp=100, charge=dict(active=False, points=0, maximum=100, units='charge_points'))]),
        dict(event='hit', t=2, player=1, monster=1, damage=15),
        dict(event='status', t=3, monster=1, kind='zinogre_charge', phase='start', count=1),
        dict(event='end', t=10)]
    journal.write_text(''.join(json.dumps(dict(schema=1, run=1, **row))+'\n' for row in rows))
    report = folder / 'result.json'
    env = dict(os.environ, XDG_CONFIG_HOME=str(folder / 'config'), XDG_DATA_HOME=str(folder / 'data'), XDG_CACHE_HOME=str(folder / 'cache'))
    subprocess.run([str(app), '--journal', str(journal), '--smoke-report', str(report)], env=env, check=True, timeout=45)
    result = json.loads(report.read_text())
    assert result['ready'] and result['graph'] and result['runs'] == 1, result
    print('Packaged native dashboard passed:', result)
