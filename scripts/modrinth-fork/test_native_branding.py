"""Branding regressions for the engine that is actually packaged."""
import json
from pathlib import Path
import tempfile
import tomllib
import unittest

import native_branding as branding


class NativeBrandingTests(unittest.TestCase):
    def test_package_authorship_survives_product_rebranding(self):
        source = 'authors = ["Horizon"]\ndescription = "Horizon client"\n'
        result = branding.transform(source, '.toml')
        self.assertIn('authors = ["Horizon"]', result)
        self.assertIn('FRE LAUNCH client', result)
        result = branding.transform('    authors: vec!["Horizon".into()],', '.rs')
        self.assertIn('authors: vec!["Horizon".into()]', result)
        embedded = 'let manifest = r#"\nauthors = ["Horizon"]\n"#;'
        self.assertIn('authors = ["Horizon"]', branding.transform(embedded, '.rs'))
        assertion = 'assert_eq!(m.authors, ["Horizon"]);'
        self.assertEqual(branding.transform(assertion, '.rs'), assertion)

    def test_packaged_runtime_rejects_old_brand_in_filenames_and_sources(self):
        from feature_fre_rust import verify_runtime_branding
        with tempfile.TemporaryDirectory(dir=branding.REPO / '.audit') as parent:
            root = Path(parent)
            old = root / 'horizon-resources.json'
            old.write_text('{}')
            with self.assertRaisesRegex(RuntimeError, 'packaged filename'):
                verify_runtime_branding(root)
            old.unlink()
            source = root / 'exporter.java'
            source.write_text('package dev.horizonlab.parity;')
            with self.assertRaisesRegex(RuntimeError, 'runtime text'):
                verify_runtime_branding(root)
            source.write_text('package dev.frelaunch.parity; // horizontal geometry')
            verify_runtime_branding(root)

    def test_packaged_runtime_preserves_required_mit_attribution(self):
        from feature_fre_rust import verify_runtime_branding
        with tempfile.TemporaryDirectory(dir=branding.REPO / '.audit') as parent:
            root = Path(parent)
            license = root / 'LICENSE.txt'
            original = 'MIT License\nCopyright Horizon contributors\n'
            license.write_text(original)
            verify_runtime_branding(root)
            self.assertEqual(license.read_text(), original)

    def test_renames_identifiers_without_corrupting_horizontal_geometry(self):
        source = 'use horizon_core::geometry; HORIZON_HOME HorizonServer horizontal Horizontally'
        self.assertEqual(branding.transform(source, '.rs'),
            'use fre_core::geometry; FRE_HOME FreServer horizontal Horizontally')
        self.assertEqual(branding.identifier('mods/horizon/dev/horizonlab'), 'mods/fre/dev/frelaunch')

    def test_display_name_is_separate_from_valid_file_and_crate_names(self):
        result = branding.transform('horizon_client::open("Horizon", "horizon-resources.json", "HORIZON_HOME");', '.rs')
        self.assertEqual(result, 'fre_client::open("FRE LAUNCH", "fre-resources.json", "FRE_HOME");')
        self.assertNotIn('playhorizon', branding.transform('https://playhorizon.dev', '.rs'))
        self.assertEqual(branding.transform('Horizon puts the prelude before your files.', '.md'),
            'FRE LAUNCH puts the prelude before your files.')

    def test_complete_branded_tree_preserves_licenses_and_has_valid_cargo_manifests(self):
        # Use the same deterministic generator as the local and CI packager.
        with tempfile.TemporaryDirectory(dir=branding.REPO / '.audit', prefix='test-native-brand-') as parent:
            root = branding.build_tree(Path(parent) / 'source')
            self.assertEqual((root / 'LICENSE').read_bytes(),
                (branding.REPO / 'vendor/fre-rust-source/LICENSE').read_bytes())
            for cargo in root.rglob('Cargo.toml'):
                tomllib.loads(cargo.read_text(encoding='utf-8'))
            for name in json.loads((root / 'FRE-BRANDING.json').read_text())['files']:
                if name.startswith('third_party/') or Path(name).name in branding.LEGAL:
                    continue
                self.assertIsNone(branding.TOKEN.search(name), name)
            game = (root / 'crates/fre-client/src/game.rs').read_text(encoding='utf-8')
            self.assertIn('FRE LAUNCH', game)
            self.assertNotIn('Horizon', game)
            self.assertIn('fre-resources.json', game)
            self.assertIn('mod fre_client_mods;', game)
            hooks = (root / 'crates/fre-client/src/game/native_hooks.rs').read_text(encoding='utf-8')
            self.assertIn('self.fre_client_zoom(zoom)', hooks)
            self.assertIn('self.fre_client_hud(batch, corners)', hooks)
            self.assertTrue((root / 'crates/fre-client/src/game/fre_mod_settings.rs').is_file())
            discord = (root / 'crates/fre-client/src/discord_presence.rs').read_text(encoding='utf-8')
            self.assertNotIn('1563857609738690560', discord)
            self.assertIn('DISCORD_APP_ID.is_empty()', discord)
            self.assertIn('FRE_DISCORD_APP_ID', discord)
            self.assertNotEqual((root / 'packaging/icons/fre.ico').read_bytes(),
                (branding.REPO / 'vendor/fre-rust-source/packaging/icons/horizon.ico').read_bytes())
            self.assertEqual(branding.build_tree(root), root)


if __name__ == '__main__':
    unittest.main()
