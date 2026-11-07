#!/usr/bin/env python3
"""Execute owned MHGU ARM timer routines in isolated memory (no running-game writes).
Usage: python verify_timer_arm.py GAME_EXEFS_MAIN RECOMP_TOOLS
Requires lz4 and unicorn. Neither game code nor snapshots are distributed.
"""
import sys,struct
sys.path.insert(0,sys.argv[2])
from nso import Nso
from unicorn import Uc,UC_ARCH_ARM,UC_MODE_ARM
from unicorn.arm_const import *
n=Nso(sys.argv[1]);u=Uc(UC_ARCH_ARM,UC_MODE_ARM);u.ctl_set_cpu_model(UC_CPU_ARM_MAX)
u.mem_map(0,n.size);u.mem_write(0,bytes(n.image))
u.mem_map(0x10000000,0x40000);u.mem_map(0x11000000,0x10000);u.mem_map(0x20000000,0x1000)
u.reg_write(UC_ARM_REG_C1_C0_2,0xf00000);u.reg_write(UC_ARM_REG_FPEXC,1<<30)
P=0x10010000;S=0x10030000;V=0x10000000;STOP=0x20000000
word=lambda a,v:u.mem_write(a,struct.pack('<I',v))
fword=lambda a,v:u.mem_write(a,struct.pack('<f',v))
get=lambda a:struct.unpack('<f',u.mem_read(a,4))[0]
def call(pc,r0,r1=0,value=0):
 u.reg_write(UC_ARM_REG_SP,0x11008000);u.reg_write(UC_ARM_REG_LR,STOP)
 u.reg_write(UC_ARM_REG_R0,r0);u.reg_write(UC_ARM_REG_R1,r1)
 u.reg_write(UC_ARM_REG_S0,struct.unpack('<I',struct.pack('<f',value))[0])
 u.emu_start(pc,STOP,count=10000)
 assert u.reg_read(UC_ARM_REG_PC)==STOP
word(P,V);word(V+0x3ac,STOP);word(P+0x1428,S)
call(0x2a6dc0,P,value=36000.0);assert get(P+0x287c)==36000.0
call(0x2a6d84,P,value=36000.0);assert get(P+0x2880)==36000.0
call(0xc0908,P);assert get(S+0x1c4)==36000.0
# Actual game updater subtracts actor frame step times the supplied multiplier.
fword(P+0x1c,60.0)
for a,multiplier in [(P+0x287c,1.5),(P+0x2880,1.0),(S+0x1c4,1.0)]:
 call(0x53aa84,P,a,multiplier);assert get(a)==36000.0-60.0*multiplier
 fword(a,0.5);call(0x53aa84,P,a,multiplier);assert get(a)==0.0
# Refresh overwrites the remaining value rather than accumulating it.
call(0x2a6dc0,P,value=36000.0);assert get(P+0x287c)==36000.0
print('Actual ARM routines passed: Hot/Cool Drink and paint setters, normal/extreme decay, expiry clamp, reapplication')
