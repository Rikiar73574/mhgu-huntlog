#!/usr/bin/env python3
"""Build the native self-contained dashboard on Windows or Linux."""
import argparse
import hashlib
import importlib.metadata
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def build():
    command = [sys.executable, '-m', 'PyInstaller', '--noconfirm', '--clean', '--windowed',
        '--name', 'HuntLog', '--paths', str(ROOT / 'tools'),
        '--add-data', f'{ROOT / "history"}:history',
        '--distpath', str(ROOT / 'build/desktop'),
        '--workpath', str(ROOT / 'build/desktop-work'),
        '--specpath', str(ROOT / 'build'), str(ROOT / 'tools/desktop.py')]
    subprocess.run(command, cwd=ROOT, check=True)
    folder = ROOT / 'build/desktop/HuntLog'
    if sys.platform != 'win32':
        # Host graphics drivers and font configuration need their host libraries.
        for name in ('libstdc++.so.6', 'libfontconfig.so.1'):
            (folder / '_internal' / name).unlink(missing_ok=True)
    for name in ['README.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md']:
        shutil.copy2(ROOT / name, folder / name)
    shutil.copytree(ROOT / 'licenses', folder / 'licenses', dirs_exist_ok=True)
    version = f'{sys.version_info.major}.{sys.version_info.minor}'
    for path in [Path(sys.base_prefix) / 'LICENSE.txt', Path(sys.base_prefix) / 'LICENSE',
                 Path(f'/usr/share/doc/python{version}/copyright'), Path('/usr/share/licenses/python/LICENSE')]:
        if path.is_file():
            shutil.copy2(path, folder / 'licenses' / f'Python-{version}-{path.name}.txt')
    # Retain license texts shipped by dependencies, alongside their native files.
    for package in ['pyinstaller', 'PySide6', 'PySide6_Essentials', 'PySide6_Addons', 'shiboken6']:
        distribution = importlib.metadata.distribution(package)
        for file in distribution.files or []:
            if any(part.lower() in ('licenses', 'license', 'copying') for part in file.parts):
                source = Path(distribution.locate_file(file))
                if source.is_file():
                    target = folder / 'licenses/desktop' / package / file
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(source, target)
    if sys.platform != 'win32':
        shutil.copy2(ROOT / 'history/huntlog.svg', folder / 'huntlog.svg')
    hashes = ''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(folder).as_posix()}\n'
        for p in sorted(folder.rglob('*')) if p.is_file())
    (folder / 'SHA256SUMS').write_text(hashes)
    return folder


def archive(folder):
    output = ROOT / 'dist'
    output.mkdir(exist_ok=True)
    if sys.platform == 'win32':
        target = output / 'HuntLog-Windows-x64.zip'
        with zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as package:
            for path in sorted(folder.rglob('*')):
                if path.is_file():
                    package.write(path, 'HuntLog/' + path.relative_to(folder).as_posix())
    else:
        target = output / 'HuntLog-Linux-x64.tar.gz'
        with tarfile.open(target, 'w:gz') as package:
            package.add(folder, arcname='HuntLog')
    print(f'{target}: {hashlib.sha256(target.read_bytes()).hexdigest()}')
    return target


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive-only', action='store_true')
    args = parser.parse_args()
    archive(ROOT / 'build/desktop/HuntLog' if args.archive_only else build())
