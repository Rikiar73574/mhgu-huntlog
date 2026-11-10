# Normal Zinogre charge, MHGU 1.4.0

Read-only trace of the owned game image and a live `uEm057_00` actor. Addresses
below are offsets from main unless described as object/status offsets. No game
bytes or snapshots are distributed.

- Species 57 uses vtable main+0x17d19bc. Object+0xcac0 points to its custom
  state; signed 16-bit custom+0 holds charge points. The known scale is 0..100.
- Status is object+0x1428. Status+0x5df3 is the charged-state flag. Getter
  main+0x81790 reads this exact byte.
- Charge entry main+0xeef1a4 writes flag 1; main+0xeef25c initializes points to
  100. Exit main+0xeefca0 clears the flag and normal-Zinogre points.
- Event handler main+0xeefed0 adjusts points by event-dependent amounts. The
  path at main+0xef0798 applies its delta and clamps depletion at zero before
  discharge. Main+0xef0c68 adjusts the same meter for actions, clamping 1..100.
  This is a charge-point meter, not a universal remaining-HP-damage threshold.
- Object+0x7420 == 1 identifies the instance running the monster's AI (also
  used by the recomp's KO trace). Only that authoritative instance exposes
  numeric charge. Remote copies show the local charged-state flag and an
  unavailable value; synchronization of this custom counter is not established.
- Capture requires species, exact vtable, matching status pointer, bounded
  aligned custom pointer, a 0/1 flag and numeric points in range. Other forms,
  including Thunderlord, are not inferred from the normal-species layout.

`tools/verify_charge_arm.py` executes original charge getter, entry/exit,
100-point initialization and depletion instructions in isolated ARM memory.
Rust fixtures cover bad pointers/states, unavailable remote progress, genuine
zero values and unsupported species/builds. Status tracker tests cover start,
end and observed-active transitions, including loss/reacquisition. Journals
sample points/state and put transitions on the existing status timeline.
