#!/usr/bin/env python3
"""Create an owned Laravel route-refactoring corpus without booting the framework."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--baseline', required=True, type=Path)
parser.add_argument('--php', required=True, type=Path)
parser.add_argument('--label', default='route-refactoring')
args = parser.parse_args()
receipt = json.loads(subprocess.check_output([
    'python3', str(ROOT / 'scripts/create-openstorm-view-fixture.py'),
    '--baseline', str(args.baseline), '--php', str(args.php), '--label', args.label,
], text=True))
project = Path(receipt['project'])
files = {
    'routes/openstorm_refactor.php': """<?php
use App\\OpenStormFixtures\\RoutesConsumer;
use Illuminate\\Support\\Facades\\Route as Routes;
Routes::name('openstorm.billing.')->group(static function (): void {
    Routes::get('/openstorm/invoices/{invoice}', [RoutesConsumer::class, 'invoice'])->name('invoice');
    Routes::get('/openstorm/invoices', [RoutesConsumer::class, 'invoice'])->name('list');
});
Routes::get('/openstorm/a', [RoutesConsumer::class, 'invoice'])->name('openstorm.ambiguous');
Routes::get('/openstorm/b', [RoutesConsumer::class, 'invoice'])->name('openstorm.ambiguous');
Routes::resource('openstorm-items', RoutesConsumer::class);
""",
    'app/OpenStormFixtures/RoutesConsumer.php': """<?php
namespace App\\OpenStormFixtures;
class RoutesConsumer {
    public function invoice(): string {
        $label = '😀'; $literal = 'openstorm.billing.invoice';
        return route('openstorm.billing.invoice', ['invoice' => 7]);
    }
    public function ambiguous(): string { return route('openstorm.ambiguous'); }
    public function generated(): string { return route('openstorm-items.show', ['openstorm_item' => 7]); }
}
""",
    'app/OpenStormFixtures/RouteMailer.php': """<?php
namespace App\\OpenStormFixtures;
class RouteMailer {
    public function link(): string { $label = '😀'; return route('openstorm.billing.invoice', ['invoice' => 8]); }
}
""",
    'resources/views/openstorm-invoice.blade.php': "<p>😀 {{ route('openstorm.billing.invoice', ['invoice' => 7]) }}</p>\n",
}
for name, text in files.items():
    path = project / name
    if path.exists():
        raise RuntimeError(f'Refusing to replace an existing fixture file: {name}')
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    if name.endswith('.php') and not name.endswith('.blade.php'):
        subprocess.run([str(args.php.resolve()), '-n', '-l', str(path)], check=True, stdout=subprocess.DEVNULL)
receipt['scenario'] = 'grouped route references/checked rename; duplicate, generated and collision refusals; Unicode PHP/Blade callers'
receipt['files'].update(files)
receipt['route_files_sha256'] = {name: hashlib.sha256((project / name).read_bytes()).hexdigest() for name in files}
assert not list(project.glob('.env*'))
print(json.dumps(receipt, indent=2))
