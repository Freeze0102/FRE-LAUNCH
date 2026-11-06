import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
import native_runtime_payload as payload

class RuntimePayloadTests(unittest.TestCase):
    def runtime(self, root):
        runtime=root/'runtime';runtime.mkdir()
        for name in payload.FILES: (runtime/name).write_bytes(('MIT fixture '+name).encode())
        for folder in payload.SOURCE_DIRS:
            base=runtime/folder;base.mkdir(parents=True)
            (base/('Exporter.java' if folder.startswith('mods') else 'source.rs')).write_text('original MIT source')
        return runtime

    def test_embedded_payload_is_deterministic_and_excludes_private_caches(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);runtime=self.runtime(root);output=root/'payload.zip'
            for folder in ['resources','.local','target']:
                (runtime/folder).mkdir();(runtime/folder/'private.bin').write_bytes(b'private Minecraft data')
            payload.embed(runtime,output);first=output.read_bytes();payload.embed(runtime,output)
            self.assertEqual(first,output.read_bytes())
            with zipfile.ZipFile(output) as archive:
                manifest=json.loads(archive.read(payload.MANIFEST))
                self.assertEqual(len(manifest['files']),len(payload.FILES)+len(payload.SOURCE_DIRS))
                for file in manifest['files']:
                    data=archive.read(file['path']);self.assertEqual(len(data),file['size'])
                    self.assertEqual(hashlib.sha256(data).hexdigest(),file['sha256'])
                self.assertFalse(any('private' in name or name.startswith(('resources/','.local/','target/')) for name in archive.namelist()))

    def test_private_files_inside_source_directories_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);runtime=self.runtime(root)
            private=runtime/payload.SOURCE_DIRS[0]/'.local';private.mkdir();(private/'secret.rs').write_text('private')
            with self.assertRaisesRegex(RuntimeError,'Private or build cache'): payload.embed(runtime,root/'payload.zip')
