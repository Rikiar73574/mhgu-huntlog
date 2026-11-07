# Equipment detail recording

MHGU 1.4 main-relative disassembly, 2026-11-03:
- cEquip is player data +18 + slot*2c, existing verified kind/id/level fields.
- main+ddf98, ddfe0, de028 read u16 decoration item IDs at +a/+c/+e.
  Each nonzero ID is passed to main+194230; zero means empty.
- main+de150 returns the +28 pointer plus entry*4. Main+ddd48 rebuilds
  11 entries, each u16 skill tree, s8 points. Entries include piece and jewel
  contributions; duplicates are summed by the history reader. They are before
  set-wide effects such as Torso Up. Unreadable arrays stay null.
- These fields are now stored inside each equipment object; existing history
  parsing preserves that object. Old journals are not modified.

The equipment-details.json armor reference was exported from the local
/tmp/mhgu-quest-db/mhgu.db (armor joined to items by _id, skill points via
item_to_skill_tree and skill_trees), matched to the recorded equipment by exact
English name. Skill tree IDs/names come from owned romfs
eng/table/skillTypeData_eng.gmd, stride 2, via export_player_names.strings.
Reference points are labeled Base skill points, not recorded loadout totals.
No equipped jewels or random talisman properties are inferred for older runs.
