"""Deterministic, MIT-only recovery payload embedded in the desktop executable."""
import hashlib
import json
from pathlib import Path
import zipfile

FILES = ('fre-rust-client.exe', 'fre-rust-prepare.exe', 'BUILD-INFO.json', 'research-lock.json', 'LICENSE.txt', 'fre-mixins.exe', 'fre-mixins-LICENSE.txt', 'fre-engine-source.zip', 'fre-build-tools.json')
SOURCE_DIRS = ('crates/fre-prepare', 'mods/fre/src/test/java/dev/frelaunch/parity')
MANIFEST = '_FRE-RUNTIME.json'

def embed(runtime: Path | None, output: Path):
    output.parent.mkdir(parents=True, exist_ok=True)
    entries = {}
    if runtime is not None:
        for name in FILES:
            path = runtime / name
            if not path.is_file() or path.is_symlink(): raise RuntimeError('Incomplete runtime payload: ' + name)
            entries[name] = path
        for folder in SOURCE_DIRS:
            base = runtime / folder
            if not base.is_dir() or base.is_symlink(): raise RuntimeError('Missing MIT exporter source: ' + folder)
            for path in base.rglob('*'):
                if path.is_symlink(): raise RuntimeError('Linked runtime payload source')
                if any(part in {'.git', '.local', 'target', 'node_modules', 'resources'} for part in path.relative_to(base).parts): raise RuntimeError('Private or build cache in exporter payload')
                if path.is_file():
                    name = path.relative_to(runtime).as_posix()
                    if path.suffix not in {'.rs', '.toml', '.java'}: raise RuntimeError('Unexpected exporter payload: ' + name)
                    entries[name] = path
    manifest = {'schema': 1, 'files': []}
    temporary = output.with_suffix('.pending')
    with zipfile.ZipFile(temporary, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for name, path in sorted(entries.items()):
            data = path.read_bytes()
            manifest['files'].append({'path': name, 'size': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
            entry = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0)); entry.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(entry, data)
        entry = zipfile.ZipInfo(MANIFEST, (1980, 1, 1, 0, 0, 0)); entry.compress_type = zipfile.ZIP_DEFLATED
        archive.writestr(entry, json.dumps(manifest, separators=(',', ':'), sort_keys=True).encode())
    # An identical refresh must not invalidate the Rust build unnecessarily.
    if output.exists() and output.read_bytes() == temporary.read_bytes(): temporary.unlink()
    else: temporary.replace(output)
