#!/usr/bin/env python3
"""Export the local MHGU Database loot tables used by hunt history."""
import json,sqlite3,sys
from pathlib import Path
import re
c=sqlite3.connect(sys.argv[1]);c.row_factory=sqlite3.Row
out={'source':'MHGU Database','monsters':{},'quests':{}}
for row in c.execute('select h.item_id,h.condition,h.rank,h.stack_size,h.percentage,m.name from hunting_rewards h join monsters m on m._id=h.monster_id'):
 r=dict(row);name=r.pop('name');out['monsters'].setdefault(name,[]).append(r)
for row in c.execute('select _id,rank from quests'):
 q=dict(row);qid=q['_id'];out['quests'][str(qid)]={'rank':q['rank'],'monsters':[r[0] for r in c.execute('select distinct m.name from monster_to_quest mq join monsters m on m._id=mq.monster_id where mq.quest_id=?',(qid,))],'rewards':[dict(r) for r in c.execute('select item_id,reward_slot,stack_size,percentage from quest_rewards where quest_id=?',(qid,))]}
# Small monsters are absent from monster_to_quest in this database. Include
# those explicitly named in the quest/subquest objectives, not every species.
small=[r[0] for r in c.execute("select name from monsters where class='1'")]
for row in c.execute('select _id,goal,sub_goal from quests'):
    q=out['quests'][str(row['_id'])]
    for name in small:
        if any(re.search(r'(?<!\w)'+re.escape(name)+r'(?!\w)',text or '') for text in (row['goal'],row['sub_goal'])) and name not in q['monsters']:
            q['monsters'].append(name)
Path(sys.argv[2]).write_text(json.dumps(out,separators=(',',':'),ensure_ascii=False)+'\n')
