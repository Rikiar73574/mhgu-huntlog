#!/usr/bin/env python3
import sys,time
from live_claws import snapshot
pid=int(sys.argv[1])
try:
    while True:
        try:
            d=snapshot(pid,0x10000000000)
            n=max(0,min(24,round(d['pool']/76*24)))
            text=(f"ARZUROS · CLAWS   {time.strftime('%H:%M:%S')}\n\n"
                  f"Pool HP   {d['pool']:>4}   [{'█'*n}{'░'*(24-n)}]\n"
                  f"Flinches  {d['flinches']:>4}\n"
                  f"Claws     {'BROKEN' if d['broken'] else 'Intact'}\n"
                  f"Monster   {d['hp']} HP\n\n"
                  "Live · 1 second · Ctrl+C to close")
        except (OSError,ValueError):
            text='Waiting for live Arzuros data…\nCtrl+C to close'
        print('\033[2J\033[H'+text,flush=True)
        time.sleep(1)
except KeyboardInterrupt:
    pass
