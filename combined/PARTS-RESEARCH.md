# Part damage and breaks — MHGU 1.4.0

Main-relative ARM disassembly, 2026-11-03. Static verification plus saved Great
Maccao snapshots; post-install live validation remains pending.

- Existing damage callback main+9d6f8 takes (monster, pool slot, raw, mode).
  Slot<8 indexes status+488+2*slot at9d818..9d840. The returned damage is now
  saved with that slot. It is hit damage assigned to a part, NOT the final
  flinch/Partbreaker-modified damage. Player/monster attribution is unchanged.
- Generic reaction loop97ddc iterates eight pools (98a50..58). Part parameters
  come from owner+75ec -> +68, ten-byte records. Accessor main+71078
  adds 64 before its caller reads +4. The earlier +4 decoder read resource
  metadata (5253 on live Arzuros), causing null snapshots. Nonzero thresholds
  alone do NOT prove a zone is used: unused generic pools default to 100.
- status+3be+12*slot is signed flinch HP. 98344..54 reads it;986b0..9874c applies
  conditional multipliers/decrements/stores;99db8 resets its threshold. The
  threshold is quest/state dependent; do not infer a max from the first sample.
- status+3bc+12*slot counts flinches, incremented983fc..98424 and capped14.
- status+3c0+12*slot is the separate sever pool, read98884, decremented98970,
  stored98a48. status+3bd counts sever reactions (99348..99384), also capped14.
  These fields do not establish human-readable part names or generic break
  progression. Snapshot fields retain them separately.
- Break definitions owner+75f0 -> +64 with count at+8c consist of four bytes:
  pool, low-rank flinch requirement, high-rank requirement, signed group.
  99f98 compares required count with3bc; linked pools can be combined according
  to the sign of group. 9a2a8..9a300 updates bit(group&127) in status+5cbe only
  after the group condition is met, dispatching49964/3a6f54 quest notification.
  This mask, not a flinch count or material reward, drives part_break records.
- First-seen existing flags are `observed`; only subsequent new flags are
  `broken`. Each monster identity retains its seen mask through area absence;
  a new hunt resets it. Sample pools and hit slots are saved independently.
- Exclude generic pool decoding for Lao-Shan, Nakarkos, Ahtal-Neset until their
  virtual overrides are traced. Missing pointers stay unavailable. Break groups
  and pool slots are distinct game IDs; do not attach guessed anatomical names.

The saved Great Maccao byte sequences begin at3b6, six bytes BEFORE the flinch
counter. Its pool0 flinch counter went0->2 and HP61->2. Byte3bd remained0, which
is a sever counter, NOT proof that its head did not break. The earlier conclusion
that this byte might be a generic break counter was incomplete. Those snapshots
do not contain5cbe, so no old break event can be reconstructed reliably.

No new runtime hooks: hit attribution extends the existing gated damage hook;
pools are sampled once per second; break bits are checked each normal update.
No game writes or retroactive journal edits.

## Arzuros live check, 2026-11-03

Alive Arzuros (species60, HP1104) has exactly one break definition: bytes
`02 01 01 01`. This identifies pool2, one flinch for LR/HR, group1. Combined
with the sole Arzuros break reward condition (Claws), this maps pool2 to the
front legs/claws. The other zone names remain unverified. Live break mask
initially0. Thresholds130/180/80/150/220/100/100/100; current pool HP
123/171/76/142/209/95/95/95. The last three are generic defaults, not evidence
of three additional anatomical zones. Web UI exposes only hit pools and
break-defined pools, and lists break groups separately from damage zones.
Corrected pointer is covered by a regression fixture with invalid metadata
at param+4 and valid threshold pointer at param+68.

### Controlled arm hit

User reported a single 6-damage hit on Arzuros arms. Read-only live snapshot
afterward: monster HP1104->1098; pool2 flinch HP76->70; all seven other
pools unchanged. Break mask remains0 and pool2 flinch count remains0. This
independently verifies the arm/claw pool mapping, but not the break transition.

### Visible claw break confirmed

User confirmed the first visible claw break and flinch. Subsequent read-only
snapshot of the same actor (202769120): HP477, pool2 flinch HP58,
flinch count1, break-group1 flag set. Together with the controlled arm hit,
this validates the Arzuros pool2 / group1 mapping and its first break
transition. It does not validate other species or multi-stage breaks.
The corrected installed mod records break transitions and sampled pool/flinch
state after the next game launch; the running older mod is unchanged.

## Anatomical mapping and live bars (2026-11-03)

`tools/build_part_names.py` builds `src/part_names.rs` and the history mapping
from Kiranico's **Body Part / Stagger / Extract** tables, matched against the
owned RomFS `dttune` eight ten-byte records at 0x3c. It checks the complete
ordered threshold vector, including duplicate thresholds, rather than matching
one value in isolation. 91 of 94 named large-monster species match. Kecha Wacha
contains an interior default record at slot3; the exact unused default record
is skipped for alignment, retaining hind legs at4 and tail at5. The runtime
checks all eight base thresholds against the generated signature before naming
pools. Kiranico source URLs are retained in `history/part-names.json`.
Lao-Shan, Nakarkos and Ahtal-Neset remain excluded from the generic decoder.

Three independent Forge toggles: Show breakable parts (names/state), Show HP for
breakable parts (names/state plus bars), Show HP for all parts (all mapped zones
plus bars). The old always_show_parts INI key migrates to names/state; both HP
options default off. All-three-off hides the details. Logging is independent
of these display toggles. The panel remains bottom anchored and grows upward.

The generic stagger-reset function main+99db8 computes actual thresholds using
quest and virtual state multipliers. A signature-gated after-call hook reads the
resulting pool HP. Missing maxima stay0/unavailable rather than using base HP
or an already damaged first sample as a denominator. A secondary maximum is
recorded only when this call actually raises its HP (a reset), not from a
partially damaged pool read alongside a stagger reset. Cache is keyed by owner,
state and parameter pointers and cleared at recorded quest completion.

Serialized snapshots now retain base thresholds, anatomical labels, reset
maxima, LR/HR break requirements and signed-group linkage. Secondary definition
fields are at row+2 (required reactions), +4 (damage category), +6 (base HP).
A mapped Tail with category1 and a valid secondary requirement is shown with
sever progress; completion requires its counter to reach the requirement.
Other secondary pools are not labelled severed. Break status still comes from
the game's break mask, never from a stagger reset alone.

Flinch and secondary/sever counter increases produce per-monster, per-slot
`part_transition` journal events; initial observations do not replay old events.
Web history retains stagger bars after a part breaks and distinguishes the last
recorded pool values from damage landed. Existing logs can gain mapped names,
but missing historical pool values or maxima are not reconstructed.

Validation: all91 mapping signatures cross-checked against owned game files;
Arzuros controlled-hit and visible-break validation above; Rust adapter tests
cover unused slots, signature mismatch, three visibility switches, persistent
stagger visibility after severing, reset capture and event deduplication.
The shared-runtime integration suite runs serially because its adapters and
reset cache are process globals. New reset hook and non-Arzuros sever transitions
still require next-launch gameplay verification; the game has not been restarted.
