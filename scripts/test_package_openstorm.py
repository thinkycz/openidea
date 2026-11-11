"""Check packaging refuses unsafe destinations before starting a build."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('package', Path(__file__).with_name('package-openstorm.py'))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PackagingBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.version('0.1.54')
        for mock in (patch.object(package, 'ROOT', self.root),
                     patch.object(package.platform, 'system', return_value='Darwin'),
                     patch.object(package.platform, 'machine', return_value='arm64'),
                     patch.dict(os.environ, {'CARGO_TARGET_DIR': str(self.root / 'artifacts/build')}),
                     patch.object(package, 'run', side_effect=AssertionError('Unexpected build or mutation'))):
            mock.start()
            self.addCleanup(mock.stop)

    def version(self, version):
        (self.root / 'package.json').write_text(json.dumps({'version': version}))

    def test_version_cannot_escape_archives(self):
        self.version('../outside')
        with self.assertRaisesRegex(RuntimeError, 'numeric'):
            package.main()

    def test_symlinked_archives_cannot_write_outside_root(self):
        outside = self.root / 'outside'
        outside.mkdir()
        (self.root / 'artifacts').mkdir()
        (self.root / 'artifacts/batches').symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(RuntimeError, 'symlinked archive'):
            package.main()
        self.assertEqual(list(outside.iterdir()), [])

    def test_previous_receipt_cannot_be_overwritten(self):
        directory = self.root / 'artifacts/batches/0.1.54'
        directory.mkdir(parents=True)
        receipt = directory / 'build-receipt.json'
        receipt.write_text('preserve this')
        with self.assertRaisesRegex(RuntimeError, 'Existing receipt'):
            package.main()
        self.assertEqual(receipt.read_text(), 'preserve this')

    def test_shared_evaluation_target_cannot_be_cleaned(self):
        with patch.dict(os.environ, {'CARGO_TARGET_DIR': str(self.root / 'evaluation')}):
            with self.assertRaisesRegex(RuntimeError, 'shared/evaluation'):
                package.main()


if __name__ == '__main__':
    unittest.main()
