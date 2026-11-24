HuntLog v0.2.6 includes the multiplayer HUD synchronization previously available
only in the source repository. The v0.2.5 download did not include it.

- Updated peers share live KO, exhaust and mount buildup from the monster's current AI owner.
- Shared damage totals and elapsed time use a common HuntLog peer snapshot.
- Missing or stale owner reports hide the three buildup rows instead of showing misleading zeros.
- Combat, quest state and recorded hit accounting remain local and unchanged.

Both players must install this release and restart MHGURecomp. Buildup rows
appear for the selected, discovered monster. An AI owner without this version
cannot supply live buildup values.

Verified with two isolated game instances in a real local-play quest, including
controlled host buildup changes, guest HUD updates, report expiry and recovery.
Online lobby services and emulator rooms have not been verified in this test.
Details and test limitations are in docs/validation.md.

Download **huntlog-mod.zip** or run MHGU Mod Updater, then restart the game.
Replace the existing HuntLog installation; do not enable a second copy.
The Windows and Linux desktop dashboards are unchanged from v0.2.5.
