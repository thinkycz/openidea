"""Check packaging refuses unsafe destinations before starting a build."""
import importlib.util
import json
import os
import plistlib
import shutil
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

    def test_bundled_development_build_keeps_application_identity(self):
        class BuildCaptured(Exception):
            pass

        with patch.dict(os.environ, {'CARGO_PROFILE_DEV_DEBUG_ASSERTIONS': 'true'}):
            with patch.object(package, 'run', side_effect=BuildCaptured) as build:
                with self.assertRaises(BuildCaptured):
                    package.main()
        self.assertEqual(build.call_args.args, ('pnpm', 'tauri', 'build', '--debug', '--bundles', 'app'))
        self.assertEqual(build.call_args.kwargs['env'].get('CARGO_PROFILE_DEV_DEBUG_ASSERTIONS'), 'false')

    def test_version_cannot_escape_archives(self):
        self.version('../outside')
        with self.assertRaisesRegex(RuntimeError, 'numeric'):
            package.main()

    def test_deferred_pruning_keeps_prior_app_until_embedded_acceptance(self):
        prior = self.root / 'artifacts/batches/0.1.53/OpenStorm.app'
        prior.mkdir(parents=True)
        sentinel = prior / 'keep'
        sentinel.write_text('prior accepted app')
        target = self.root / 'artifacts/build'
        source = target / 'debug/bundle/macos/OpenStorm.app'
        destination = self.root / 'artifacts/batches/0.1.54/OpenStorm.app'
        commands = []

        def run(*args, **kwargs):
            commands.append(args)
            if args[0] == 'pnpm':
                (source / 'Contents/MacOS').mkdir(parents=True)
                (source / 'Contents/MacOS/openidea').write_bytes(b'owned fixture')
                (source / 'Contents/Info.plist').write_bytes(plistlib.dumps({
                    'CFBundleIdentifier': 'cz.thinky.openstorm',
                    'CFBundleShortVersionString': '0.1.54',
                    'CFBundleExecutable': 'openidea',
                }))
            elif args[0] == 'ditto':
                shutil.copytree(args[1], args[2])
            elif args[0] == 'codesign':
                return
            elif 'openstorm-build-receipt.py' in args[1]:
                kwargs['stdout'].write('{"signature_verification":true}\n')
            elif args[:2] == ('cargo', 'clean'):
                shutil.rmtree(target)
            else:
                self.fail(f'Unexpected command: {args}')

        with patch.object(package, 'run', side_effect=run), \
                patch.object(package.subprocess, 'check_output', return_value='arm64\n'):
            package.main(defer_prune=True)
        self.assertEqual(sentinel.read_text(), 'prior accepted app')
        self.assertTrue(destination.is_dir())
        self.assertTrue((destination.parent / 'build-receipt.json').is_file())
        self.assertFalse((destination.parent / 'retention.json').exists())
        self.assertFalse(source.exists())
        self.assertEqual(commands[-1][:2], ('cargo', 'clean'))

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
