import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
from player_names import enrich_player,NAMES
class EquipmentDetails(unittest.TestCase):
 def player(self):
  key=next(k for k,v in NAMES['armor'].items() if v[0]=='Jaggi Helm')
  return {'id':1,'equipment':[{'slot':1,'id':int(key),'level':1}]}
 def test_old_hunt_keeps_missing_decorations_unknown(self):
  p=self.player();enrich_player(p,{},[]);e=p['equipment'][0]
  self.assertEqual(e['reference']['slots'],0)
  self.assertIn({'name':'Attack','points':3},e['reference']['skill_points'])
  self.assertNotIn('decoration_names',e)
  self.assertNotIn('skill_point_names',e)
 def test_signed_points_and_duplicate_sources(self):
  p=self.player();p['equipment'][0].update(decorations=[2638],skill_points=[{'tree':1,'points':3},{'tree':1,'points':-1}])
  enrich_player(p,{'2638':'Test jewel'},[]);e=p['equipment'][0]
  self.assertEqual(e['decoration_names'],['Test jewel'])
  self.assertEqual(e['skill_point_names'],[{'name':'Poison','points':2}])
if __name__=='__main__':unittest.main()
