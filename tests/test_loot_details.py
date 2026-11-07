import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
from loot_details import enrich_loot,matches
class LootTests(unittest.TestCase):
 def run_data(self):return {'quest_id':206,'monsters':[{'name':'Great Maccao'}],'players':[{'id':1,'item_events':[]}],'loot':[]}
 def test_group_duplicates_hide_unknown_and_preserve_raw(self):
  r=self.run_data();p=r['players'][0]
  p['item_events']=[dict(t=t,item_id=i,quantity=q,action=a) for t,i,q,a in [(3,70,2,'acquired_unknown'),(41,70,-1,'action_consumed'),(191,56,-1,'action_consumed'),(254,576,1,'gathered'),(254,584,1,'gathered'),(254,576,1,'gathered'),(254,572,1,'gathered'),(254,571,1,'gathered')]]
  enrich_loot(r,{})
  self.assertEqual(len(p['item_events']),8)
  self.assertEqual(len(p['item_groups']),1)
  self.assertEqual(p['item_groups'][0]['label'],'Possible carves')
  self.assertEqual(p['item_groups'][0]['items'][0]['quantity'],2)
  self.assertFalse(any(s.get('confirmed') for i in p['item_groups'][0]['items'] for s in i['possible_sources']))
 def test_rank_and_unknown_monster_do_not_fabricate_confirmation(self):
  r=self.run_data();m=matches(r,576)
  self.assertTrue(any(x['monster']=='Great Maccao' and x['condition']=='Body Carve' for x in m))
  self.assertTrue(all(x['rank']=='LR' for x in m))
  self.assertFalse(matches(r,23))
  self.assertTrue(any(x['monster']=='Maccao' for x in matches(r,571)))
 def test_rewards_total_without_reclassifying_as_carves(self):
  r=self.run_data();r['loot']=[dict(item_id=576,quantity=2),dict(item_id=576,quantity=1)]
  enrich_loot(r,{})
  self.assertEqual(r['reward_items'][0]['quantity'],3)
  self.assertFalse(any('Carve' in x['condition'] for x in r['reward_items'][0]['possible_sources']))
 def test_separate_collection_sessions_stay_separate(self):
  r=self.run_data();r['players'][0]['item_events']=[dict(t=t,item_id=576,quantity=1,action='gathered') for t in [1,5,90]]
  enrich_loot(r,{})
  self.assertEqual(len(r['players'][0]['item_groups']),2)
if __name__=='__main__':unittest.main()
