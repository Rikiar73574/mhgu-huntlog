# Local-player loadout baseline, 2026-11-03

Read-only live inspection, in town before and after eating. Evidence snapshots/screens reside in `../build/player-baseline/`. No game memory writes or save edits.

Pointer chain: sGameControl at runtime 0x1a8ca48, local slot +0x30; sPlayer at 0x1a85b58, owner +0x1c+slot*4; player data at owner+0x13c4. Current runtime base for main is 0x200000. Do not hardcode current owner/data pointers.

Verified screen/memory matches: Rikiar HR1, weapon type3/id80 (Thane Lance Lv1); premeal Attack95, Defense119, HP100; after meal Attack98, HP110. Data+0x33e signed16 matches attack, +0x34a signed16 matches defense, +0x34c five signed16 matches fire/water/thunder/ice/dragon (6,2,-4,-4,0). Data+0x338 matches displayed health. Stamina display100 while adjacent value600 needs unit confirmation. Data+0x320 also follows attack; semantics versus +0x33e still need disassembly/change testing.

Screen equipment: Jaggi Helm, Kut-Ku Mail, Bulldrome Braces, Kut-Ku Tassets, Bulldrome Greaves, Bishop Talisman. Active Attack Up (M), Gathering +1. Style Striker; arts Absolute Evasion, Shield Assault I, Enraged Guard I (third is SP).

Seven skill-contribution pointers in data at 0x40,0x6c,0x98,0xc4,0xf0,0x11c,0x148 match weapon/head/chest/arms/waist/legs/talisman. In inspected payload, 4-byte records use u16 tree ID, signed8 points at +2. Do not parse full 0x70 allocation: metadata/pointers begin after the skill entries. Scanning first0x40 produces all screen matches in this set, but full list boundaries still need proof.

Tree18 Attack: head3/chest3/arms4/waist2/legs4 =16. Tree19 Defense: -2/-3/-1/-2/-1 =-9. Tree27 Fire Atk: head1/chest2 =3. Tree131 Gathering: talisman10. Screen totals and all slot contributions match. Equipment names and level mappings are not yet decoded generically.

Meal activated Felyne Black Belt. Only data differences: 0x320 95->98, 0x338 100->110, 0x33e 95->98, 0x3a8 0->1. Last is a food-related candidate, not a verified general food decoder. Food skill names currently verified visually. Preserve no-food versus unavailable distinctly.

Next: camp baseline and stable inventory pointer/entry identification, then controlled consumption and collection; verify loadout survives quest transition. Do not mark placeholder equipment/food/item recorder fields implemented based on these observations alone.

## Camp and pouch baseline

Quest transition changed the local owner pointer; resolve it on each update. Player data remained readable and retained Attack98/Defense119/HP110 and the postmeal field at0x3a8.

Verified local pouch global runtime0x1a584b8 (main+0x18584b8), derived from item-selection wrapper main+0x2880fc and next-slot routine main+0x19bfcc. Entries use item ID u16 at pouch+0x1a+4*slot, quantity signed16 at+0x1c+4*slot. The selection code tests quantity>0 then retrieves the item ID at main+0x19c070..0x19c080. There are50 navigable entries including virtual/game-managed slots: slot40 ID93 qty1, slot41 ID206 qty99, slot47 ID100 qty99. Do not report these as gathered inventory. Normal carried pouch slot range still needs full boundary proof.

User-listed pouch and memory matched all eight entries: Potion ID9 qty10, Cool Drink ID23 qty5, Hot Drink ID24 qty5, Well-done Steak ID50 qty10, Whetstone ID56 qty10, Iron Pickaxe ID58 qty5, Bug Net ID61 qty5, Paintball ID70 qty2.

A quantity decrease alone is not proof of usage: discard, combining, transfers and breakage also change counts. Controlled-use/gather tests establish observations but general event classification needs the game's corresponding action signal or hooks. This pouch is local-only; never imply remote players' inventories are available from it.

Controlled camp test: user consumed exactly one Cool Drink. Snapshot changed only slot1 ID23 qty5->4; all other50 entries identical. This verifies the item/quantity decode, not a generic usage-vs-discard classifier.

Controlled gathering test: user gathered Needleberry x4 once. Compared with post-drink snapshot, only normal slot8 changed from empty (ID0 qty0) to ID308 qty4. MHGU Database item308 independently resolves to Needleberry. No other50-entry differences. Raw deltas preserved in build/player-baseline/{cool-drink-delta,gather-delta}.json.

## Great Maccao after-kill comparison

Same live object0x0c1515d0, species85: HP804/920 at baseline, HP0/920 after user reported death. At candidate part array status+0x3b6 with12-byte stride, byte+6 rose0->2 in slots0 and1; byte+7 stayed0 in all slots. Candidate signed16+10 stayed0 throughout. These are NOT confirmed break events: inherited MHXX layout labels +6 as stagger count, and MHGU layout/part naming remain unverified. User could not visually confirm head break. Record killed from HP0; head-break outcome unknown. Preserve both raw snapshots for disassembly validation; do not treat the +6 changes as two breaks or zero +7 as proof of no break.

Carve test: user reports Great Maccao Hide x3 from carves. Pouch slot9 changed empty->ID579 qty3; database independently resolves ID579 to Great Maccao Hide. Saved `carves-observation.json` with user-confirmed provenance; exact carve timestamps were not captured. Automatic logging must distinguish body carve, severed-tail carve, gathering, shinies, supplies and quest rewards using action/source signals, not just item ID or inventory growth. Additional Potion/Steak/Paintball decreases occurred between snapshots; their causes/times were not independently verified and must not be fabricated as confirmed uses.

## Implemented automatic recorder (2026-11-03)

`player_details.rs` now reads seven equipment records at data+0x18+slot*0x2c. Packed+4: type bits0..4, level index bits5..9; base ID+6. Active skill IDs are 20 u16 at pointer(data+0x18c)+0x3a0 (main+e09c8..e0adc); food IDs are three bytes at pointer(data+0x510)+0xd, gated by data+0x504 bit4 (main+26fe10 /4d3a08, used by meal-skill UI main+62f44c). Data+0x3a8 is a stat-change comparison, NOT a food-skill ID. Equipment contribution arrays contain 11 entries, not16. Names extracted reproducibly from the owned English GMDs. Weapon names are base upgrade-chain names plus level; renamed final/ultimate upgrade variants are not yet resolved.

Local item capture hooks main+19b7fc and compares actual pouch counts before/after the operation. Starting inventory, rearrangements, failed/full-pouch operations, and unchanged calls emit no event. Hook main+3015e0 supplies caller context for consumption; return sites2fb098/2fb8e8 are verified item-effect consumption. Hook main+2e04d4 supplies collection context; return site2e0884 is the gathering manager award. All other mutations retain their exact caller/context and are explicitly unclassified; no unsupported carve, supply, discard, combine, transfer or monster attribution. Actions are timestamped at the latest quest update. Inventory events are local-only. Item-conservation skills that avoid quantity loss do not emit a use event yet.

Full item-source classification, successful uses without consumption, weapon element/affinity, decorations, individual armor statistics, and verified remote-player loadouts remain follow-ups. All player samples retain equipment/stats/skill changes. The website currently shows the last sampled loadout and timed item actions. Controlled-snapshot and synthetic tests pass; the new hooks need a live hunt after restarting. Existing status/reward tracking remains enabled.
