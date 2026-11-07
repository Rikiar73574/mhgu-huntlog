#!/usr/bin/env python3
"""Local, read-only hunt-history web page. Run with --port 8768."""
import argparse
import json
from pathlib import Path
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from hunt_history import read_journal
from player_names import enrich_player
from loot_details import enrich_loot

from history_paths import HISTORY as ROOT, default_journal
QUESTS = json.loads((ROOT / 'quests.json').read_text(encoding="utf-8"))
ITEMS = json.loads((ROOT / "items.json").read_text(encoding="utf-8"))
DEFAULT = default_journal()

def handler(journal):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            route = self.path.split('?', 1)[0]
            if route == '/api/hunts':
                try:
                    with (journal() if callable(journal) else journal).open(encoding='utf-8', errors='replace') as f:
                        report = read_journal(f)
                    for run in report["runs"]:
                        run["quest"] = QUESTS.get(str(run["quest_id"]))
                        for player in run["players"]:
                            enrich_player(player, ITEMS, run["item_events"])
                        enrich_loot(run, ITEMS)
                        for item in run["loot"]:
                            item["name"] = ITEMS.get(str(item["item_id"]), f"Item {item['item_id']}")
                    # Raw samples/hits support arbitrary time windows and HP curves.
                    body = json.dumps(report, allow_nan=False).encode()
                    kind, status = 'application/json', 200
                except FileNotFoundError:
                    body = json.dumps({'error': 'No hunt journal yet. Complete a hunt with the recorder loaded, then refresh.'}).encode()
                    kind, status = 'application/json', 404
                except (OSError, ValueError):
                    body = json.dumps({'error': 'Could not read the hunt journal. Try refreshing after the game finishes writing.'}).encode()
                    kind, status = 'application/json', 503
            elif route in ('/', '/index.html', '/app.js', '/style.css', '/icon-sets.js', '/monster-icons.js', '/part-names.js', '/home-stats.js'):
                file = ROOT / ('index.html' if route in ('/', '/index.html') else route[1:])
                body = file.read_bytes()
                kind = {'.html':'text/html', '.js':'text/javascript', '.css':'text/css'}[file.suffix]
                status = 200
            else:
                self.send_error(404); return
            self.send_response(status)
            self.send_header('Content-Type', kind + '; charset=utf-8')
            self.send_header('Cache-Control', 'no-store')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers(); self.wfile.write(body)
        def log_message(self, *_): pass
    return Handler

if __name__ == '__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--journal',type=Path,default=DEFAULT)
    p.add_argument('--port',type=int,default=8768)
    a=p.parse_args()
    server=ThreadingHTTPServer(('127.0.0.1',a.port),handler(a.journal))
    print(f'Hunt history: http://127.0.0.1:{a.port}',flush=True)
    server.serve_forever()
