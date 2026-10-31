#!/usr/bin/env python3
"""Acquire the pinned PHP engine and replay its verified local patch series."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--pin', type=Path, default=ROOT / 'engine/source.json', help='Source manifest; defaults to the active engine pin')
args = parser.parse_args()
PIN = json.loads(args.pin.read_text())
PATCHES = [ROOT / 'engine/patches' / item['file'] for item in PIN['patches']]
for entry, patch in zip(PIN['patches'], PATCHES):
    if hashlib.sha256(patch.read_bytes()).hexdigest() != entry['sha256']:
        raise RuntimeError(f'Patch checksum mismatch: {patch.name}')
    for line in patch.read_text().splitlines():
        if line.startswith(('+++ b/', '--- a/')) and any(p.startswith('.env') for p in Path(line[6:]).parts):
            raise RuntimeError('Engine preparation must not modify environment files')
identity = hashlib.sha256(b''.join(p.read_bytes() for p in PATCHES)).hexdigest()
if identity != PIN['patches_sha256']:
    raise RuntimeError('Patch series identity mismatch')
cache = ROOT / 'artifacts/phpantom'
cache.mkdir(parents=True, exist_ok=True)
dest = cache / f"{PIN['commit'][:12]}-{identity[:12]}-verified"
marker = dest / 'openstorm-source.json'
receipt = cache / f'{dest.name}.files.json'

def source_files():
    files = {}
    for path in sorted(dest.rglob('*')):
        relative = path.relative_to(dest)
        if any(part in ('.git', 'target') for part in relative.parts):
            continue
        if any(part.startswith('.env') for part in relative.parts):
            raise RuntimeError('Unexpected environment file in acquired source')
        if path.is_symlink():
            payload = b'link:' + str(path.readlink()).encode()
        elif path.is_file():
            payload = path.read_bytes()
        else:
            continue
        files[str(relative)] = hashlib.sha256(payload).hexdigest()
    return files

if marker.exists():
    if json.loads(marker.read_text()) != PIN:
        raise RuntimeError('Cached source identity mismatch')
    if not receipt.exists() or json.loads(receipt.read_text()) != source_files():
        raise RuntimeError('Cached engine source changed; preserve it and acquire a clean source tree')
    print(dest)
    raise SystemExit(0)
if dest.exists():
    raise RuntimeError(f'Incomplete source preserved at {dest}')
archive = cache / f"{PIN['commit']}.tar.gz"
if not archive.exists():
    urllib.request.urlretrieve(f"https://api.github.com/repos/PHPantom-dev/phpantom_lsp/tarball/{PIN['commit']}", archive)
if hashlib.sha256(archive.read_bytes()).hexdigest() != PIN['archive_sha256']:
    raise RuntimeError('Source archive checksum mismatch')
dest.mkdir()
with tarfile.open(archive) as source:
    for member in source:
        member.name = '/'.join(member.name.split('/')[1:])
        if not member.name or any(p.startswith('.env') for p in Path(member.name).parts):
            continue
        target = (dest / member.name).resolve()
        if dest not in target.parents or member.islnk():
            raise RuntimeError('Unsafe archive path')
        if member.issym() and dest not in (target.parent / member.linkname).resolve().parents:
            raise RuntimeError('Unsafe archive link')
        source.extract(member, dest)
subprocess.run(['git', 'init', '--quiet', str(dest)], check=True)
for patch in PATCHES:
    subprocess.run(['git', 'apply', str(patch)], cwd=dest, check=True)
    subprocess.run(['git', 'apply', '--reverse', '--check', str(patch)], cwd=dest, check=True)
with (dest / 'Cargo.toml').open('a') as f:
    f.write('\n[workspace]\nexclude = ["vendor/tower-lsp"]\n')
marker.write_text(json.dumps(PIN, indent=2) + '\n')
receipt.write_text(json.dumps(source_files(), indent=2) + '\n')
print(dest)
