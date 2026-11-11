#!/usr/bin/env python3
"""Build/sign a local Apple Silicon development app and retain only that archive.

Run the affected source gates first. This command does not install, publish,
notarize or launch the app. Configure Cargo/pnpm in PATH before running it.
"""
import datetime
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent


def run(*args, **kwargs):
    subprocess.run(args, cwd=ROOT, check=True, **kwargs)


def main():
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        raise RuntimeError('This recipe targets Apple Silicon macOS')
    version = json.loads((ROOT / 'package.json').read_text())['version']
    if not re.fullmatch(r'\d+\.\d+\.\d+', version):
        raise RuntimeError('Expected a numeric OpenStorm version')
    destination = ROOT / 'artifacts/batches' / version / 'OpenStorm.app'
    for path in (ROOT / 'artifacts', destination.parent.parent, destination.parent):
        if path.is_symlink():
            raise RuntimeError(f'Refusing symlinked archive path: {path}')
    for name in ('build-receipt.json', 'retention.json'):
        if (destination.parent / name).exists():
            raise RuntimeError(f'Existing receipt for version {version}; use a new version')
    if destination.exists() or destination.is_symlink():
        raise RuntimeError(f'Archive already exists: {destination}')
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(ROOT / 'artifacts/build'))).resolve()
    if target != (ROOT / 'artifacts/build').resolve():
        raise RuntimeError('Use artifacts/build; shared/evaluation build caches are not managed here')
    if (ROOT / 'artifacts').is_symlink() or (ROOT / 'artifacts/build').is_symlink():
        raise RuntimeError('Refusing a symlinked build directory')
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL='0')
    run('pnpm', 'tauri', 'build', '--debug', '--bundles', 'app', env=env)
    source = target / 'debug/bundle/macos/OpenStorm.app'
    info = plistlib.loads((source / 'Contents/Info.plist').read_bytes())
    if (info['CFBundleIdentifier'] != 'cz.thinky.openstorm'
            or info['CFBundleShortVersionString'] != version):
        raise RuntimeError('Built app identity does not match source')
    binary = source / 'Contents/MacOS' / info['CFBundleExecutable']
    architecture = subprocess.check_output(['lipo', '-archs', str(binary)], text=True).strip()
    if architecture != 'arm64':
        raise RuntimeError(f'Unexpected architecture: {architecture}')
    destination.parent.mkdir(parents=True, exist_ok=True)
    run('ditto', str(source), str(destination))
    run('codesign', '--force', '--sign', '-', '--timestamp=none', str(destination))
    run('codesign', '--verify', '--deep', '--strict', str(destination))
    # The receipt also verifies engine provenance and hashes the source snapshot.
    receipt = destination.parent / 'build-receipt.json'
    with receipt.open('x') as stream:
        run(sys.executable, str(ROOT / 'scripts/openstorm-build-receipt.py'),
            str(destination), '--profile', 'development', stdout=stream)
    # Retain the prior app until the new signature and receipt have succeeded.
    # Save the deletion inventory alongside the current receipt.
    with (destination.parent / 'retention.json').open('x') as stream:
        run(sys.executable, str(ROOT / 'scripts/prune-openstorm-builds.py'), '--apply', stdout=stream)
    shutil.rmtree(source)  # Avoid keeping a second copy of the same signed archive.
    # All source gates have finished and the signed archive is self-contained.
    # Keep their receipts, not test binaries/objects from previous version builds.
    run('cargo', 'clean', '--target-dir', str(target), env=env)
    print(json.dumps({'bundle': str(destination), 'receipt': str(receipt),
                      'created_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                      'installed': False, 'notarized': False}))


if __name__ == '__main__':
    main()
