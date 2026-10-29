#!/usr/bin/env python3
"""Build the acquired engine; never silently replace the bundled release."""
import json
import os
from pathlib import Path
import subprocess
import hashlib
ROOT = Path(__file__).resolve().parent.parent
pin = json.loads((ROOT / 'engine/source.json').read_text())
source = subprocess.check_output(['python3', str(ROOT / 'scripts/prepare-phpantom.py')], text=True).strip()
subprocess.run(['cargo', 'build', '--manifest-path', str(Path(source) / 'Cargo.toml'), '--release', '--locked'],
    env=dict(os.environ, PHPANTOM_GIT_VERSION=pin['version']), check=True)
binary = Path(os.environ.get('CARGO_TARGET_DIR', str(Path(source) / 'target'))) / 'release/phpantom_lsp'
print(json.dumps({'binary':str(binary), 'version':pin['version'], 'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(), 'source':source}, indent=2))
