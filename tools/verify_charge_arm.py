#!/usr/bin/env python3
"""Execute owned MHGU charge instructions in isolated ARM memory.
Usage: python verify_charge_arm.py GAME_EXEFS_MAIN RECOMP_TOOLS
Requires lz4 and unicorn; no live-process access or distributed game bytes.
"""
from pathlib import Path
import struct,sys
sys.path.insert(0,sys.argv[2]);from nso import Nso
from unicorn import Uc,UC_ARCH_ARM,UC_MODE_ARM
from unicorn.arm_const import *
n=Nso(sys.argv[1]);u=Uc(UC_ARCH_ARM,UC_MODE_ARM);u.ctl_set_cpu_model(UC_CPU_ARM_MAX)
u.mem_map(0,n.size);u.mem_write(0,bytes(n.image));O=0x08000000;S=0x08020000;C=0x08030000
u.mem_map(O,0x40000)
def word(a,v):u.mem_write(a,struct.pack('<I',v&0xffffffff))
word(O+0x1428,S);word(O+0xcac0,C)
for active in (0,1):
 u.mem_write(S+0x5df3,bytes([active]));u.reg_write(UC_ARM_REG_R0,O);u.reg_write(UC_ARM_REG_LR,0x817a4)
 u.emu_start(0x81790,0x817a4);assert u.reg_read(UC_ARM_REG_R0)==active
# The real charged-state initialization fills the species-specific meter.
u.reg_write(UC_ARM_REG_R5,O+0xcac0);u.emu_start(0xeef25c,0xeef26c)
assert struct.unpack('<h',u.mem_read(C,2))[0]==100
for meter,delta,expected in [(100,-30,70),(70,-50,20),(20,-30,0)]:
 u.mem_write(C,struct.pack('<h',meter));u.reg_write(UC_ARM_REG_R4,O+0xcac0);u.reg_write(UC_ARM_REG_R9,delta&0xffffffff)
 u.emu_start(0xef0798,0xef07c0 if expected==0 else 0xef07d4)
 assert struct.unpack('<h',u.mem_read(C,2))[0]==expected
# Native charge entry and exit write the same flag consumed by the HUD.
u.reg_write(UC_ARM_REG_R4,O);u.reg_write(UC_ARM_REG_R3,1);u.emu_start(0xeef1ec,0xeef1fc)
assert u.mem_read(S+0x5df3,1)==b'\1'
u.reg_write(UC_ARM_REG_R5,O+0x1428);u.emu_start(0xeefce4,0xeefcf4)
assert u.mem_read(S+0x5df3,1)==b'\0'
print('Native charge flag, 100-point meter, event depletion, charge entry and exit passed')
