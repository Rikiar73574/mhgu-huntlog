# HuntLog validation

This standalone project keeps the combined HP HUD, DPS meter, background
recorder and local desktop dashboard together. Builds/tests exercise source
behavior and packaging; they do not by themselves establish in-game results.

The starting installed artifact was SHA-256
`adce11b90a9218ee8e820e76e98b2ec501f4d7570ec4874339de5a59d7379ec1`.
It matched the original combined source's ELF-to-NRO output. The former root
Makefile's C-only 16 KiB HUD was a different, obsolete artifact.

The standalone offline build with source/toolchain path remapping produced
`a00b407d12109848fba8092b88cc1763ed3101e92f44cbe3243068b7c390d462`
on 2026-11-07. Its image is 638,976 bytes, BSS 266,240 bytes, aligned footprint
917,504 bytes. No `/home/` or `/tmp/` strings remain in that NRO. The mod archive
contains the exact build artifact and the exact previously installed font/OFL.
The source has hook-entry preflight, checked installation and reverse rollback;
the hook-policy tests cover unsupported entries and each failed installation.
An independent offline rebuild copied only source into a clean directory with
spaces in its name, copied no target/build outputs, passed all host cases, and
produced that same NRO byte for byte. The build uses encoded Rust flags so
source/toolchain directory spaces are preserved during path remapping.

## Automated scope

- Synthetic Rust memory and filesystem adapters exercise target selection,
  configuration, recorder runs, retries, timestamps, status and equipment data.
- No captured player memory, save files or hunt journals are test fixtures.
- Python checks cover journal parsing, malformed/torn records, equipment/reward
  enrichment, and exact release layout/checksums/privacy rejection.
- JavaScript checks cover the recorded-part presentation.
- Package checks validate NRO headers, source allowlists, required notices,
  deterministic output, workstation-path rejection and plugin memory limits.

The portable host runner passed all eleven suites (159 executed cases, including
shared decoder tests exercised in multiple adapters). The Python non-desktop
tests and JavaScript part-presentation regression passed. Desktop tests are
recorded separately with the app builds.

## Save transaction regression

Version 0.1.0 could crash during a native save: its periodic journal flush used
Forge's shared `savedata` accessor and could commit the game's pending `system`
file concurrently. A disposable copy of the reported save reproduced the exact
`001f4002` abort and missing temporary-file rename on stock Windows 0.14.2.
The debugger widened the native rename window by 5.5 seconds to expose the race;
the two threads committed the same filesystem object. The original save loaded
and completed a native save successfully, so no older save was restored.

HuntLog now uses its own SDK accessor for the same physical save directory.
Reads do not commit or unmount the game's accessor. New files are committed
empty before writing, avoiding the runtime's shared copy buffer. A creation or
creation-commit failure disables further storage access until restart rather
than risking a stale recovery file. Tests execute the real storage module and
model both failure paths. Account readiness is checked before SDK calls; saved
settings and fonts load once the account becomes available.

The final 0.2.0 NRO is SHA-256
`62e24c83b94fdde9b5b3438e3537b0d9e165858906bc4870f90fb81db4b936c7`.
On the same stock Windows build with all twelve mod folders, native `system`
and backup saves completed while journal commits used a separate filesystem
object. The current 127 MB journal retained its prefix, and the existing 836-byte
INI and configured font loaded. A fresh journal also followed empty-create,
commit, reopen, then write. One normal lance hit reduced HP 1032 to 1025,
displayed one Damage Numbers 7, and appended exactly one 7-damage hit record.
These checks used isolated copied saves; all eight protected original files
kept their hashes. Physical Windows hardware remains outside this Wine test.

## Installed-stack static audit

All twelve folders were inventoried. Seven NROs total 11,010,048 bytes after
image+BSS alignment; adding 65,536 bytes loader headroom leaves 1,507,328 bytes
in the 12,582,912-byte region. The new HuntLog footprint is unchanged.

The only duplicate RomFS path is `nativeNX/loc/arc/resident.arc`, supplied by
HD UI v4 and then zz-hd-hunter-font; the later font override wins. The 4K patch
changes main-relative words `BBCD34`, `BBCD38`, `BBCD48`, `BBCD54`.

Installed ARM NRO callsites identify MapAssist hooks at main+`2864C4`,
`19C21C`, `A4FC9C`; Key Quest Markers at `616CB0`, `54D030`; and World
Sharpness Gauge at `A494A0`, `A39300`, `A4BA94`. These do not overlap HuntLog,
Damage Numbers, Armor Fusion or Compendium hook targets. Known shared targets
are HuntLog/Damage Numbers `170968`, `09D6F8` and Armor Fusion/Compendium
`6F1AE8`. This is a static collision inventory, not exhaustive behavior proof.

## Runtime scope and known limitations

### Stench countdown

Congalala's Stench uses group 0 mask `0x800` and the remaining frame timer at
player `+0x28dc`. The native flag builder main+`29f358` calls its active-state
predicate main+`2f5508`; main+`2f54f8` clears that timer on cure. The missing
mapping is covered by a regression test for countdown, refresh, cure, invalid
values and the separate group-2 poison icon sharing the same mask.

All 162 Rust tests pass. Isolated execution also verifies the unmodified native
icon predicate, frame decay, expiry and cure routines. A hidden copied-save
Windows/Wine test with all seven installed plugins shows `1:30` at the Stench
icon and a decreasing remaining counter. No writes or inputs were sent to the
player's running game.

### Elemental countdowns and guest buildup visibility

Elemental-blight icon flags map to the five native remaining-time fields, with
minor/major variants sharing a timer. Regression tests cover all variants,
expiry, refresh and ambiguous multi-effect icons. KO, exhaust and mount retain
valid thresholds at zero; the layout and renderer share the visibility rule.

The Rust suite passes 161 cases, the Python suite 34, and the JavaScript parts
check passes. `tools/verify_timer_arm.py` also executes the unmodified blight
countdown routine and netEmAction receive block against isolated memory,
checking KO/exhaust/mount updates and resets. This is not an end-to-end
connected multiplayer test.

A hidden Windows/Wine instance using a copied save and all seven installed
plugins visibly showed Thunderblight decreasing from 0:30 to 0:10, then expiring,
and KO 0%, EXH 0%, MNT 0% on a selected, discovered monster. The built NRO's
SHA256 is `52f50cbab0908a1e02d35bec59226f2c771d48aa9cdb239ebdca5b8ee6d2135b`.
The plugin stack uses 11,010,048 aligned bytes with 1,507,328 bytes remaining
after loader headroom. Only the disposable test process was controlled and
closed; the player's game was not restarted.

The installed stack has twelve mod folders and seven NROs. Its existing World
Sharpness Gauge stage-21 warning and ImGui mirror-pool warning reproduced with
the original stack without Armor Fusion. The latter originates in the stock
bridge's treatment of the moving vertex-write cursor as a mirrored object.
They are not claimed fixed by packaging HuntLog.

The final standalone NRO and desktop archives require their own launch checks.
Record final artifact hashes and the actual observed cases before publishing
runtime success claims. Live multiplayer attribution, all monster/quest layouts,
capture/kill interpretation and every third-party mod combination are not
exhaustively established. Desktop package checks must be distinguished from
actual Windows/Linux desktop launches.

## Exact Windows installed-stack combat check

Stock Windows MHGURecomp 0.14.2 under Wine 11.20 loaded all 12 installed modfolders and seven NROs with the final HuntLog build. Live entry/trampoline dumps proved game→HuntLog→Damage Numbers→original game, also with those two mods reversed. HuntLog alone used its direct-stock fallback. One real lance thrust changed Arzuros HP 1032→1017; HuntLog counters advanced once, its journal contained exactly one 15-damage hit, and the screenshot shows one Damage Numbers 15.

The monster was placed once ahead of the copied-save hunter after a valid room loaded, using the same placement formula as MHGU_SUMMON. No damage clamp, invulnerability or pinning helper was active. The builtin helper can trigger while room 255 is still loading, so it was not used for the final damage proof. Original saves/settings/executable/mod order remained unchanged and all owned test clients were closed.

World Sharpness Gauge's stage-21 startup warning reproduced with the original stack. Its custom gauge is not claimed verified by this compatibility test. Unknown future mod versions and every possible gameplay interaction remain outside the tested scope.

## Desktop release platform checks

Windows: actual x64 PE built with CPython 3.12.10, Qt 6.11.2 and PyInstaller 6.22.3. All 34 Python/Qt cases passed under Wine 11.20. The packaged executable rendered its dashboard and cumulative graph in a second clean Wine profile without Python installed. Physical Windows hardware was not available. Software rendering and sandbox overrides were confined to the Wine test environment.

Linux: build target is Ubuntu 22.04, Python 3.10.12 and glibc 2.35. The initial bundle passed container checks but exposed a real forward-compatibility problem on the newer host: its bundled C++ runtime lacked a graphics-driver symbol, and its bundled fontconfig rejected the host font configuration. A controlled comparison reproduced the failure only in the bundle; excluding those two system libraries restored normal windowed rendering without test Qt flags. Other libraries remained unchanged. The build helper now applies this policy before checksumming.

The app and parser code are shared between both platforms. Both bundles include their Python/Qt runtime, licenses and per-file checksums. They require no separately installed Python. Keep each extracted application folder together.
