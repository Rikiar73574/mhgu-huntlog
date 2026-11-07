"""Release archives must not absorb saves, memory captures or workstation paths."""
import hashlib
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
RUNTIME_EXPECTED = {
    'README.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md', 'SHA256SUMS',
    'licenses/HuntLog-upstream-MIT.txt', 'licenses/Forge-RS-MIT.txt',
    'licenses/MHGURecomp-MIT.txt', 'licenses/MHGenDatabase-MIT.txt',
    'licenses/Rust-MIT.txt', 'licenses/Rust-Apache-2.0.txt', 'licenses/log-MIT.txt',
    'licenses/compiler-builtins-LICENSE.txt',
    'romfs/nativeNX/forge/fonts/NotoSansSC-Medium.otf',
    'romfs/nativeNX/forge/fonts/OFL.txt',
    'romfs/nativeNX/plugins/hunter_hud.nro',
}


def nro():
    image = bytearray(0x3000)
    struct.pack_into('<I', image, 4, 0x80)
    image[0x10:0x14] = b'NRO0'
    struct.pack_into('<I', image, 0x18, len(image))
    for i in range(3):
        struct.pack_into('<II', image, 0x20 + 8*i, 0x1000*i, 0x1000)
    image[0x80:0x84] = b'MOD0'
    return image


class ReleaseTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = ROOT / 'tools/package.py'
        if path.exists():
            spec = importlib.util.spec_from_file_location('huntlog_package', path)
            cls.package = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(cls.package)

    def test_package_tool_exists(self):
        self.assertTrue((ROOT / 'tools/package.py').is_file())

    def test_runtime_exact_layout_and_hashes(self):
        if not hasattr(self, 'package'):
            self.skipTest('package tool not implemented')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, source in self.package.RUNTIME_FILES.items():
                target = root / source
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(('notice ' + name).encode())
            (root / 'hunter_hunts.jsonl').write_text('PRIVATE JOURNAL')
            output = root / 'runtime.zip'
            self.package.runtime_archive(root, bytes(nro()), output)
            first = output.read_bytes()
            with zipfile.ZipFile(output) as archive:
                expected = RUNTIME_EXPECTED
                self.assertEqual(set(archive.namelist()), expected)
                self.assertEqual(archive.read(self.package.PLUGIN_PATH), nro())
                self.assertNotIn(b'PRIVATE JOURNAL', b''.join(archive.read(n) for n in archive.namelist()))
                hashes = dict(line.split('  ', 1)[::-1] for line in archive.read('SHA256SUMS').decode().splitlines())
                self.assertEqual(set(hashes), expected - {'SHA256SUMS'})
                for name, digest in hashes.items():
                    self.assertEqual(hashlib.sha256(archive.read(name)).hexdigest(), digest)
            self.package.runtime_archive(root, bytes(nro()), output)
            self.assertEqual(first, output.read_bytes())

    def test_rejects_invalid_private_and_over_budget_images(self):
        if not hasattr(self, 'package'):
            self.skipTest('package tool not implemented')
        cases = [b'invalid', nro()[:-1]]
        for offset, value in [(4, 0xffffffff), (0x18, 0), (0x24, 0), (0x2c, 0x1001), (0x80, 0)]:
            image = nro()
            struct.pack_into('<I', image, offset, value)
            cases.append(image)
        for marker in [b'/home/alice/private', b'/tmp/private-build', b'C:\\Users\\Alice\\private']:
            image = nro()
            image[0x100:0x100+len(marker)] = marker
            cases.append(image)
        for image in cases:
            with self.subTest(image=bytes(image)[:24]), self.assertRaises(ValueError):
                self.package.validate_nro(image)
        with self.assertRaises(ValueError):
            self.package.check_budget(bytes(nro()), [bytes(nro())] * 200)

    def test_source_archive_uses_explicit_manifest(self):
        if not hasattr(self, 'package'):
            self.skipTest('package tool not implemented')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root/'README.md').write_text('public')
            (root/'source-files.txt').write_text('README.md\nsource-files.txt\n')
            (root/'hunter_hunts.jsonl').write_text('PRIVATE JOURNAL')
            (root/'postmeal-data.bin').write_bytes(b'PRIVATE GAME MEMORY')
            output = root/'source.zip'
            self.package.source_archive(root, output)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(set(archive.namelist()), {'README.md', 'source-files.txt', 'SHA256SUMS'})
            for entry in ['hunter_hunts.jsonl', '../outside', 'postmeal-data.bin', 'hunter_hud.ini', 'mhgurecomp.ini', 'system', 'backup.zip']:
                if '/' not in entry:
                    (root/entry).write_text('PRIVATE')
                (root/'source-files.txt').write_text(entry+'\n')
                with self.subTest(entry=entry), self.assertRaises(ValueError):
                    self.package.source_archive(root, output)
            (root/'source-files.txt').write_text('README.md\n')
            (root/'README.md').unlink()
            (root/'README.md').symlink_to('/etc/hostname')
            with self.assertRaises(ValueError):
                self.package.source_archive(root, output)


if __name__ == '__main__':
    unittest.main()
