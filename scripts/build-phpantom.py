#!/usr/bin/env python3
"""Build the acquired engine; never silently replace the bundled release."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import hashlib
ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--pin', type=Path, default=ROOT / 'engine/source.json', help='Source manifest; defaults to the active engine pin')
args = parser.parse_args()
pin_path = args.pin.resolve()
pin = json.loads(pin_path.read_text())
source = subprocess.check_output(['python3', str(ROOT / 'scripts/prepare-phpantom.py'), '--pin', str(pin_path)], text=True).strip()
subprocess.run(['cargo', 'build', '--manifest-path', str(Path(source) / 'Cargo.toml'), '--release', '--locked'],
    env=dict(os.environ, PHPANTOM_GIT_VERSION=pin['version']), check=True)
binary = Path(os.environ.get('CARGO_TARGET_DIR', str(Path(source) / 'target'))) / 'release/phpantom_lsp'
print(json.dumps({'binary':str(binary), 'version':pin['version'], 'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(), 'source':source}, indent=2))
