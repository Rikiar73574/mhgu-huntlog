# MHGU 1.4.0 target camera evidence

Addresses below are offsets from main's text base (normally guest 0x200000).
The local decompressed game image was inspected read-only; it is not included.

- `uQuestCamera` constructor main+0x1fa00 installs vtable main+0x1730d0c.
- Game target panel obtains the active camera from `sCamera + 0x54` and checks
  it is a quest camera (e.g. main+0x656d94). The HUD uses exact vtable matching,
  conservatively rejecting other camera types.
- Getter main+0x29868 returns camera+0xa6c, the selected target number.
- main+0x29a58 resolves that selection. Normal mode uses a 1-based index over
  sEnemy+0x1c's ten entries after skipping object+0xb5f7 != 0. Zero deselects.
- Special mode camera+0xa8e is Nakarkos (species 0x54). Its three target points
  belong to the parent at camera+0xa74. Map them to the parent HUD card.
- Camera update main+0x1fec4 resets the icon state/species each update. The
  non-special path at +0x20398 admits encountered monsters to a70/a74 when
  status+0x2c0 is 1 or 2, and writes species for accepted identities at +0x2048c.
- The game's enemy target panel calls main+0x29c38 for icon state and
  main+0x29ce8 for species. States originate at a7d/a7e (special extra a7f),
  species at a80/a82 (special extra a84). At main+0x657028 the icon is resolved
  only when both the UI state and species are nonzero; otherwise no identity.
- This HUD maps state/species by current eligible enemy index, checks discovered
  membership separately in a70/a74, and honors the a90 icon visibility mask.
  It requires an exact object AND species match,
  and only accepts emitted states 1,2,4,8. It never reveals by species alone.

Verification:
- Ten fixture tests cover target switching/deselection, filtered list indexing,
  unrevealed selection, same-species different objects, wrong camera/stale species,
  invalid/missing data special target parts, enemy reordering and icon visibility.
- The original game's main+0x29a58 getter was executed in Unicorn using memory
  fixtures, verifying zero/first/second/out-of-range/skipped-entry selections.
- ARM build and existing 24 data/config + 3 bottom-layout tests pass.
- Live multi-monster target switching and identity reveal remain to be compared
  against the game's own HUD. Binary-derived checks are not that live proof.
