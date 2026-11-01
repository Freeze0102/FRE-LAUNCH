"""Offline regressions. Run: py -3 -m unittest discover -s scripts/modrinth-fork -p test_*.py -v"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import prepare
import verify

REPOSITORY = Path(__file__).resolve().parents[2]
UPSTREAM = REPOSITORY / 'vendor/modrinth'
TEMP_ROOT = REPOSITORY / '.audit'
TEMP_ROOT.mkdir(exist_ok=True)


class PatchTests(unittest.TestCase):
    def test_wrong_revision_rejected_without_mutation(self):
        with tempfile.TemporaryDirectory(dir=TEMP_ROOT, prefix='test-pin-') as directory:
            root = Path(directory)
            with self.assertRaisesRegex(RuntimeError, 'Unsupported Modrinth revision'):
                prepare.customize(root, '0' * 40)
            self.assertEqual(list(root.iterdir()), [])

    def test_anchor_drift_does_not_silently_patch(self):
        with tempfile.TemporaryDirectory(dir=TEMP_ROOT, prefix='test-anchor-') as directory:
            source = Path(directory) / 'source.txt'
            source.write_text('duplicate duplicate', encoding='utf-8')
            with self.assertRaises(RuntimeError):
                prepare.edit(source, 'duplicate', 'changed')
            self.assertEqual(source.read_text(), 'duplicate duplicate')

    @unittest.skipUnless((UPSTREAM / 'apps/app/src/main.rs').exists(), 'Initialize the pinned submodule first')
    def test_complete_pipeline_on_real_pin(self):
        sha = subprocess.check_output(['git', '-C', str(UPSTREAM), 'rev-parse', 'HEAD'], text=True).strip()
        self.assertEqual(sha, prepare.PINNED_UPSTREAM_SHA)
        with tempfile.TemporaryDirectory(dir=TEMP_ROOT, prefix='test-patches-') as directory:
            root = Path(directory) / 'source'
            shutil.copytree(UPSTREAM, root, ignore=shutil.ignore_patterns('.git', 'target', 'node_modules'))
            # A failed integration must never advertise a complete generated tree.
            import feature_curseforge
            with patch.object(feature_curseforge, 'apply', side_effect=RuntimeError('injected failure')):
                with self.assertRaisesRegex(RuntimeError, 'injected failure'):
                    prepare.customize(root, sha)
            self.assertFalse((root / 'FRE_LAUNCH_FORK.md').exists())
            self.assertTrue(root.resolve().is_relative_to(TEMP_ROOT.resolve()))
            shutil.rmtree(root)
            shutil.copytree(UPSTREAM, root, ignore=shutil.ignore_patterns('.git', 'target', 'node_modules'))
            prepare.customize(root, sha)
            verify.verify(root)
            main = root / 'apps/app/src/main.rs'
            before = main.read_bytes()
            with self.assertRaisesRegex(RuntimeError, 'already customized'):
                prepare.customize(root, sha)
            self.assertEqual(main.read_bytes(), before)
            capability = json.loads((root / 'apps/app/capabilities/plugins.json').read_text())
            self.assertTrue({'fre-curseforge:default', 'fre-horizon:default'} <= set(
                p for p in capability['permissions'] if isinstance(p, str)))
            # Optionally retain this exact verified result for local compilation.
            destination = os.environ.get('FRE_AUDIT_SOURCE')
            if destination:
                destination = Path(destination).resolve()
                self.assertTrue(destination.is_relative_to(REPOSITORY))
                self.assertFalse(destination.exists())
                shutil.copytree(root, destination)


if __name__ == '__main__':
    unittest.main()
