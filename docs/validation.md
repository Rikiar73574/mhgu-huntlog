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

The portable host runner passed all ten suites (156 executed cases, including
shared decoder tests exercised in multiple adapters). The Python non-desktop
tests and JavaScript part-presentation regression passed. Desktop tests are
recorded separately with the app builds.

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
