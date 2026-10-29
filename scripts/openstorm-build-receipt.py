#!/usr/bin/env python3
"""Record a local bundle, its source snapshot and pinned engine identity."""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import platform
import plistlib
import subprocess

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('bundle', type=Path)
parser.add_argument('--profile', choices=['development', 'release'], required=True)
args = parser.parse_args()
bundle = args.bundle.resolve()
info = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
version = json.loads((ROOT / 'package.json').read_text())['version']
if info['CFBundleIdentifier'] != 'cz.thinky.openstorm' or info['CFBundleShortVersionString'] != version:
    raise RuntimeError('Bundle identity/version does not match OpenStorm sources')

def digest(path):
    hasher = hashlib.sha256()
    with path.open('rb') as stream:
        while chunk := stream.read(1024 * 1024):
            hasher.update(chunk)
    return hasher.hexdigest()

def git(*arguments):
    return subprocess.check_output(['git', *arguments], cwd=ROOT).decode().strip()

engine = bundle / 'Contents/Resources/phpantom/phpantom_lsp'
provenance = json.loads((bundle / 'Contents/Resources/phpantom/provenance.json').read_text())
if digest(engine) != provenance['sha256']:
    raise RuntimeError('Bundled engine differs from its source provenance receipt')
paths = subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard', '-z'], cwd=ROOT).decode().split('\0')
snapshot = {}
for name in sorted(set(filter(None, paths))):
    path = ROOT / name
    if any(part.startswith('.env') for part in Path(name).parts) or not path.is_file():
        continue
    snapshot[name] = digest(path)
receipt = {
    'version': version,
    'identifier': info['CFBundleIdentifier'],
    'profile': args.profile,
    'bundle': str(bundle),
    'created_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'architecture': platform.machine(),
    'source_commit': git('rev-parse', 'HEAD'),
    'source_branch': git('branch', '--show-current'),
    'source_is_dirty': bool(git('status', '--porcelain')),
    'source_snapshot_sha256': hashlib.sha256(json.dumps(snapshot, sort_keys=True).encode()).hexdigest(),
    'source_files_sha256': snapshot,
    'executable_sha256': digest(bundle / 'Contents/MacOS' / info['CFBundleExecutable']),
    'phpantom': provenance,
    'signature_verification': subprocess.run(['codesign', '--verify', '--deep', '--strict', str(bundle)], capture_output=True, text=True).returncode == 0,
    'notarized': False,
}
if not receipt['signature_verification']:
    raise RuntimeError('Bundle signature verification failed')
print(json.dumps(receipt, indent=2))
