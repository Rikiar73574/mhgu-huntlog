"""Exercise the real loopback service, journal selection and shutdown."""
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from urllib.error import HTTPError
from urllib.request import urlopen

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from history_paths import default_journal
from desktop import JournalServer


class DesktopServerTests(unittest.TestCase):
    def test_switch_journal_without_restarting_server_and_close_listener(self):
        with tempfile.TemporaryDirectory() as folder:
            first, second = [Path(folder) / name for name in ('first.jsonl', 'second.jsonl')]
            for path, number in [(first, 7), (second, 9)]:
                path.write_text(json.dumps(dict(schema=1, run=number, event='start', t=0)) + '\n')
            with JournalServer(first) as server:
                self.assertEqual(server.address[0], '127.0.0.1')
                url = server.url + 'api/hunts'
                self.assertEqual(json.load(urlopen(url))['runs'][0]['run'], 7)
                address = server.address
                server.journal = second
                self.assertEqual(json.load(urlopen(url))['runs'][0]['run'], 9)
                self.assertEqual(server.address, address)
                self.assertIn(b'HuntLog', urlopen(server.url).read())
                with self.assertRaises(HTTPError) as missing:
                    urlopen(server.url + 'private.jsonl')
                self.assertEqual(missing.exception.code, 404)
                missing.exception.close()
            self.assertFalse(server.thread.is_alive())
            with self.assertRaises(OSError):
                urlopen(url, timeout=1)

    def test_missing_journal_is_a_recoverable_empty_state(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'missing.jsonl'
            with JournalServer(path) as server:
                with self.assertRaises(HTTPError) as missing:
                    urlopen(server.url + 'api/hunts')
                self.assertEqual(missing.exception.code, 404)
                missing.exception.close()
                self.assertFalse(path.exists(), 'the dashboard must never create or rewrite a game journal')

    def test_discovery_supports_native_linux_and_wine_prefixes(self):
        with tempfile.TemporaryDirectory() as folder:
            home = Path(folder)
            native = home / 'data/MHGURecomp/save/save_0/hunter_hunts.jsonl'
            wine = home / 'prefix/drive_c/users/steamuser/AppData/Roaming/MHGURecomp/save/save_0/hunter_hunts.jsonl'
            env = {'XDG_DATA_HOME': str(home / 'data'), 'WINEPREFIX': str(home / 'prefix')}
            self.assertEqual(default_journal(home=home, environ=env, platform='posix'), native)
            wine.parent.mkdir(parents=True)
            wine.write_text('')
            self.assertEqual(default_journal(home=home, environ=env, platform='posix'), wine)
            native.parent.mkdir(parents=True)
            native.write_text('')
            os.utime(native, (2000000000, 2000000000))
            self.assertEqual(default_journal(home=home, environ=env, platform='posix'), native)

    def test_discovery_supports_windows_appdata_and_save_override(self):
        with tempfile.TemporaryDirectory() as folder:
            home = Path(folder)
            self.assertEqual(default_journal(home=home, environ={'APPDATA': str(home / 'roaming')}, platform='nt'), home / 'roaming/MHGURecomp/save/save_0/hunter_hunts.jsonl')
            self.assertEqual(default_journal(home=home, environ={'MHGU_SAVE_DIR': str(home / 'save')}, platform='nt'), home / 'save/save_0/hunter_hunts.jsonl')

if __name__ == '__main__':
    unittest.main()
