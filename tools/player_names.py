"""Human-readable loadout names, preserving raw IDs and every timed sample."""
import json
from pathlib import Path
from history_paths import HISTORY
NAMES=json.loads((HISTORY/'player-names.json').read_text(encoding="utf-8"))
DETAILS=json.loads((HISTORY/'equipment-details.json').read_text(encoding="utf-8"))
SLOTS=('Weapon','Head','Chest','Arms','Waist','Legs','Talisman')
def enrich_player(p,items,events):
    equipment=p.get('equipment')
    if isinstance(equipment,list):
        for e in equipment:
            if not isinstance(e,dict):continue
            slot=e.get('slot');key=str(e.get('id'))
            if slot==0:name=NAMES['weapons'].get(str(p.get('weapon_type')),{}).get(key)
            elif isinstance(slot,int) and 1<=slot<=5:
                names=NAMES['armor'].get(key,[]);name=names[slot-1] if len(names)>=slot else None
            elif slot==6:name=NAMES['talismans'].get(key)
            else:continue
            e['name']=name or f'Equipment {key}'
            e['slot_name']=SLOTS[slot]
            e['reference']=DETAILS['armor'].get(e['name']) if 1<=slot<=5 else None
            if isinstance(e.get('decorations'),list):
                e['decoration_names']=[items.get(str(i),f'Decoration {i}') for i in e['decorations']]
            if isinstance(e.get('skill_points'),list):
                totals={}
                for entry in e['skill_points']:
                    name=DETAILS['skill_trees'].get(str(entry['tree']),f"Skill tree {entry['tree']}")
                    totals[name]=totals.get(name,0)+entry['points']
                e['skill_point_names']=[dict(name=name,points=points) for name,points in totals.items()]

            if slot==0:p['weapon_name']=f"{e['name']} · Lv {e.get('level','?')}"
    for field in ('skills','food_skills'):
        if isinstance(p.get(field),list):p[field+'_names']=[NAMES[field].get(str(i),f'Skill {i}') for i in p[field]]
    p['item_events']=[dict(e,name=items.get(str(e['item_id']),f"Item {e['item_id']}")) for e in events if e['player']==p['id']]
