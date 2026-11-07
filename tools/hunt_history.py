#!/usr/bin/env python3
"""Render Forge schema-1 JSONL hunts as an offline HTML report.

Usage: python3 hunt_history.py --input forge-hunts.jsonl --output hunts.html
       [--json hunts.json]

Damage rates use consecutive five-second timestamp windows, with the last
window divided by its actual observed duration. They are not lifetime DPS.
An end record means recording stopped, not that a quest or monster was won.
"""
import argparse
import html
import json
import math
from pathlib import Path
import sys

BIN_SECONDS = 5.0
COLORS = ("#60a5fa", "#fbbf24", "#f472b6", "#34d399", "#a78bfa", "#fb923c")


def integer(value):
    return type(value) is int


def number(value):
    return type(value) in (int, float) and math.isfinite(value)


def validate(row):
    if not isinstance(row, dict) or row.get("schema") != 1:
        raise ValueError("unsupported schema or non-object record")
    if not integer(row.get("run")) or not number(row.get("t")) or row["t"] < 0:
        raise ValueError("invalid run or timestamp")
    event = row.get("event")
    if event == "part_transition":
        if not integer(row.get("monster")) or row["monster"]<1 or not integer(row.get("slot")) or not 0<=row["slot"]<8 or row.get("kind") not in ("flinch","sever","secondary") or not integer(row.get("count")) or not 0<=row["count"]<=14:
            raise ValueError("invalid part transition")
        return

    if event == "sample":
        for field in ("players", "monsters"):
            if not isinstance(row.get(field), list):
                raise ValueError("invalid sample list")
            for item in row[field]:
                if not isinstance(item, dict) or not integer(item.get("id")) or item["id"] < 0:
                    raise ValueError("invalid entity ID")
    elif event == "hit":
        if not integer(row.get("player")) or row["player"] < 0:
            raise ValueError("invalid hit entity ID")
        if "monster" not in row or (row["monster"] is not None and
                (not integer(row["monster"]) or row["monster"] < 0)):
            raise ValueError("invalid hit monster ID")
        if not integer(row.get("damage")) or row["damage"] <= 0:
            raise ValueError("invalid hit damage")
    elif event == "part_break":
        if not integer(row.get("monster")) or row["monster"]<1 or not integer(row.get("group")) or not 0<=row["group"]<8 or row.get("phase") not in ("broken","observed"):
            raise ValueError("invalid part break")
    elif event == "item":
        if not integer(row.get("player")) or not 1 <= row["player"] <= 4:
            raise ValueError("invalid item player")
        if not integer(row.get("item_id")) or not 0 < row["item_id"] < 20000 or not integer(row.get("quantity")) or not 0 < abs(row["quantity"]) <= 999:
            raise ValueError("invalid item mutation")
        if row.get("action") not in ("used", "gathered", "acquired_unknown", "removed_unknown", "action_consumed"):
            raise ValueError("invalid item action")
        if (row["quantity"] > 0) != (row["action"] in ("gathered", "acquired_unknown")):
            raise ValueError("item action direction mismatch")
    elif event == "loot":
        if row.get("source") != "quest_reward" or row.get("monster") is not None:
            raise ValueError("invalid loot source")
        for key, limit in (("group",4),("slot",48)):
            if not integer(row.get(key)) or not 0 <= row[key] < limit:
                raise ValueError("invalid reward position")
        if not integer(row.get("item_id")) or not 0 < row["item_id"] < 20000 or not integer(row.get("quantity")) or not 0 < row["quantity"] <= 999:
            raise ValueError("invalid reward item")
    elif event == "status":
        if not integer(row.get("monster")) or row["monster"] < 1:
            raise ValueError("invalid status monster")
        if row.get("kind") not in ("sleep", "poison", "paralysis", "stun", "exhaust_proc", "mount_opening", "mount_ended", "blast") or row.get("phase") not in ("start", "end", "observed", "trigger"):
            raise ValueError("invalid status signal")
        if not integer(row.get("count")) or not 1 <= row["count"] <= 255:
            raise ValueError("invalid status count")
    elif event not in ("start", "end"):
        raise ValueError("unknown event")


def new_run(run_id, timestamp):
    return dict(run=run_id, quest_id=None, timer_source=None, partial=True,
                complete=False, part_breaks=[], item_events=[], loot=[], status_tracking=False, status_events=[], outcome=None, reason=None, started_at=timestamp,
                last_at=timestamp, completed_at=None, recorder_player_id=None, recorder_hr=None, monsters={}, players={}, hits=[], samples=[], has_start=False, unattributed_hits=[])


def monster(run, identity):
    return run["monsters"].setdefault(identity, dict(id=identity, species=None, name=None, hp=None,
        max_hp=None, size=None, size_scale=None, crown=None, outcome=None, damage=0))


def player(run, identity):
    return run["players"].setdefault(identity, dict(id=identity, slot=None, name=None,
                                                 weapon_type=None, weapon_id=None, damage=0))


def read_journal(lines):
    """Read an iterable of text lines; preserve run IDs and count skipped records."""
    runs, warnings = {}, []
    for lineno, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line, parse_constant=lambda value: (_ for _ in ()).throw(ValueError("nonfinite number")))
            validate(row)
            run_id, event, timestamp = row["run"], row["event"], row["t"]
            existing = runs.get(run_id)
            if existing and (existing["complete"] or timestamp < existing["last_at"]):
                raise ValueError("event after end or timestamp moved backwards")
            if existing and event == "start":
                raise ValueError("duplicate or late start for run ID")
            run = runs.setdefault(run_id, new_run(run_id, timestamp))
            if not existing and event != "start":
                warnings.append(f"Line {lineno}: run {run_id} has no start record; marked partial")
            run["last_at"] = timestamp
            dropped = row.get("dropped_events")
            if event in ("sample", "end") and integer(dropped) and dropped > 0:
                warnings.append(f"Line {lineno}: run {run_id} reports {dropped} dropped events; recorded damage may be incomplete")
            if event == "start":
                run.update(quest_id=row.get("quest_id"), timer_source=row.get("timer_source"),
                           partial=row.get("partial", True), has_start=True, status_tracking=row.get("status_tracking") is True, recorder_player_id=row.get("recorder_player_id"), recorder_hr=row.get("recorder_hr"))
            elif event == "sample":
                for item in row["monsters"]:
                    target = monster(run, item["id"])
                    for field in ("species", "name", "hp", "max_hp", "size", "size_scale", "crown", "outcome", "parts", "break_mask"):
                        if field in item:
                            target[field] = item[field]
                for item in row["players"]:
                    target = player(run, item["id"])
                    for field in ("slot", "name", "weapon_type", "weapon_id", "hr", "is_local", "equipment", "stats", "skills", "food_skills"):
                        if field in item:
                            target[field] = item[field]
                run["samples"].append({k: v for k, v in row.items() if k not in ("schema", "run", "event")})
            elif event == "part_transition":
                run.setdefault("part_events", []).append({k:v for k,v in row.items() if k not in ("schema","run","event")})
            elif event == "part_break":
                run["part_breaks"].append({k:v for k,v in row.items() if k not in ("schema","run","event")})
            elif event == "item":
                player(run, row["player"])
                run["item_events"].append({k:v for k,v in row.items() if k not in ("schema","run","event")})
            elif event == "loot":
                if not any(x["group"] == row["group"] and x["slot"] == row["slot"] for x in run["loot"]):
                    run["loot"].append({k:v for k,v in row.items() if k not in ("schema","run","event")})
            elif event == "status":
                run["status_events"].append({k:v for k,v in row.items() if k not in ("schema","run","event")})
            elif event == "hit":
                if row["monster"] is None:
                    run["unattributed_hits"].append(row)
                    player(run, row["player"])
                    continue
                player(run, row["player"])["damage"] += row["damage"]
                if row["monster"] is not None:
                    monster(run, row["monster"])["damage"] += row["damage"]
                run["hits"].append(dict(t=timestamp, player=row["player"], monster=row["monster"], damage=row["damage"], part=row.get("part") if integer(row.get("part")) and 0<=row["part"]<8 else None))
            else:
                run.update(complete=True, reason=row.get("reason"), outcome=row.get("outcome"), completed_at=row.get("completed_at"))
        except (ValueError, TypeError) as exc:
            warnings.append(f"Line {lineno}: ignored record ({exc})")

    for run in runs.values():
        duration = run["last_at"] - run["started_at"]
        run["duration_seconds"] = duration
        count = max(1, math.ceil(duration / BIN_SECONDS))
        widths = [min(BIN_SECONDS, max(0, duration - i * BIN_SECONDS)) for i in range(count)]
        run["bin_durations"] = widths
        run["bin_starts"] = [i * BIN_SECONDS for i in range(count)]
        for entry in run["players"].values():
            entry["damage_bins"] = [0] * count
            entry["hit_count"] = sum(h["player"] == entry["id"] for h in run["hits"])
        for hit in run["hits"]:
            index = min(count - 1, int((hit["t"] - run["started_at"]) / BIN_SECONDS))
            run["players"][hit["player"]]["damage_bins"][index] += hit["damage"]
        for entry in run["players"].values():
            entry["dps_bins"] = [damage / width if width else None for damage, width in zip(entry["damage_bins"], widths)]
            entry["average_dps"] = entry["damage"] / duration if duration else None
        for target in run["monsters"].values():
            if target["outcome"] is None and target["hp"] == 0:
                target["outcome"] = "killed"
                target["outcome_source"] = "inferred_zero_hp"
            target["players"] = []
            for entry in run["players"].values():
                bins = [0] * count
                for hit in run["hits"]:
                    if hit["monster"] == target["id"] and hit["player"] == entry["id"]:
                        index = min(count - 1, int((hit["t"] - run["started_at"]) / BIN_SECONDS))
                        bins[index] += hit["damage"]
                damage = sum(bins)
                target["players"].append(dict(entry, damage=damage, hit_count=sum(h["player"] == entry["id"] and h["monster"] == target["id"] for h in run["hits"]), damage_bins=bins,
                    dps_bins=[value / width if width else None for value, width in zip(bins, widths)],
                    average_dps=damage / duration if duration else None))
        run["players"] = list(run["players"].values())
        run["monsters"] = list(run["monsters"].values())
    return dict(schema=1, bin_seconds=BIN_SECONDS, warning_count=len(warnings), warnings=warnings, runs=list(runs.values()))


def label(value):
    return html.escape("Unknown" if value is None else str(value), quote=True)


def chart(run):
    """SVG uses horizontal steps: each rate applies to its entire time window."""
    duration = max(run["duration_seconds"], 1)
    maximum = max([1] + [v for p in run["players"] for v in p["dps_bins"] if v is not None])
    maximum *= 1.1
    parts = ['<svg viewBox="0 0 900 300" role="img" aria-label="Per-player damage per second over observed time">']
    for fraction in (0, 0.25, 0.5, 0.75, 1):
        y = 250 - fraction * 220
        parts.append(f'<path class="grid" d="M65 {y} H875"/><text x="56" y="{y + 4}" text-anchor="end">{maximum * fraction:.1f}</text>')
        x = 65 + fraction * 810
        parts.append(f'<text x="{x}" y="273" text-anchor="middle">{run["duration_seconds"] * fraction:.1f}s</text>')
    parts.append('<text x="65" y="17">Damage / second</text>')
    for index, entry in enumerate(run["players"]):
        points = []
        for start, width, dps in zip(run["bin_starts"], run["bin_durations"], entry["dps_bins"]):
            if dps is None:
                continue
            y = 250 - dps / maximum * 220
            points.extend((f'{65 + start / duration * 810:.2f},{y:.2f}', f'{65 + (start + width) / duration * 810:.2f},{y:.2f}'))
        parts.append(f'<polyline fill="none" stroke="{COLORS[index % len(COLORS)]}" stroke-width="2.5" points="{" ".join(points)}"><title>{label(entry["name"])} — {entry["damage"]} damage</title></polyline>')
    parts.append('</svg><ul class="legend">')
    for index, entry in enumerate(run["players"]):
        rate = "Unknown" if entry["average_dps"] is None else f'{entry["average_dps"]:.2f}'
        parts.append(f'<li style="border-color:{COLORS[index % len(COLORS)]}">{label(entry["name"])} · slot {label(entry["slot"])} · weapon {label(entry["weapon_type"])} · {entry["damage"]} damage · {rate} average DPS</li>')
    parts.append('</ul>')
    return "".join(parts)


def render_html(report):
    parts = ['<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Forge hunt history</title>',
             '<style>body{font:16px system-ui,sans-serif;background:#111827;color:#e5e7eb;max-width:1100px;margin:auto;padding:24px}article{background:#1f2937;border:1px solid #374151;border-radius:12px;padding:20px;margin:24px 0}h1,h2{color:#fff}p{line-height:1.5}svg{width:100%;height:auto}svg text{fill:#cbd5e1;font-size:13px}.grid{stroke:#374151;stroke-width:1}.legend{padding:0;list-style:none;display:flex;gap:12px;flex-wrap:wrap}.legend li{border-left:4px solid;padding:4px 8px}table{width:100%;border-collapse:collapse;font-size:14px}th,td{text-align:left;border-bottom:1px solid #374151;padding:8px}.scroll{overflow:auto}.muted{color:#cbd5e1}</style>',
             '<h1>Forge hunt history</h1><p>Per-player DPS in consecutive 5-second windows, using recorded timestamps. The final window uses its actual duration. Idle players remain in the legend. Durations and totals cover only the observed portion of partial or incomplete runs.</p>',
             f'<p>{len(report["runs"])} runs · {report["warning_count"]} journal warnings</p>']
    if report["warnings"]:
        parts.append('<details><summary>Journal warnings</summary><ul>' + ''.join(f'<li>{label(w)}</li>' for w in report["warnings"]) + '</ul></details>')
    for run in report["runs"]:
        state = "Closed" if run["complete"] else "Incomplete — no end record"
        if run["partial"]:
            state += " · Partial recording"
        parts.append(f'<article><h2>Run {run["run"]} · Quest {label(run["quest_id"])}</h2><p>{state}<br>Observed duration: {run["duration_seconds"]:.2f}s · Timer source: {label(run["timer_source"])}<br>Outcome: {label(run["outcome"])} · End reason: {label(run["reason"])}</p>')
        parts.append('<h3>All large monsters</h3>' + chart(run))
        if run["unattributed_hits"]:
            parts.append(f'<p>{len(run["unattributed_hits"])} legacy hits without a verified large-monster target excluded from DPS.</p>')
        for target in run["monsters"]:
            parts.append(f'<details><summary>DPS · {label(target["name"] or target["species"])} · monster {target["id"]}</summary>')
            parts.append(chart(dict(run, players=target["players"])))
            parts.append('</details>')
        parts.append('<h3>Monsters</h3><div class="scroll"><table><thead><tr><th>ID</th><th>Species</th><th>Last HP / max</th><th>Size</th><th>Size scale</th><th>Crown</th><th>Outcome</th><th>Recorded damage</th></tr></thead><tbody>')
        for entry in run["monsters"]:
            hp = 'Unknown' if entry['hp'] is None else str(entry['hp'])
            max_hp = 'Unknown' if entry['max_hp'] is None else str(entry['max_hp'])
            parts.append('<tr>' + ''.join(f'<td>{label(v)}</td>' for v in (entry["id"], entry["name"] or entry["species"], f'{hp} / {max_hp}', entry["size"], entry["size_scale"], entry["crown"], entry["outcome"], entry["damage"])) + '</tr>')
        parts.append('</tbody></table></div><p class="muted">Monster IDs distinguish separate monsters of the same species. Killed is inferred when the last recorded HP is zero; capture requires a separate signal.</p></article>')
    parts.append('</html>')
    return ''.join(parts)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--input', required=True, type=Path, help='Forge JSONL journal')
    parser.add_argument('--output', required=True, type=Path, help='Standalone HTML report')
    parser.add_argument('--json', type=Path, help='Optional machine-readable summary and timeline')
    args = parser.parse_args(argv)
    paths = [args.input.resolve(), args.output.resolve()] + ([args.json.resolve()] if args.json else [])
    if len(paths) != len(set(paths)):
        parser.error('input and output paths must be different')
    try:
        with args.input.open(encoding='utf-8', errors='replace') as source:
            report = read_journal(source)
        args.output.write_text(render_html(report), encoding='utf-8')
        if args.json:
            args.json.write_text(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False) + '\n', encoding='utf-8')
    except (OSError, ValueError) as exc:
        parser.exit(1, f'Error: {exc}\n')
    print(f'Wrote {len(report["runs"])} runs to {args.output}; {report["warning_count"]} warnings', file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())
