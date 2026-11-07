#!/usr/bin/env python3
"""Read-only MHGU 1.4.0 Arzuros pool diagnostic. No journal or game writes."""
import argparse
import json
import os
import struct
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

PAGE = Path(__file__).resolve().parents[1] / 'history/live-claws.html'

def snapshot(pid, base):
    with open(f'/proc/{pid}/mem', 'rb', buffering=0) as mem:
        def read(addr, fmt):
            size = struct.calcsize(fmt)
            return struct.unpack(fmt, os.pread(mem.fileno(), size, base + addr))[0]
        def pointer(addr):
            p = read(addr, '<I')
            if not 0x08000000 <= p < 0xa8000000 or p & 3:
                raise ValueError('Actor data unavailable')
            return p
        manager = pointer(read(0x200000 + 0x3013ac + 0x1537814, '<I'))
        for i in range(10):
            actor = read(manager + 0x1c + i*4, '<I')
            if not 0x08000000 <= actor < 0xa8000000:
                continue
            state = pointer(actor + 0x1428)
            if read(state + 0x6210, '<H') != 60:
                continue
            return dict(actor=actor, hp=read(state+0x370, '<I'),
                        pool=read(state+0x3be+2*12, '<h'),
                        flinches=read(state+0x3bc+2*12, '<B'),
                        broken=bool(read(state+0x5cbe, '<B') & 2))
        raise ValueError('No Arzuros loaded')

def main():
    args=argparse.ArgumentParser(description=__doc__)
    args.add_argument('--pid', type=int, required=True)
    args.add_argument('--base', type=lambda x:int(x,0), default=0x10000000000)
    args.add_argument('--port', type=int, default=8769)
    a=args.parse_args()
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == '/api/live':
                try:
                    body=json.dumps(snapshot(a.pid,a.base)).encode(); status=200
                except (OSError,ValueError,struct.error):
                    body=b'{"error":"Live data unavailable - game closed or Arzuros unloaded"}'; status=503
                kind='application/json'
            elif self.path == '/':
                body=PAGE.read_bytes(); status=200; kind='text/html; charset=utf-8'
            else:
                self.send_error(404); return
            self.send_response(status)
            self.send_header('Content-Type',kind)
            self.send_header('Cache-Control','no-store')
            self.send_header('Content-Length',str(len(body)))
            self.end_headers(); self.wfile.write(body)
        def log_message(self,*args): pass
    print(f'Live claws: http://127.0.0.1:{a.port}',flush=True)
    HTTPServer(('127.0.0.1',a.port),Handler).serve_forever()

if __name__=='__main__': main()
