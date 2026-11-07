#!/usr/bin/env python3
"""Create deterministic HuntLog mod and source archives from explicit allowlists."""
import argparse
import hashlib
from pathlib import Path, PurePosixPath
import re
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PLUGIN_PATH = 'romfs/nativeNX/plugins/hunter_hud.nro'
BUDGET = 12 * 1024 * 1024
ALIGNMENT = 0x10000
RUNTIME_FILES = {
    'README.md': 'README.md',
    'LICENSE': 'LICENSE',
    'THIRD_PARTY_NOTICES.md': 'THIRD_PARTY_NOTICES.md',
    'licenses/HuntLog-upstream-MIT.txt': 'combined/LICENSE',
    'licenses/Forge-RS-MIT.txt': 'licenses/Forge-RS-MIT.txt',
    'licenses/MHGURecomp-MIT.txt': 'licenses/MHGURecomp-MIT.txt',
    'licenses/Rust-MIT.txt': 'licenses/Rust-MIT.txt',
    'licenses/Rust-Apache-2.0.txt': 'licenses/Rust-Apache-2.0.txt',
    'licenses/log-MIT.txt': 'licenses/log-MIT.txt',
    'licenses/compiler-builtins-LICENSE.txt': 'licenses/compiler-builtins-LICENSE.txt',
    'licenses/MHGenDatabase-MIT.txt': 'history/MHGenDatabase-LICENSE.txt',
    'romfs/nativeNX/forge/fonts/NotoSansSC-Medium.otf': 'assets/fonts/NotoSansSC-Medium.otf',
    'romfs/nativeNX/forge/fonts/OFL.txt': 'assets/fonts/OFL.txt',
}


def validate_nro(image, check_privacy=True):
    if len(image) < 0x80 or image[0x10:0x14] != b'NRO0':
        raise ValueError('expected an NRO image')
    if struct.unpack_from('<I', image, 0x18)[0] != len(image):
        raise ValueError('NRO size does not match header')
    end = 0
    for offset in (0x20, 0x28, 0x30):
        start, size = struct.unpack_from('<II', image, offset)
        if start != end or not size or start % 0x1000 or size % 0x1000:
            raise ValueError('NRO segments must be contiguous and page aligned')
        end = start + size
    mod = struct.unpack_from('<I', image, 4)[0]
    if end != len(image) or mod < 0x80 or mod + 28 > len(image) or image[mod:mod+4] != b'MOD0':
        raise ValueError('NRO segments or MOD0 are invalid')
    if check_privacy and re.search(rb'/home/|/tmp/|/Users/|/root/|[A-Za-z]:[\\/]Users[\\/]', image):
        raise ValueError('NRO contains a workstation path; rebuild with source remapping')
    bss = struct.unpack_from('<I', image, 0x38)[0]
    return (len(image) + bss + ALIGNMENT - 1) & ~(ALIGNMENT - 1)


def check_budget(image, others=()):
    used = validate_nro(image) + sum(validate_nro(other, check_privacy=False) for other in others)
    if used + ALIGNMENT > BUDGET:
        raise ValueError('plugins plus loader headroom exceed the shared 12 MiB region')
    return used


def archive(files, output):
    files = dict(files)
    files['SHA256SUMS'] = ''.join(
        f'{hashlib.sha256(data).hexdigest()}  {name}\n'
        for name, data in sorted(files.items())
    ).encode()
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as result:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            if name.endswith('.sh'):
                info.external_attr = 0o100755 << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            result.writestr(info, data, compresslevel=9)


def runtime_archive(root, image, output, others=()):
    check_budget(image, others)
    files = {name: (root/source).read_bytes() for name, source in RUNTIME_FILES.items()}
    files[PLUGIN_PATH] = image
    archive(files, output)


def source_archive(root, output):
    paths = (root/'source-files.txt').read_text().splitlines()
    files = {}
    for name in paths:
        if not name or name.startswith('#'):
            continue
        path = PurePosixPath(name)
        if (path.is_absolute() or '..' in path.parts or str(path) != name
                or any(part in {'build', 'target', 'dist', '.git', '__pycache__', '.wine'} for part in path.parts)
                or path.suffix in {'.bin', '.jsonl', '.log', '.nro', '.so', '.elf', '.exe', '.db', '.sqlite', '.pyc', '.zip', '.gz', '.tar', '.7z'}
                or path.suffix == '.ini' and name != 'combined/conf/mhgu_dps_meter.ini'
                or path.name in {'system', 'system_backup', 'compendium_wishlist.txt'}
                or name in files):
            raise ValueError(f'unsafe source manifest entry: {name}')
        source = root/name
        if not source.is_file() or source.is_symlink() or any(p.is_symlink() for p in source.parents if p != root):
            raise ValueError(f'missing or symlinked source file: {name}')
        files[name] = source.read_bytes()
    archive(files, output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', action='store_true', help='export source-files.txt instead of the runtime mod')
    parser.add_argument('--nro', type=Path, default=ROOT/'build/hunter_hud.nro')
    parser.add_argument('--other-plugin', type=Path, action='append', default=[])
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    output = args.output or ROOT/'dist'/('huntlog-source.zip' if args.source else 'huntlog-mod.zip')
    try:
        if args.source:
            source_archive(ROOT, output)
        else:
            runtime_archive(ROOT, args.nro.read_bytes(), output, [p.read_bytes() for p in args.other_plugin])
    except (OSError, ValueError) as error:
        parser.exit(1, f'package: {error}\n')
    print(f'{hashlib.sha256(output.read_bytes()).hexdigest()}  {output.name}')


if __name__ == '__main__':
    main()
