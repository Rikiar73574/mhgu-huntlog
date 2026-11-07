#!/usr/bin/env bash
set -euo pipefail

huntlog_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
huntlog_url='http://127.0.0.1:8768/#hunts'

website_ready() {
    python3 - <<'PY' >/dev/null 2>&1
from urllib.request import urlopen
with urlopen('http://127.0.0.1:8768/', timeout=1) as response:
    assert response.status == 200
PY
}

if ! website_ready; then
    huntlog_state="${XDG_STATE_HOME:-$HOME/.local/state}/HuntLog"
    mkdir -p -- "$huntlog_state"
    nohup python3 "$huntlog_dir/tools/serve_history.py" --port 8768 \
        >"$huntlog_state/website.log" 2>&1 </dev/null &
    for attempt in {1..30}; do
        if website_ready; then break; fi
        sleep 0.2
    done
    if ! website_ready; then
        printf 'HuntLog could not start. Check %s\n' "$huntlog_state/website.log" >&2
        exit 1
    fi
fi

xdg-open "$huntlog_url" >/dev/null 2>&1 &
