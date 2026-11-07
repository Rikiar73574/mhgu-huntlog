# Source and content provenance

The root MIT license covers authored HuntLog changes. It does not relicense
third-party code, game artwork, game text, or extracted game metadata.
Monster Hunter content belongs to CAPCOM.

- `combined/` adapts sadgnk169's MIT-licensed
  [MHGU-DPS-Meter](https://github.com/sadgnk169/MHGU-DPS-Meter).
  Its original 2026 notice is retained in `combined/LICENSE` and the mod archive.
- Forge Rust SDK is pinned to Lawrence-al/forge-rs commit
  `f2af0622a2a0aa291ffbd3d2f7628c0e2d96d220` in Cargo.toml/Cargo.lock.
  Its MIT notice and linker-template attribution are retained in
  `licenses/Forge-RS-MIT.txt`. Forge itself is supplied by MHGURecomp.
- Rust core/alloc and compiler support are built from the pinned Rust toolchain.
  Its MIT and Apache 2.0 notices are retained in `licenses/Rust-MIT.txt` and
  `licenses/Rust-Apache-2.0.txt`. The linked `log` crate (0.4.35, MIT or Apache
  2.0) has its MIT notice retained in `licenses/log-MIT.txt`.
  Compiler-builtins' combined license/LLVM-exception notices are retained in
  `licenses/compiler-builtins-LICENSE.txt`. Rust notices match compiler commit
  [3e92e1a2d20b991ef23a6c6064c34d57ee8c98d3](https://github.com/rust-lang/rust/tree/3e92e1a2d20b991ef23a6c6064c34d57ee8c98d3).
- MHGURecomp's public layout/ABI references and ELF/NRO conventions informed
  the bridge and converter; its Rokk (2026) MIT notice is retained in
  `licenses/MHGURecomp-MIT.txt`.
- Classic weapon/monster artwork and quest, item, armor and reward reference
  tables originate from Gathering Hall Studios'
  [MHGenDatabase](https://github.com/gatheringhallstudios/MHGenDatabase).
  The upstream DavianCorp (2014) MIT notice is retained in
  `history/MHGenDatabase-LICENSE.txt` and beside the classic PNGs. This records
  the supplied repository license without claiming ownership of CAPCOM art.
- `combined/src/icons.rs` embeds artwork from that classic icon set and the
  upstream DPS meter; the dashboard's `icon-sets.js`/`monster-icons.js` embed
  the corresponding reference images. These are content, not personal records.
- Monster/part names and ordered threshold signatures in `part_names.rs` and
  `history/part-names.*` combine Kiranico references with locally verified
  MHGU 1.4 layouts. Source URLs remain in `part-names.json`. Equipment/skill
  names in `history/player-names.json` and skill-tree labels originate from
  owned English game tables; the export tool retains their origin.
- `assets/fonts/NotoSansSC-Medium.otf` is Noto Sans SC, distributed under SIL
  Open Font License 1.1. Its accompanying `OFL.txt` must travel with the font.
- The desktop application uses PySide6/Qt and Qt WebEngine's Chromium components.
  Supplied LGPL/GPL texts are retained in `licenses/Qt/`; PyInstaller and wheel
  license files found by the app builder are retained under `licenses/desktop/`.
  **Help > About Qt** displays Qt's own license information. **Help > Third-party
  licenses** opens the bundled Qt WebEngine/Chromium copyright and license texts
  in `licenses/Qt-WebEngine/index.html`; this viewer works offline. The collection
  records its exact upstream revisions and source URLs alongside the notices.
  Upstream sources and licensing references:
  [PySide](https://code.qt.io/pyside/pyside-setup.git/),
  [Qt WebEngine](https://code.qt.io/qt/qtwebengine.git/),
  [Qt licensing](https://www.qt.io/licensing/),
  [PyInstaller](https://github.com/pyinstaller/pyinstaller).
- CI desktop builds use CPython 3.12. Its PSF license and bundled third-party
  notices are retained in `licenses/Python-3.12.txt`, copied from the
  [official CPython 3.12 license](https://github.com/python/cpython/blob/3.12/LICENSE).

Release allowlists exclude game executables, extracted game archives, character
saves, personal hunt journals, memory captures, build caches and workstation
configuration. Runtime recording creates `hunter_hunts.jsonl` only in the
selected game's save directory; no journal is bundled in a release.
