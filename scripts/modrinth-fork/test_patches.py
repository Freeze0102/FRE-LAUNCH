"""Offline regressions. Run: py -3 -m unittest discover -s scripts/modrinth-fork -p test_*.py -v"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import prepare
import verify

REPOSITORY = Path(__file__).resolve().parents[2]
UPSTREAM = REPOSITORY / 'vendor/modrinth'
TEMP_ROOT = REPOSITORY / '.audit'
TEMP_ROOT.mkdir(exist_ok=True)


class PatchTests(unittest.TestCase):
    def test_vendored_engine_matches_the_complete_pinned_git_tree(self):
        import native_source
        meta = native_source.verify(REPOSITORY / 'vendor/fre-rust-source')
        self.assertFalse(meta['proprietary_minecraft_assets_included'])

    def test_security_lock_drift_cannot_mutate_workspace(self):
        import feature_security_dependencies
        with tempfile.TemporaryDirectory(dir=TEMP_ROOT, prefix='test-security-drift-') as directory:
            root = Path(directory)
            (root / 'pnpm-lock.yaml').write_text('wrong upstream lock')
            (root / 'pnpm-workspace.yaml').write_text('original workspace')
            with self.assertRaisesRegex(RuntimeError, 'lockfile base drift'):
                feature_security_dependencies.apply(root)
            self.assertEqual((root / 'pnpm-workspace.yaml').read_text(), 'original workspace')

    def test_security_release_age_exceptions_are_exact(self):
        import feature_security_dependencies
        with self.assertRaisesRegex(RuntimeError, 'exact security releases'):
            feature_security_dependencies.workspace_overrides({
                'overrides': {}, 'minimumReleaseAgeExclude': ['glob@*'],
            })

    def test_frontend_fixes_are_transactional(self):
        import feature_frontend_types
        with tempfile.TemporaryDirectory(dir=TEMP_ROOT, prefix='test-frontend-drift-') as directory:
            root = Path(directory)
            (root / 'source.ts').write_text('original', encoding='utf-8')
            fixes = [
                {'path': 'source.ts', 'old': 'original', 'new': 'changed', 'count': 1},
                {'path': 'source.ts', 'old': 'missing', 'new': 'wrong', 'count': 1},
            ]
            with patch.object(feature_frontend_types.json, 'loads', return_value=fixes):
                with self.assertRaisesRegex(RuntimeError, 'anchor drift'):
                    feature_frontend_types.apply(root)
            self.assertEqual((root / 'source.ts').read_text(), 'original')

    def test_frontend_fixes_cannot_escape_source(self):
        import feature_frontend_types
        with tempfile.TemporaryDirectory(dir=TEMP_ROOT, prefix='test-frontend-path-') as directory:
            fixes = [{'path': '../outside.ts', 'old': 'x', 'new': 'y', 'count': 1}]
            with patch.object(feature_frontend_types.json, 'loads', return_value=fixes):
                with self.assertRaisesRegex(RuntimeError, 'escaped'):
                    feature_frontend_types.apply(Path(directory))

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
            # Catch duplicate dependency keys before spending Windows build time.
            for manifest in root.rglob('Cargo.toml'):
                with self.subTest(manifest=str(manifest.relative_to(root))):
                    tomllib.loads(manifest.read_text(encoding='utf-8'))
            main = root / 'apps/app/src/main.rs'
            before = main.read_bytes()
            with self.assertRaisesRegex(RuntimeError, 'already customized'):
                prepare.customize(root, sha)
            self.assertEqual(main.read_bytes(), before)
            capability = json.loads((root / 'apps/app/capabilities/plugins.json').read_text())
            self.assertTrue({'fre-curseforge:default', 'fre-rust:default'} <= set(
                p for p in capability['permissions'] if isinstance(p, str)))
            self.assertNotIn('fre-horizon:default', capability['permissions'])
            app_config = json.loads((root / 'apps/app/tauri.conf.json').read_text())
            icon_scope = app_config['app']['security']['assetProtocol']['scope']
            for extension in ('png', 'jpg', 'webp'):
                self.assertIn(f'$APPDATA/fre-rust-minecraft/profiles/*/icon.{extension}', icon_scope)
            # Optionally retain this exact verified result for local compilation.
            destination = os.environ.get('FRE_AUDIT_SOURCE')
            if destination:
                destination = Path(destination).resolve()
                self.assertTrue(destination.is_relative_to(REPOSITORY))
                self.assertFalse(destination.exists())
                shutil.copytree(root, destination)


if __name__ == '__main__':
    unittest.main()
