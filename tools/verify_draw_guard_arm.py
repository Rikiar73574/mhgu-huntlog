#!/usr/bin/env python3
"""Exercise the compiled overlay draw-handle guard with a failing Forge getter.
Usage: python verify_draw_guard_arm.py ELF NRO
Requires Unicorn and llvm-objdump. Does not touch the running game.
"""
import re
import subprocess
import sys
from pathlib import Path
from unicorn import Uc, UC_ARCH_ARM, UC_MODE_ARM, UC_HOOK_CODE
from unicorn.arm_const import *

asm = subprocess.check_output(['llvm-objdump', '-d', '--demangle', sys.argv[1]], text=True)
def address(name):
    return int(re.search(r'^([0-9a-f]+) <' + re.escape(name) + r'>:', asm, re.M)[1], 16)
guard = address('mhgu_dps_meter::foreground_draw_list')
getter = address('igGetForegroundDrawList_Nil@plt')
image = Path(sys.argv[2]).read_bytes()
u = Uc(UC_ARCH_ARM, UC_MODE_ARM)
u.ctl_set_cpu_model(UC_CPU_ARM_MAX)
u.mem_map(0, 0x100000)
u.mem_write(0, image)
u.mem_map(0x10000000, 0x10000)
stop = 0x1000f000
value = 0
calls = 0
def hook(u, pc, size, _):
    global calls
    if pc == getter:
        calls += 1
        u.reg_write(UC_ARM_REG_R0, value)
        u.reg_write(UC_ARM_REG_PC, u.reg_read(UC_ARM_REG_LR))
u.hook_add(UC_HOOK_CODE, hook)
# First failure, repeated failure, and recovery must all return safely.
# Logging is uninitialized, as it is in this isolated NRO harness.
for value in [0, 0, 0x12345000, 0]:
    u.reg_write(UC_ARM_REG_SP, 0x10008000)
    u.reg_write(UC_ARM_REG_LR, stop)
    u.emu_start(guard, stop, count=10000)
    assert u.reg_read(UC_ARM_REG_PC) == stop
    assert u.reg_read(UC_ARM_REG_R0) == int(value != 0)
    assert u.reg_read(UC_ARM_REG_R1) == value
assert calls == 4
print('Compiled ARM guard passed: null, repeated null, valid handle recovery, null again')
