"""Optional original-code verification: pass a local decompressed MHGU 1.4.0 main image. Requires Unicorn."""
from pathlib import Path
import sys
import struct
from unicorn import Uc, UC_ARCH_ARM, UC_MODE_ARM
from unicorn.arm_const import UC_ARM_REG_R0,UC_ARM_REG_R1,UC_ARM_REG_LR,UC_ARM_REG_SP
b=Path(sys.argv[1]).read_bytes();u=Uc(UC_ARCH_ARM,UC_MODE_ARM);u.mem_map(0,(len(b)+4095)&~4095);u.mem_write(0,b)
u.mem_map(0x10000000,0x500000);u.mem_map(0x3000000,4096)
M,C,A,B,SP=0x10000000,0x10100000,0x10200000,0x10300000,0x104ff000
w=lambda a,v:u.mem_write(a,struct.pack('<I',v))
w(struct.unpack_from('<I',b,0x29880+8+0x180f338)[0],M)
w(M+0x1c,A);w(M+0x20,B)
def call(fn,arg=0):
 u.reg_write(UC_ARM_REG_R0,C);u.reg_write(UC_ARM_REG_R1,arg);u.reg_write(UC_ARM_REG_SP,SP);u.reg_write(UC_ARM_REG_LR,0x3000000)
 u.emu_start(fn,0x3000000,count=10000);return u.reg_read(UC_ARM_REG_R0)
for index,expected in [(0,0),(1,A),(2,B),(3,0)]:
 w(C+0xa6c,index);assert call(0x29a58)==expected
u.mem_write(A+0xb5f7,b'\1');w(C+0xa6c,1);assert call(0x29a58)==B
print('Original game target getter: deselect, first/second, invalid index, skipped entry verified')
