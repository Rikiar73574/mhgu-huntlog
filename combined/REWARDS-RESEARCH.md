# Automatic quest reward capture (MHGU 1.4.0)

Static code trace, 2026-11-03; live reward-screen comparison still required after restart.

- uUIResult RTTI name at main+0x1563357, registration +0x391400.
- Its reward page routine +0x386c6c takes (object, page). First instructions are
  E92D48F0 E24DD018; hook is separately gated on both words.
- Four item vectors at object+0x5c/0x64/0x6c/0x74: pointer then count.
  Call sites +0x386d6c/+0x386de8/+0x386e64/+0x386f28 pass these to
  cUIOResultRem +0x623c08, which copies u16 item ID and u16 quantity,
  stride four, into display slots +0x60. Display capacity is 48 slots.
- +0x623cb4 reads an item ID and calls the item-definition lookup +0x194228.
  +0x6236cc reads its signed quantity. The logger rejects negative/sentinel values.
- Hook captures all four lists before the original page function. The recorder
  retains the first nonempty value per group/slot for each run. Repeated page
  draws, taking items and selling do not add duplicate awards. A new run resets it.
- These are rewards offered, not a receipt proving every item was accepted.
  Group meanings, specific monster, break/capture provenance and exact award
  time are intentionally not inferred. The event time is the current hunt timer.
- Hook buffers JSON only; normal update performs disk writes. It does nothing
  without an active run. Existing explicit-town boundary flushes these events.
- Old hunts cannot gain automatic drops retrospectively. Manual test observations
  remain in build/player-baseline, excluded from the history API.
- Carve/gather/item-use action hooks are separate, not implemented by this reward hook.

Validation: synthetic memory decoder tests; duplicate/consumption/new-run tests;
real runtime journal test includes disk failure/retry and rewards before town;
Python reader validates and preserves source, slots and item quantities.
