HuntLog brings the HP overlay, hunt recorder and hunt-history dashboard together.

- In-game monster HP, damage/DPS, parts and status tracking, configurable through Forge.
- Saved equipment, skills, item events and quest rewards, with the existing hunt-journal format.
- Native Windows and Linux desktop dashboard with graphs, rank/weapon filters, a journal file picker, refresh and zoom. Python and Qt are bundled.
- Damage Numbers remains responsible for floating hit numbers; HuntLog chains the shared collision/damage hooks and records each hit once.

Download **huntlog-mod.zip** for the in-game mod. Extract it into its own mod folder and restart MHGURecomp. Replace an existing HuntLog/Hunter HUD installation rather than enabling a duplicate.

Download **HuntLog-Windows-x64.zip** or **HuntLog-Linux-x64.tar.gz** for the desktop app. Extract the whole folder and run HuntLog.exe on Windows or ./HuntLog on Linux. The app discovers the default native/Wine journal; use File > Open hunt journal for custom saves.

Desktop requirements: Windows 10/11 x64 or Linux x64 with glibc 2.35 or newer. Keep the application folder together. Checksums are in SHA256SUMS; source and license notices accompany the release.

Compatibility coverage and known runtime limitations are recorded in docs/validation.md. No universal compatibility with untested mods or game versions is promised.
