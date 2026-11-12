#!/usr/bin/env python3
"""Create an isolated view-data corpus from the pinned, installed Laravel example."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--baseline', type=Path, required=True,
                    help='Disposable pinned engine example with Laravel 13.37.0 already installed')
parser.add_argument('--php', type=Path, required=True, help='Explicit PHP 8.4+ CLI used only for syntax checks')
parser.add_argument('--label', default='view', help='Short label for the new temporary fixture')
parser.add_argument('--pin', type=Path, default=ROOT / 'engine/source.json',
                    help='Engine source manifest used to validate the baseline')
parser.add_argument('--nested-includes', action='store_true',
                    help='Render the data through an intermediate page and included partial')
args = parser.parse_args()
if not re.fullmatch(r'[A-Za-z0-9._-]+', args.label):
    parser.error('Label must contain only letters, digits, periods, underscores or hyphens')
baseline = args.baseline.resolve()
pin = json.loads(args.pin.read_text())
source = ROOT / 'artifacts/phpantom' / f"{pin['commit'][:12]}-{pin['patches_sha256'][:12]}-verified"
if not (source / 'openstorm-source.json').is_file():
    parser.error('Acquire the pinned engine source with scripts/prepare-phpantom.py first')
if json.loads((source / 'openstorm-source.json').read_text()) != pin:
    parser.error('Pinned engine source receipt does not match engine/source.json')
source_files = json.loads(source.with_name(source.name + '.files.json').read_text())
verified = 0
for name, expected in source_files.items():
    if not name.startswith('examples/laravel/'):
        continue
    relative = name.removeprefix('examples/laravel/')
    if relative in ('composer.json', 'composer.lock'):
        continue
    path = baseline / relative
    if not (path.is_file() or path.is_symlink()):
        parser.error(f'Pinned baseline file is missing: {relative}')
    payload = b'link:' + str(path.readlink()).encode() if path.is_symlink() else path.read_bytes()
    if hashlib.sha256(payload).hexdigest() != expected:
        parser.error(f'Baseline differs from the pinned Laravel example: {relative}')
    verified += 1
lock = json.loads((baseline / 'composer.lock').read_text())
framework = next((p for p in lock['packages'] if p['name'] == 'laravel/framework'), None)
if framework is None or framework['version'] != 'v13.37.0':
    parser.error('Baseline must use the explicitly installed Laravel 13.37.0 acceptance lock')
composer = json.loads((baseline / 'composer.json').read_text())
if composer.get('autoload', {}).get('psr-4', {}).get('App\\') != 'app/':
    parser.error('Baseline must map App\\ to app/ through Composer PSR-4')
if not (baseline / 'vendor/composer/autoload_files.php').is_file():
    parser.error('Installed Composer helper autoload metadata is required')
installed = json.loads((baseline / 'vendor/composer/installed.json').read_text())
packages = installed['packages'] if isinstance(installed, dict) else installed
if not any(p['name'] == 'laravel/framework' and p['version'] == 'v13.37.0' for p in packages):
    parser.error('Installed framework does not match the acceptance lock')
files = {
    'app/OpenStormFixtures/Item.php': '<?php\nnamespace App\\OpenStormFixtures;\nclass Item { public string $name; public int $price; }\n',
    'app/OpenStormFixtures/Stock.php': '<?php\nnamespace App\\OpenStormFixtures;\nclass Stock { public string $sku; public int $quantity; }\n',
    'app/OpenStormFixtures/Controller.php': "<?php\nnamespace App\\OpenStormFixtures;\nclass Controller { public function show(): mixed {\n$item = new Item();\nreturn view('openstorm-stock', compact('item'));\n} }\n",
    'resources/views/openstorm-stock.blade.php': '<h1>😀 shop</h1>\n{{ $item->name }}\n',
}
if args.nested_includes:
    files['resources/views/openstorm-stock.blade.php'] = "@include('partials.openstorm-stock-row', ['row' => $item])\n"
    files['resources/views/partials/openstorm-stock-row.blade.php'] = '<p>😀 stock</p>\n{{ $row->name }}\n'
if any((baseline / name).exists() for name in files):
    parser.error('Baseline already contains the owned view-data fixture files')
base = Path(tempfile.mkdtemp(prefix=f'openstorm-{args.label}-')).resolve()
project, profile = base / 'project', base / 'profile'
shutil.copytree(baseline, project, symlinks=True,
                ignore=shutil.ignore_patterns('.git', '.openidea', '.env*'))
profile.mkdir()
for name, text in files.items():
    path = project / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
php_files = [path for path in project.rglob('*.php')
             if 'vendor' not in path.relative_to(project).parts and '.blade.php' not in path.name]
for path in php_files:
    subprocess.run([str(args.php.resolve()), '-n', '-l', str(path)], check=True,
                   stdout=subprocess.DEVNULL)
assert not list(project.glob('.env*'))
receipt = {
    'project': str(project), 'profile': str(profile), 'framework': '13.37.0',
    'source_pin': str(args.pin.resolve()),
    'source_pin_sha256': hashlib.sha256(args.pin.read_bytes()).hexdigest(),
    'baseline': str(baseline), 'engine_source': str(source),
    'composer_lock_sha256': hashlib.sha256((project / 'composer.lock').read_bytes()).hexdigest(),
    'composer_autoload_files_sha256': hashlib.sha256((project / 'vendor/composer/autoload_files.php').read_bytes()).hexdigest(),
    'environment_files': 'excluded; none created',
    'verified_pinned_baseline_files': verified,
    'syntax': f'{len(php_files)} non-Blade PHP files linted with explicit PHP CLI and -n',
    'scenario': 'nested includes' if args.nested_includes else 'direct view data',
    'files': files,
}
print(json.dumps(receipt, indent=2))
