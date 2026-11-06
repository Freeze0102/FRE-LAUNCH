"""MIT-only compiler payloads must never carry the owner's generated game caches."""
import hashlib,json,unittest,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
class NativeBuilderAssetsTests(unittest.TestCase):
    def test_native_instance_labels_keep_ukrainian_and_english_in_the_generators_order(self):
        messages=json.loads((ROOT/'scripts/modrinth-fork/fre_rust_mod_ui_messages.json').read_text(encoding='utf-8'))
        self.assertEqual(messages['fre-rust.worlds.add'],['Додати сервер','Add server'])
        self.assertEqual(messages['fre-launch.account.mode.account'],['Акаунт','Account'])
        self.assertIn('Грати',messages['fre-rust.catalog.needs-build'][0])
    def test_vendored_mixin_tool_preserves_the_pinned_mit_source(self):
        source=ROOT/'vendor/fre-mixins-source'
        info=json.loads((source/'FRE-VENDORED-SOURCE.json').read_text(encoding='utf-8'))
        self.assertEqual(info['commit'],'a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab')
        self.assertEqual(info['license'],'MIT')
        self.assertIn('MIT License',(source/'LICENSE').read_text(encoding='utf-8'))
        for name,expected in info['files'].items():
            self.assertEqual(hashlib.sha256((source/name).read_bytes()).hexdigest(),expected,name)
    @unittest.skipUnless((ROOT/'scripts/modrinth-fork/fre-rust-runtime/fre-engine-source.zip').exists(),'Build the native runtime first')
    def test_bundled_source_contains_only_verified_sources_and_no_private_cache(self):
        runtime=ROOT/'scripts/modrinth-fork/fre-rust-runtime'
        info=json.loads((runtime/'fre-build-tools.json').read_text(encoding='utf-8'))
        archive=runtime/'fre-engine-source.zip'
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(),info['source_sha256'])
        self.assertEqual(hashlib.sha256((runtime/'fre-mixins.exe').read_bytes()).hexdigest(),info['mixins_sha256'])
        self.assertEqual((runtime/'fre-mixins-LICENSE.txt').read_bytes(),(ROOT/'vendor/fre-mixins-source/LICENSE').read_bytes())
        with zipfile.ZipFile(archive) as package:
            names=package.namelist()
            self.assertEqual(len(names),len(set(names)))
            self.assertIn('LICENSE',names)
            self.assertIn('crates/fre-client/src/game.rs',names)
            for name in names:
                self.assertFalse(any(part in {'.local','.git','target','node_modules'} for part in Path(name).parts),name)
            self.assertEqual(package.read('.fre-revision').strip(),b'975c07d9151847e6af13afc4b5ab37b6bbe78ef5')
            self.assertEqual(package.read('LICENSE'),(ROOT/'vendor/fre-rust-source/LICENSE').read_bytes())
if __name__=='__main__':unittest.main()
