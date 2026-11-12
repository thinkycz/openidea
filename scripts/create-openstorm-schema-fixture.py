#!/usr/bin/env python3
"""Create an owned migration/model corpus without running Laravel or a database."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--baseline', required=True, type=Path)
parser.add_argument('--php', required=True, type=Path)
parser.add_argument('--label', default='schema')
parser.add_argument('--pin', type=Path, default=ROOT / 'engine/source.json')
args = parser.parse_args()
receipt = json.loads(subprocess.check_output([
    'python3', str(ROOT / 'scripts/create-openstorm-view-fixture.py'),
    '--baseline', str(args.baseline), '--php', str(args.php), '--label', args.label,
    '--pin', str(args.pin.resolve()),
], text=True))
project = Path(receipt['project'])
files = {
    'app/OpenStormFixtures/SchemaRecord.php': """<?php
namespace App\\OpenStormFixtures;
use Illuminate\\Database\\Eloquent\\Model;
final class SchemaRecord extends Model {
    protected $table = 'openstorm_schema_records';
}
""",
    'database/migrations/2026_01_01_000001_create_openstorm_schema_records.php': """<?php
use Illuminate\\Database\\Migrations\\Migration;
use Illuminate\\Database\\Schema\\Blueprint;
use Illuminate\\Support\\Facades\\Schema;
return new class extends Migration {
    public function up(): void {
        Schema::create('openstorm_schema_records', static function (Blueprint $table): void {
            $table->id();
            $marker = '😀'; $table->string('email');
            $table->timestamp('email_verified_at')->nullable();
            $table->boolean('is_active')->default(true);
            $table->index('email', 'openstorm_email_index');
        });
    }
    public function down(): void { Schema::dropIfExists('openstorm_schema_records'); }
};
""",
    'app/OpenStormFixtures/SchemaConsumer.php': """<?php
namespace App\\OpenStormFixtures;
final class SchemaConsumer {
    public function inspect(SchemaRecord $record): void {
        $label = '😀';
        $record->email;
        $record->email_verified_at;
        $record->is_active;
    }
}
""",
}
for name, text in files.items():
    path = project / name
    assert not path.exists(), name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    subprocess.run([str(args.php.resolve()), '-n', '-l', str(path)],
                   check=True, stdout=subprocess.DEVNULL)
receipt['files'].update(files)
receipt['scenario'] = 'migration-derived model completion, type/nullability and schema provenance; index names excluded'
receipt['schema_files_sha256'] = {
    name: hashlib.sha256(text.encode()).hexdigest() for name, text in files.items()
}
receipt['environment_files'] = 'excluded; none created'
assert not list(project.rglob('.env*'))
print(json.dumps(receipt, indent=2))
