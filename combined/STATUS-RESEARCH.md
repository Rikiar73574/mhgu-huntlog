# MHGU 1.4.0 status event evidence

Offsets relative to the monster status object at enemy+0x1428; code addresses relative to decompressed main. Read only.

- Active status byte +0x5e08: poison bit 2 set at main+0xa24a4, cleared +0xa25b0; sleep bit 1 set +0xa2908; paralysis bit 4 set +0xa2c4c; stun bit 0x10 set +0xa2f54. These routines also clear the corresponding buildup fields and configure timers. Buildup alone is never interpreted as an activation.
- Exhaust proc counter +0x5d8c increments in +0xa3198 (increment +0xa31f0), clamps to 30, raises threshold and dispatches exhaust reaction through +0xa4130. Label as an exhaust proc, not a timed fatigue interval.
- Mount opening counter +0x5da4 increments +0xa3638; mount-end counter +0x5da5 increments +0xa3734. Original MHXX NTR structure documents the latter as completed rides regardless of success. Do not claim successful mount topples. Both counters cap at 60.
- Blast counter +0x5da6 passed by the blast routines at +0xaad7c and +0xaadf0 to +0xa227c; that helper increments the pointed byte at +0xa2294/+0xa22a8 and caps at 30.

Tracker runs each game update, before one-second sample throttling. First seen or reacquired active flags are marked observed, never a start at a known time. Counters are baselined on acquisition and reset. Timing is observed game-frame time; events between observations can be missed. Status fields are absent from older journals and remain explicitly unavailable.

Pure tracker tests cover initial state, transitions, counters, disappearance/reacquisition, resets and separate monster identities. Live hunt comparison remains required after restart.

Counters stop reporting new procs once the game counter saturates (30 for exhaust/blast, 60 for mounting). No fabricated events beyond that cap.
