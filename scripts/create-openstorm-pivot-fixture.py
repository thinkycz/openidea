#!/usr/bin/env python3
"""Create a disposable Eloquent pivot-navigation corpus without booting Laravel."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--baseline', required=True, type=Path)
parser.add_argument('--php', required=True, type=Path)
parser.add_argument('--pin', type=Path, default=ROOT / 'engine/source.json',
                    help='Engine source manifest used to validate the baseline')
parser.add_argument('--label', default='pivot-navigation')
args = parser.parse_args()
receipt = json.loads(subprocess.check_output([
    'python3', str(ROOT / 'scripts/create-openstorm-view-fixture.py'),
    '--baseline', str(args.baseline), '--php', str(args.php), '--pin', str(args.pin),
    '--label', args.label,
], text=True))
project = Path(receipt['project'])
models = ['app/Models/Bakery.php', 'app/Models/BakeryRecipe.php',
          'app/Models/RecipeIngredient.php', 'app/Demo.php']
for name in models:
    if not (project / name).is_file():
        raise SystemExit(f'Missing pinned pivot source: {name}')
if "getQuantityLabel(): string { return ''; }" not in (project / models[2]).read_text():
    raise SystemExit('Pinned pivot method no longer matches this regression')
source = """<?php
use App\\Models\\Bakery;
$bakery = new Bakery();
$label = '😀'; $bakery->masterRecipe->first()->pivot->getQuantityLabel();
$bakery->seasonalRecipes->first()->ingredient->getQuantityLabel();
$bakery->masterRecipe->first()->pivot->quantityInGrams();
$bakery->seasonalRecipes->first()->ingredient->quantityInGrams();
"""
name = 'app/OpenStormFixtures/PivotConsumer.php'
path = project / name
if path.exists():
    raise SystemExit('Owned pivot consumer already exists')
path.write_text(source)
subprocess.run([str(args.php.resolve()), '-n', '-l', str(path)],
               check=True, stdout=subprocess.DEVNULL)
receipt['files'][name] = source
receipt['pivot_sources_sha256'] = {
    name: hashlib.sha256((project / name).read_bytes()).hexdigest() for name in models
}
assert not list(project.rglob('.env*'))
count = sum(1 for path in project.rglob('*.php')
            if 'vendor' not in path.relative_to(project).parts and '.blade.php' not in path.name)
receipt['syntax'] = f'{count} non-Blade PHP files linted across the baseline and pivot additions'
receipt['scenario'] = 'default/renamed pivot accessors; dependency-only method replacement with unchanged caller, Unicode and restore/restart controls'
print(json.dumps(receipt, indent=2))
