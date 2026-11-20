# MHGU 1.4.0 status event evidence

Offsets relative to the monster status object at enemy+0x1428; code addresses relative to decompressed main. Read only.

- Active status byte +0x5e08: poison bit 2 set at main+0xa24a4, cleared +0xa25b0; sleep bit 1 set +0xa2908; paralysis bit 4 set +0xa2c4c; stun bit 0x10 set +0xa2f54. These routines also clear the corresponding buildup fields and configure timers. Buildup alone is never interpreted as an activation.
- Exhaust proc counter +0x5d8c increments in +0xa3198 (increment +0xa31f0), clamps to 30, raises threshold and dispatches exhaust reaction through +0xa4130. Label as an exhaust proc, not a timed fatigue interval.
- Mount opening counter +0x5da4 increments +0xa3638; mount-end counter +0x5da5 increments +0xa3734. Original MHXX NTR structure documents the latter as completed rides regardless of success. Do not claim successful mount topples. Both counters cap at 60.
- Blast counter +0x5da6 passed by the blast routines at +0xaad7c and +0xaadf0 to +0xa227c; that helper increments the pointed byte at +0xa2294/+0xa22a8 and caps at 30.

Tracker runs each game update, before one-second sample throttling. First seen or reacquired active flags are marked observed, never a start at a known time. Counters are baselined on acquisition and reset. Timing is observed game-frame time; events between observations can be missed. Status fields are absent from older journals and remain explicitly unavailable.

Pure tracker tests cover initial state, transitions, counters, disappearance/reacquisition, resets and separate monster identities. Live hunt comparison remains required after restart.

Counters stop reporting new procs once the game counter saturates (30 for exhaust/blast, 60 for mounting). No fabricated events beyond that cap.

## Guest buildup correction and HuntLog sharing

Ordinary `netEmAction` uses runtime send `0x2cb330` / receive `0x2cc154`.
It does not copy KO `+0x5d76`, exhaust `+0x5d82`, or mount `+0x5d9a`.
The previously tested copy at `0x2cca0c..0x2cca58` belongs to the separate
full-state receive routine `0x2cc86c`; corresponding sends occur around
`0x2cbb74`. Passing that isolated block did not establish regular replication.
A read-only selected, discovered guest Nargacuga snapshot had valid thresholds
220 / 216 / 72 but remained at zero throughout 20 seconds of HP loss.

The HUD now accepts bounded, expiring HuntLog status reports only from the
monster's current AI owner, matched by quest, network ID and species. It never
writes those values into the game's monster state. Without a fresh owner report,
the guest rows are hidden. Owner-local readings keep their existing behavior.

Party damage, hit totals and elapsed meter time are published together. The
lowest present HuntLog player slot supplies the shared HUD reference, including
its own published snapshot, so peers use the same denominator and clock. Receipt
replaces the displayed snapshot and does not add hits or alter local journals.
The channel uses Forge's virtual local-play UDP sockets on port 52689, with
bounded nonblocking receive work, no external host-network listener, and no
initialization or shutdown of the game's socket subsystem.
