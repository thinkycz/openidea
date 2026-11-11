"""Regression checks for the destructive build-retention boundary."""
import importlib.util
from pathlib import Path
import plistlib
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('prune', Path(__file__).with_name('prune-openstorm-builds.py'))
prune = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prune)


class RetentionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def app(self, version, identifier='cz.thinky.openstorm'):
        app = self.root / 'artifacts/batches' / version / 'OpenStorm.app'
        (app / 'Contents').mkdir(parents=True)
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
            'CFBundleIdentifier': identifier, 'CFBundleShortVersionString': version}))
        return app

    def test_numeric_order_and_receipts_preserved(self):
        old = self.app('0.1.9')
        latest = self.app('0.1.10')
        receipt = old.parent / 'receipt.json'
        receipt.write_text('{}')
        self.assertEqual(prune.plan(self.root), ([old], latest))
        self.assertTrue(old.exists())  # Planning is read-only.
        self.assertTrue(receipt.exists())
        self.assertEqual(prune.apply(self.root), ([old], latest))
        self.assertFalse(old.exists())
        self.assertTrue(latest.exists())
        self.assertTrue(receipt.exists())

    def test_unknown_identity_refuses_whole_plan(self):
        self.app('0.1.9', 'other.app')
        self.app('0.1.10')
        with self.assertRaises(ValueError):
            prune.plan(self.root)

    def test_symlinked_archive_refused(self):
        app = self.app('0.1.9')
        (app.parent.parent / '0.1.10').symlink_to(app.parent, target_is_directory=True)
        with self.assertRaises(ValueError):
            prune.plan(self.root)

    def test_empty_inventory(self):
        self.assertEqual(prune.plan(self.root), ([], None))


if __name__ == '__main__':
    unittest.main()
