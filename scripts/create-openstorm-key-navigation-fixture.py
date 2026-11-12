#!/usr/bin/env python3
"""Create disposable Laravel config/translation navigation sources; never boot PHP."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--baseline', required=True, type=Path)
parser.add_argument('--php', required=True, type=Path)
parser.add_argument('--label', default='key-navigation')
args = parser.parse_args()
receipt = json.loads(subprocess.check_output([
    'python3', str(ROOT / 'scripts/create-openstorm-view-fixture.py'),
    '--baseline', str(args.baseline), '--php', str(args.php), '--label', args.label,
], text=True))
project = Path(receipt['project'])
files = {
    'config/openstorm_keys.php': """<?php
return [
    'limit' => 3,
    'nested' => ['daily' => 10],
];
""",
    'lang/en/openstorm.php': """<?php
return [
    'title' => 'Title',
    'items' => 'One item|Many items',
];
""",
    'lang/cs/openstorm.php': """<?php
return [
    'title' => 'Nadpis',
    'items' => 'Jedna položka|Více položek',
];
""",
    'lang/cs.json': '{"OpenStorm greeting":"Ahoj"}\n',
    'app/OpenStormFixtures/KeyConsumer.php': """<?php
namespace App\\OpenStormFixtures;
use function config as setting;
use function __ as message;
use Illuminate\\Support\\Facades\\Config as Settings;
use Illuminate\\Support\\Facades\\Lang as Labels;
$label = '😀'; setting(default: 0, key: 'openstorm_keys.limit');
Settings::get(default: 0, key: 'openstorm_keys.limit');
message(key: 'openstorm.title');
Labels::get(key: 'openstorm.title');
message('OpenStorm greeting');
setting(default: 'openstorm_keys.limit', key: 'openstorm_keys.missing');
message(key: 'openstorm.missing', replace: ['name' => 'openstorm.title']);
""",
    'app/OpenStormFixtures/KeyOther.php': """<?php
namespace App\\OpenStormFixtures;
use function config as setting;
use function trans as translate;
setting('openstorm_keys.limit');
translate('openstorm.title');
""",
    'app/OpenStormFixtures/LocalKeyConsumer.php': """<?php
namespace App\\OpenStormLocalKeys;
function config(string $key): string { return $key; }
function __(string $key): string { return $key; }
config('openstorm_keys.limit');
__('openstorm.title');
""",
}
for name, text in files.items():
    path = project / name
    if path.exists():
        raise SystemExit(f'Owned fixture path already exists: {name}')
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    if path.suffix == '.php':
        subprocess.run([str(args.php.resolve()), '-n', '-l', str(path)],
                       check=True, stdout=subprocess.DEVNULL)
    elif path.suffix == '.json':
        json.loads(text)
receipt['files'].update(files)
assert not list(project.rglob('.env*'))
php_count = sum(1 for path in project.rglob('*.php')
                if 'vendor' not in path.relative_to(project).parts and '.blade.php' not in path.name)
receipt['syntax'] = f'{php_count} non-Blade PHP files linted across baseline and key-navigation additions'
receipt['scenario'] = 'aliased helpers/facades, named keys, multiple locales and local/non-key argument isolation'
receipt['key_files_sha256'] = {name: hashlib.sha256((project / name).read_bytes()).hexdigest() for name in files}
print(json.dumps(receipt, indent=2))
