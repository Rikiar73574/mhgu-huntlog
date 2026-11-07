"""Group observed loot; table matches are evidence of possibilities, not actions."""
import json
from pathlib import Path
from history_paths import HISTORY
TABLES=json.loads((HISTORY/'loot-tables.json').read_text(encoding="utf-8"))

def matches(run,item_id,reward=False):
    quest=TABLES['quests'].get(str(run['quest_id']),{})
    rank=quest.get('rank')
    names=set(quest.get('monsters',[]))|{m['name'] for m in run['monsters'] if m.get('name')}
    found=[]
    for name in sorted(names):
        for row in TABLES['monsters'].get(name,[]):
            condition=row['condition']
            eligible=(condition.startswith(('Break','Capture')) if reward else any(x in condition for x in ('Carve','Shiny','Gather','Mining')))
            if row['item_id']==item_id and (not rank or row['rank']==rank) and eligible:
                match=dict(monster=name,condition=condition,rank=row['rank'])
                if match not in found:found.append(match)
    if reward:
        for row in quest.get('rewards',[]):
            if row['item_id']==item_id:
                match=dict(quest=True,condition=row['reward_slot'],rank=rank)
                if match not in found:found.append(match)
    return found

def add_item(group,e,run,items,reward=False):
    item=next((x for x in group['items'] if x['item_id']==e['item_id']),None)
    if item is None:
        item=dict(item_id=e['item_id'],name=items.get(str(e['item_id']),f"Item {e['item_id']}"),quantity=0,possible_sources=matches(run,e['item_id'],reward))
        group['items'].append(item)
    item['quantity']+=abs(e['quantity'])

def enrich_loot(run,items):
    for player in run['players']:
        groups=[]
        # Presentation only: raw events, including unclassified ones, stay in JSON.
        for event in player.get('item_events',[]):
            if event['action'] not in ('used','gathered'):continue
            action=event['action']
            if not groups or groups[-1]['action']!=action or event['t']-groups[-1]['end']>15:
                groups.append(dict(action=action,start=event['t'],end=event['t'],items=[]))
            group=groups[-1];group['end']=event['t'];add_item(group,event,run,items)
        for group in groups:
            group['label']='Used' if group['action']=='used' else 'Collected'
            if group['action']=='gathered' and any('Carve' in s['condition'] for item in group['items'] for s in item['possible_sources']):
                group['label']='Possible carves'
        player['item_groups']=groups
    group={'items':[]}
    for event in run['loot']:add_item(group,event,run,items,True)
    run['reward_items']=group['items']
