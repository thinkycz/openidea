#!/usr/bin/env python3
"""Keep the newest archived OpenStorm app. Dry run unless --apply is supplied.

Only artifacts/batches/<numeric version>/OpenStorm.app bundles are eligible.
Source, receipts, fixtures, installed applications and compiler caches are untouched.
"""
import argparse
import json
from pathlib import Path
import plistlib
import re
import shutil

ROOT = Path(__file__).resolve().parent.parent


def plan(root):
    batches = root / 'artifacts/batches'
    if (root / 'artifacts').is_symlink() or batches.is_symlink():
        raise ValueError('Refusing a symlinked batches directory')
    apps = []
    if not batches.exists():
        return [], None
    for directory in batches.iterdir():
        if not re.fullmatch(r'\d+\.\d+\.\d+', directory.name):
            continue
        app = directory / 'OpenStorm.app'
        if directory.is_symlink() or app.is_symlink():
            raise ValueError(f'Refusing symlink: {directory}')
        if not app.exists():
            continue
        info_path = app / 'Contents/Info.plist'
        if info_path.is_symlink() or (app / 'Contents').is_symlink():
            raise ValueError(f'Refusing symlinked bundle metadata: {app}')
        info = plistlib.loads(info_path.read_bytes())
        if (info.get('CFBundleIdentifier') != 'cz.thinky.openstorm'
                or info.get('CFBundleShortVersionString') != directory.name):
            raise ValueError(f'Unexpected bundle identity: {app}')
        apps.append((tuple(map(int, directory.name.split('.'))), app))
    apps.sort()
    return [app for _, app in apps[:-1]], apps[-1][1] if apps else None


def apply(root):
    obsolete, kept = plan(root)
    for app in obsolete:
        shutil.rmtree(app)
    return obsolete, kept


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args()
    obsolete, kept = plan(ROOT)
    receipt = {'mode': 'apply' if args.apply else 'dry-run',
               'kept': str(kept) if kept else None,
               'obsolete': [str(app) for app in obsolete]}
    if args.apply:
        # Validate every candidate before the first deletion. Refuse changed plans.
        if plan(ROOT) != (obsolete, kept):
            raise RuntimeError('Build inventory changed; run again')
        apply(ROOT)
    print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()
