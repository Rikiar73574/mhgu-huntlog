# Hunt history page

The Windows and Linux native applications in the release include this dashboard.
They open a desktop window and can select a journal from a different save slot.
For a source/browser session, run from the repository root:

```sh
python tools/serve_history.py --port 8768
```

Open http://127.0.0.1:8768. The server binds only to localhost and reads the Forge journal; it never changes the journal or game saves. `--journal /path/to/hunter_hunts.jsonl` overrides the default Wine save path.

The page defaults to closed runs. Search monsters, quest IDs, run IDs or hunters; filter recording status, crown or weapon. Choose a monster, toggle players, switch between DPS/cumulative damage/HP, adjust DPS windows and zoom the time range. Refresh loads subsequent hunts. JSON downloads include the selected run's recorded data.

HP zero implies killed, with `outcome_source: inferred_zero_hp`; explicit outcomes take precedence. Other unavailable fields remain unknown. Headers and hunter table are whole-run totals; graph controls affect the graph. All history stays local. Stop the server when it is no longer needed.

The history defaults to cumulative damage. Quest names, hover descriptions and classic monster/weapon artwork come from Gathering Hall Studios' MHGenDatabase (https://github.com/gatheringhallstudios/MHGenDatabase); license retained in MHGenDatabase-LICENSE.txt. Quest metadata is a local export, so browsing needs no external requests. Internal run IDs remain in JSON downloads, not headings.

Monster sections contain per-monster hunter damage, recorded hit-event count and full-hunt average DPS. Hunter sections contain loadout/item fields when available; old unrecorded data is explicitly unknown. Single-monster runs omit the graph monster selector; only duplicate species need instance suffixes.

Recorder v2 logs verified sampled status-flag transitions and counter increases. Graph events respect the monster selector and visible time range. Mount openings and mount endings do not assert successful topples; exhaust procs do not assert timed fatigue. Initial/reacquired active states are observations, not known start times. Saturated game counters cannot report further procs. Old runs have no status coverage and cannot be backfilled from damage data.

## Desktop applications

Extract the Windows or Linux desktop archive and run HuntLog. Runtime dependencies
and the dashboard are included; no separate Python installation is required.
The native window uses a private loopback listener for the dashboard and stops
it when the app closes. Open Journal selects another file without restarting.

It reads `%APPDATA%\MHGURecomp\save\save_0\hunter_hunts.jsonl` by default.
For a different save slot or portable install, use Open Journal or run
`HuntLog --journal "C:\path\hunter_hunts.jsonl"`.
The recorder mod and Forge must be installed in the game to produce this log.
No personal hunt records are bundled in the applications.

To rebuild on the target OS, install `requirements-desktop.txt` and run
`python tools/build_app.py`. The optional old browser/console launcher remains
available as `python tools/website.py`.
