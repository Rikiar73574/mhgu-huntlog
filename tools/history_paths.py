"""Resource lookup and native/Wine save discovery for source and bundled apps."""
import os
import sys
from pathlib import Path

HISTORY = (Path(sys._MEIPASS) if getattr(sys, 'frozen', False) else Path(__file__).resolve().parents[1]) / 'history'


def default_journal(*, home=None, environ=None, platform=None):
    home = Path.home() if home is None else Path(home)
    env = os.environ if environ is None else environ
    platform = os.name if platform is None else platform
    relative = Path('MHGURecomp/save/save_0/hunter_hunts.jsonl')
    if env.get('MHGU_SAVE_DIR'):
        return Path(env['MHGU_SAVE_DIR']) / 'save_0/hunter_hunts.jsonl'
    if platform == 'nt':
        return Path(env.get('APPDATA', home / 'AppData/Roaming')) / relative
    native = Path(env.get('XDG_DATA_HOME', home / '.local/share')) / relative
    candidates = [native]
    users = Path(env.get('WINEPREFIX', home / '.wine')) / 'drive_c/users'
    if users.is_dir():
        candidates.extend(user / 'AppData/Roaming' / relative for user in users.iterdir() if user.is_dir())
    existing = [path for path in candidates if path.is_file()]
    return max(existing, key=lambda path: path.stat().st_mtime_ns) if existing else native
