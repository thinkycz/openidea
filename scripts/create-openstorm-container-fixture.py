#!/usr/bin/env python3
"""Create an owned static Laravel container-navigation fixture; never boot PHP."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--baseline', required=True, type=Path)
parser.add_argument('--php', required=True, type=Path)
parser.add_argument('--label', default='container-navigation')
parser.add_argument('--reverse-providers', action='store_true',
                    help='Register the base provider after its subclass to exercise last-registration precedence')
args = parser.parse_args()
receipt = json.loads(subprocess.check_output([
    'python3', str(ROOT / 'scripts/create-openstorm-view-fixture.py'),
    '--baseline', str(args.baseline), '--php', str(args.php), '--label', args.label,
], text=True))
project = Path(receipt['project'])
files = {
    'app/OpenStormFixtures/BaseContainerProvider.php': """<?php
namespace App\\OpenStormFixtures;
use Illuminate\\Support\\ServiceProvider;
class BaseContainerProvider extends ServiceProvider {
    protected static string $abstract = 'openstorm.container';
    public function register(): void {
        $this->app->singleton(static::$abstract, OldContainerService::class);
    }
}
""",
    'app/OpenStormFixtures/ContainerProvider.php': """<?php
namespace App\\OpenStormFixtures;
class ContainerProvider extends BaseContainerProvider {
    public function register(): void {
        $label = '😀'; $this->app->singleton(static::$abstract, ContainerService::class);
        $this->app->alias(static::$abstract, 'openstorm.alias');
    }
}
""",
    'app/OpenStormFixtures/OldContainerService.php': """<?php
namespace App\\OpenStormFixtures;
class OldContainerService { public function oldOnly(): int { return 1; } public function deliver(): string { return 'old'; } }
""",
    'app/OpenStormFixtures/ContainerService.php': """<?php
namespace App\\OpenStormFixtures;
class ContainerService { public function deliver(): string { return 'current'; } }
""",
    'app/OpenStormFixtures/AlternateContainerService.php': """<?php
namespace App\\OpenStormFixtures;
class AlternateContainerService { public function deliver(): string { return 'alternate'; } }
""",
    'app/OpenStormFixtures/ContainerConsumer.php': """<?php
namespace App\\OpenStormFixtures;
$label = '😀'; $service = app('openstorm.container');
$alias = resolve('openstorm.alias');
$service->deliver();
$alias->deliver();
""",
}
# Change only the new fixture's provider inventory, preserving baseline entries.
providers = project / 'bootstrap/providers.php'
original = providers.read_text()
assert original.rstrip().endswith('];')
provider_order = ['App\\OpenStormFixtures\\BaseContainerProvider', 'App\\OpenStormFixtures\\ContainerProvider']
if args.reverse_providers:
    provider_order.reverse()
updated = original.rstrip()[:-2] + ''.join(f'    {name}::class,\n' for name in provider_order) + '];\n'
files['bootstrap/providers.php'] = updated
for name, text in files.items():
    path = project / name
    if path.exists() and name != 'bootstrap/providers.php':
        raise RuntimeError(f'Refusing to replace an existing fixture file: {name}')
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    subprocess.run([str(args.php.resolve()), '-n', '-l', str(path)], check=True, stdout=subprocess.DEVNULL)
assert not list(project.rglob('.env*'))
receipt['files'].update(files)
receipt['baseline_syntax'] = receipt['syntax']
php_count = sum(1 for path in project.rglob('*.php')
                if 'vendor' not in path.relative_to(project).parts and not path.name.endswith('.blade.php'))
receipt['syntax'] = f'{php_count} non-Blade PHP files linted across the baseline and container additions'
receipt['provider_order'] = provider_order
receipt['scenario'] = 'static container key and alias registration/class targets; inherited provider replacement; Unicode, unsaved rebinding and restart'
receipt['container_files_sha256'] = {n: hashlib.sha256((project / n).read_bytes()).hexdigest() for n in files}
print(json.dumps(receipt, indent=2))
