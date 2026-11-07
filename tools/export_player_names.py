#!/usr/bin/env python3
"""Export name-only lookup tables from an owned MHGU English romfs."""
import argparse,json,struct
from pathlib import Path

def strings(path):
    data=path.read_bytes()
    if data[:4]!=b'GMD\0': raise ValueError(f'Not GMD: {path}')
    return [s.decode('utf-8') for s in data[41+struct.unpack_from('<I',data,36)[0]:].split(b'\0')]

def export(root):
    def names(file,stride):
        rows=strings(root/file)
        return {str(i//stride):rows[i] for i in range(0,len(rows)-1,stride) if rows[i]}
    return {'weapons':{str(i):names(f'weapon{i:02d}MsgData_eng.gmd',6) for i in range(15) if i!=5},
            'armor':{str(i//10):strings(root/'armorSeriesData_eng.gmd')[i:i+5] for i in range(0,len(strings(root/'armorSeriesData_eng.gmd'))-1,10)},
            'talismans':names('amuletData_eng.gmd',2),'skills':names('skillData_eng.gmd',2),'food_skills':names('catSkillData_eng.gmd',3)}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('output',type=Path);a=p.parse_args();a.output.write_text(json.dumps(export(a.root),ensure_ascii=False,separators=(',',':'))+'\n')
