"""Behavior tests for the standalone Forge hunt journal report."""
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "tools" / "hunt_history.py"


def row(run, event, t, **fields):
    return dict(schema=1, run=run, event=event, t=t, **fields)


class HistoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if SCRIPT.exists():
            spec = importlib.util.spec_from_file_location("hunt_history", SCRIPT)
            cls.history = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(cls.history)

    def read(self, rows, tail=""):
        self.assertTrue(SCRIPT.exists(), "journal reader has not been implemented")
        return self.history.read_journal(io.StringIO("".join(json.dumps(r) + "\n" for r in rows) + tail))

    def test_loadouts_and_local_item_actions_preserve_timing_and_provenance(self):
        loadout=dict(id=1,equipment=[dict(slot=0,type=10,id=18,level=1)],stats=dict(attack=98),skills=[10],food_skills=[4])
        report=self.read([row(1,"start",0),row(1,"sample",0,players=[loadout],monsters=[]),
            row(1,"item",5,player=1,item_id=23,quantity=-1,action="used"),
            row(1,"item",8,player=1,item_id=308,quantity=4,action="gathered"),
            row(1,"item",9,player=1,item_id=579,quantity=3,action="acquired_unknown"),
            row(1,"item",10,player=1,item_id=23,quantity=1,action="used"),row(1,"end",11)])
        run=report["runs"][0]
        self.assertEqual(report["warning_count"],1)
        self.assertEqual(run["players"][0]["equipment"],loadout["equipment"])
        self.assertEqual(run["players"][0]["food_skills"],[4])
        self.assertEqual([e["t"] for e in run["item_events"]],[5,8,9])
        self.assertEqual(run["item_events"][2]["action"],"acquired_unknown")

    def test_part_hits_pools_and_breaks_are_distinct(self):
        report=self.read([row(1,"start",0),row(1,"sample",0,players=[],monsters=[dict(id=1,parts=[dict(slot=0,flinch_hp=61,flinches=2)],break_mask=0)]),
            row(1,"hit",1,player=1,monster=1,damage=25,part=0),
            row(1,"part_break",2,monster=1,group=1,phase="broken"),row(1,"end",3)])
        run=report["runs"][0]
        self.assertEqual(report["warning_count"],0)
        self.assertEqual(run["hits"][0]["part"],0)
        self.assertEqual(run["monsters"][0]["parts"][0]["flinches"],2)
        self.assertEqual(len(run["part_breaks"]),1)
        self.assertEqual(run["part_breaks"][0]["group"],1)

    def test_automatic_rewards_preserve_slots_and_runs(self):
        award = dict(source="quest_reward",group=0,slot=0,item_id=576,quantity=2,monster=None)
        report=self.read([row(1,"start",0),row(1,"loot",10,**award),row(1,"loot",10,**award),
                          row(1,"loot",10,**dict(award,slot=1,quantity=1)),row(1,"end",10),
                          row(2,"start",0),row(2,"loot",10,**award),row(2,"end",10)])
        self.assertEqual(report["warning_count"],0)
        self.assertEqual(sum(x["quantity"] for x in report["runs"][0]["loot"]),3)
        self.assertEqual(sum(x["quantity"] for x in report["runs"][1]["loot"]),2)
        self.assertEqual(report["runs"][0]["monsters"],[])

    def test_invalid_rewards_are_rejected(self):
        award=dict(source="quest_reward",group=0,slot=0,item_id=576,quantity=2,monster=None)
        bad=[dict(award,quantity=-1),dict(award,slot=48),dict(award,group=4),dict(award,item_id=True),dict(award,source="carve"),dict(award,monster=1)]
        report=self.read([row(1,"start",0)]+[row(1,"loot",1,**x) for x in bad])
        self.assertEqual(report["warning_count"],len(bad))
        self.assertEqual(report["runs"][0]["loot"],[])

    def test_status_events_and_per_monster_hit_counts(self):
        rows = [row(1,"start",0,status_tracking=True),
                row(1,"sample",0,monsters=[dict(id=1),dict(id=2)],players=[dict(id=1)]),
                row(1,"hit",1,monster=1,player=1,damage=20),
                row(1,"hit",2,monster=2,player=1,damage=30),
                row(1,"status",3,monster=1,kind="paralysis",phase="start",count=1),
                row(1,"status",4,monster=2,kind="mount_ended",phase="trigger",count=1),
                row(1,"end",5)]
        report = self.read(rows)
        run = report["runs"][0]
        self.assertEqual(report["warning_count"],0)
        self.assertTrue(run["status_tracking"])
        self.assertEqual(len(run["status_events"]),2)
        self.assertEqual(run["players"][0]["hit_count"],2)
        self.assertEqual(run["monsters"][0]["players"][0]["hit_count"],1)
        self.assertEqual(run["monsters"][1]["players"][0]["damage"],30)
        self.assertFalse(self.read([row(2,"start",0)])["runs"][0]["status_tracking"])
        self.assertEqual(self.read([row(3,"status",1,monster=1,kind="fake",phase="start",count=1)])["warning_count"],1)

    def test_recorder_metadata_is_preserved_without_guessing_missing_history(self):
        report = self.read([
            row(1, "start", 0),
            row(1, "sample", 1, monsters=[], players=[dict(id=2, hr=47, is_local=True), dict(id=1, hr=999, is_local=False)]),
            row(1, "end", 10, completed_at=1793678400),
            row(2, "start", 0), row(2, "end", 10),
        ])
        first, old = report["runs"]
        self.assertEqual(first["completed_at"], 1793678400)
        self.assertEqual(next(p for p in first["players"] if p["is_local"])["hr"], 47)
        self.assertIsNone(old["completed_at"])
        self.assertIsNone(old["recorder_hr"])

    def test_same_quest_runs_and_same_species_monsters_stay_separate(self):
        report = self.read([
            row(101, "start", 0, quest_id=77, timer_source="quest", partial=False),
            row(101, "sample", 1, players=[], monsters=[
                dict(id=1, species="Rathalos", hp=100, max_hp=100),
                dict(id=2, species="Rathalos", hp=90, max_hp=100)]),
            row(101, "end", 8, reason="quest_left", outcome=None),
            row(102, "start", 0, quest_id=77, timer_source="quest", partial=False),
        ])
        self.assertEqual([r["run"] for r in report["runs"]], [101, 102])
        self.assertEqual(len(report["runs"][0]["monsters"]), 2)
        self.assertIsNone(report["runs"][0]["monsters"][0]["crown"])
        self.assertIsNone(report["runs"][0]["outcome"])
        self.assertTrue(report["runs"][0]["complete"])
        self.assertFalse(report["runs"][1]["complete"])

    def test_partial_torn_and_invalid_records_are_counted(self):
        report = self.read([
            row(7, "start", 10, quest_id=None, timer_source="observed", partial=True),
            row(7, "sample", 11, monsters=[], players=[]),
            row(7, "hit", 12, player=1, monster=1, damage=-10),
        ], '{"schema":1,"run":7,')
        self.assertEqual(report["warning_count"], 2)
        self.assertTrue(report["runs"][0]["partial"])
        self.assertFalse(report["runs"][0]["complete"])
        self.assertEqual(report["runs"][0]["duration_seconds"], 1)

    def test_timestamp_bins_include_idle_player_and_short_final_bin(self):
        report = self.read([
            row(1, "start", 100, partial=False),
            row(1, "sample", 100, monsters=[], players=[
                dict(id=10, slot=0, name="Alice", weapon_type=1),
                dict(id=20, slot=1, name="Bob", weapon_type=2)]),
            row(1, "hit", 101, player=10, monster=1, damage=50),
            row(1, "hit", 105, player=10, monster=1, damage=100),
            row(1, "hit", 111, player=10, monster=1, damage=40),
            row(1, "end", 112, reason="left", outcome=None),
        ])
        run = report["runs"][0]
        self.assertEqual(run["duration_seconds"], 12)
        self.assertEqual(run["players"][0]["damage"], 190)
        self.assertEqual(run["players"][0]["dps_bins"], [10, 20, 20])
        self.assertEqual(run["players"][1]["dps_bins"], [0, 0, 0])
        self.assertEqual(run["bin_durations"], [5, 5, 2])

    def test_cli_outputs_html_and_json_and_escapes_labels(self):
        self.assertTrue(SCRIPT.exists(), "journal reader has not been implemented")
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            source = tmp / "hunts.jsonl"
            source.write_text(json.dumps(row(1, "start", 0, timer_source="<script>")) + "\n" +
                              json.dumps(row(1, "sample", 2, monsters=[], players=[
                                  dict(id=3, name='<img src=x onerror="alert(1)">', slot=0, weapon_type=None)])) + "\n")
            result = subprocess.run([sys.executable, str(SCRIPT), "--input", str(source),
                                     "--output", str(tmp / "report.html"), "--json", str(tmp / "report.json")],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            html = (tmp / "report.html").read_text()
            self.assertNotIn("<img", html)
            self.assertIn("&lt;img", html)
            self.assertIn("<svg", html)
            self.assertFalse(json.loads((tmp / "report.json").read_text())["runs"][0]["complete"])

    def test_missing_start_preserves_hits_as_partial_with_unknown_metadata(self):
        report = self.read([
            row(9, "hit", 25, player=100, monster=4, damage=12),
            row(9, "end", 30, reason="left", outcome=None),
        ])
        run = report["runs"][0]
        self.assertTrue(run["partial"])
        self.assertTrue(run["complete"])
        self.assertEqual(report["warning_count"], 1)
        self.assertIsNone(run["players"][0]["name"])
        self.assertIsNone(run["monsters"][0]["species"])
        self.assertEqual(run["players"][0]["dps_bins"], [2.4])

    def test_end_boundary_hit_is_retained_and_invalid_tail_cannot_extend_run(self):
        report = self.read([
            row(3, "start", 0, partial=False),
            row(3, "hit", 5, player=1, monster=1, damage=100),
            row(3, "end", 5, reason="left"),
            row(3, "hit", 10, player=1, monster=1, damage=200),
        ])
        self.assertEqual(report["warning_count"], 1)
        self.assertEqual(report["runs"][0]["players"][0]["dps_bins"], [20])

    def test_empty_journal_generates_report(self):
        report = self.read([])
        self.assertEqual(report["runs"], [])
        self.assertEqual(report["warning_count"], 0)
        self.assertIn("0 runs", self.history.render_html(report))

    def test_zero_hp_infers_killed_without_overriding_capture(self):
        report=self.history.read_journal([json.dumps(row(1,"sample",0,monsters=[dict(id=1,hp=0),dict(id=2,hp=0,outcome="captured"),dict(id=3,hp=10)],players=[]))])
        monsters=report["runs"][0]["monsters"]
        self.assertEqual(monsters[0]["outcome"],"killed")
        self.assertEqual(monsters[0]["outcome_source"],"inferred_zero_hp")
        self.assertEqual(monsters[1]["outcome"],"captured")
        self.assertIsNone(monsters[2]["outcome"])

    def test_per_monster_graphs_do_not_mix_same_species_instances(self):
        report = self.history.read_journal([json.dumps(r) for r in [
            row(1, "start", 0, quest_id=42, partial=False),
            row(1, "sample", 0, monsters=[dict(id=1,species=17),dict(id=2,species=17)], players=[dict(id=1,name="Hunter")]),
            row(1, "hit", 1, player=1, monster=1, damage=20),
            row(1, "hit", 2, player=1, monster=2, damage=50),
            row(1, "hit", 3, player=1, monster=None, damage=999),
            row(1, "end", 5, reason="town")]])
        run = report["runs"][0]
        self.assertEqual(run["players"][0]["damage"],70)
        self.assertEqual([m["players"][0]["dps_bins"] for m in run["monsters"]],[[4.0],[10.0]])

    def test_unknown_hit_target_is_preserved_but_excluded_from_dps(self):
        report = self.read([
            row(1, "start", 0),
            row(1, "hit", 1, player=42, monster=None, damage=30),
            row(1, "end", 5, reason="left"),
        ])
        run = report["runs"][0]
        self.assertEqual(report["warning_count"], 0)
        self.assertEqual(run["monsters"], [])
        self.assertIsNone(run["unattributed_hits"][0]["monster"])
        self.assertEqual(run["hits"], [])
        self.assertEqual(run["players"][0]["damage"], 0)
        self.assertEqual(run["players"][0]["dps_bins"], [0])

    def test_size_crown_weapon_id_and_dropped_event_warnings(self):
        report = self.read([
            row(1, "start", 0),
            row(1, "sample", 2, dropped_events=3,
                monsters=[dict(id=1, species=10, size=1234.5, size_scale=1.1, crown="large_gold")],
                players=[dict(id=42, name="Hunter", slot=0, weapon_type=2, weapon_id=99)]),
            row(1, "end", 5, reason="left", dropped_events=2),
        ])
        run = report["runs"][0]
        self.assertEqual(run["monsters"][0]["size"], 1234.5)
        self.assertEqual(run["monsters"][0]["size_scale"], 1.1)
        self.assertEqual(run["monsters"][0]["crown"], "large_gold")
        self.assertEqual(run["players"][0]["weapon_id"], 99)
        self.assertEqual(report["warning_count"], 2)
        self.assertIn("1234.5", self.history.render_html(report))
        self.assertTrue(all("dropped" in warning for warning in report["warnings"]))


if __name__ == "__main__":
    unittest.main()

class PartTransitionTests(unittest.TestCase):
    def test_part_transition_roundtrip_and_validation(self):
        import importlib.util
        path = Path(__file__).resolve().parents[1] / 'tools' / 'hunt_history.py'
        spec = importlib.util.spec_from_file_location('part_history', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        event = dict(schema=1,run=999,event='part_transition',t=12,monster=1,slot=2,kind='flinch',count=1)
        module.validate(event)
        report=module.read_journal([json.dumps(event)])
        self.assertEqual(report['runs'][0]['part_events'][0]['kind'],'flinch')
        for patch in ({'slot':8},{'count':15},{'kind':'broken'},{'monster':0}):
            with self.assertRaises(ValueError):module.validate(dict(event,**patch))
