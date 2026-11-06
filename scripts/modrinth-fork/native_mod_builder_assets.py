"""Stage reproducible MIT native-mod build tools without any Minecraft game assets."""
from pathlib import Path
import hashlib,json,shutil,zipfile,subprocess,sys
ROOT=Path(__file__).resolve().parents[2]
PIN='a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab'
def prepare_tool():
    from native_branding import identifier
    original=ROOT/'vendor/fre-mixins-source'
    provenance=json.loads((original/'FRE-VENDORED-SOURCE.json').read_text(encoding='utf-8'))
    if provenance['commit']!=PIN or provenance['license']!='MIT':raise RuntimeError('Unexpected mixins source provenance')
    for name,expected in provenance['files'].items():
        if hashlib.sha256((original/name).read_bytes()).hexdigest()!=expected:raise RuntimeError('Edited mixins source: '+name)
    destination=ROOT/'.audit/fre-mixins-source'
    for p in original.rglob('*'):
        if not p.is_file():continue
        rel=Path(identifier(p.relative_to(original).as_posix()))
        target=destination/rel;target.parent.mkdir(parents=True,exist_ok=True)
        data=identifier(p.read_text(encoding='utf-8')).encode('utf-8') if p.suffix in {'.rs','.toml'} or p.name=='Cargo.lock' else p.read_bytes()
        if not target.exists() or target.read_bytes()!=data:target.write_bytes(data)
    subprocess.run(['cargo','+stable','build','--locked','--release','-p','fre-mixins-cli'],cwd=destination,check=True)

def stage():
    from native_branding import fingerprint
    runtime=ROOT/'scripts/modrinth-fork/fre-rust-runtime'
    source=ROOT/'.audit/fre-native-source/current'
    binary=ROOT/'.audit/fre-mixins-source/target/release/fre-mixins.exe'
    if not binary.is_file():raise RuntimeError('Build the pinned FRE mixins tool first')
    branded=json.loads((source/'FRE-BRANDING.json').read_text())
    if branded['branding_sha256']!=fingerprint():raise RuntimeError('Stale native source')
    archive=runtime/'fre-engine-source.zip'
    with zipfile.ZipFile(archive.with_suffix('.pending'), 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as output:
        # Include only the generator's verified MIT source allowlist. Build-time
        # .local caches may contain the owner's Minecraft data and are excluded.
        for name,expected in sorted(branded['files'].items()):
            p=source/name
            if not p.is_file() or p.is_symlink():raise RuntimeError('Missing or linked native build source')
            if hashlib.sha256(p.read_bytes()).hexdigest()!=expected:raise RuntimeError('Edited native source: '+name)
            if any(part in {'target','.git','.local','node_modules'} for part in Path(name).parts):raise RuntimeError('Unsafe source allowlist entry')
            entry=zipfile.ZipInfo(name,(1980,1,1,0,0,0));entry.compress_type=zipfile.ZIP_DEFLATED
            output.writestr(entry,p.read_bytes())
        output.writestr('.fre-revision','975c07d9151847e6af13afc4b5ab37b6bbe78ef5\n')
    archive.with_suffix('.pending').replace(archive)
    shutil.copyfile(binary,runtime/'fre-mixins.exe')
    shutil.copyfile(ROOT/'vendor/fre-mixins-source/LICENSE',runtime/'fre-mixins-LICENSE.txt')
    sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
    info={'schema':1,'mixins_commit':PIN,'license':'MIT','source_branding_sha256':fingerprint(),'source_sha256':sha(archive),'mixins_sha256':sha(runtime/'fre-mixins.exe')}
    (runtime/'fre-build-tools.json').write_text(json.dumps(info,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(info))
if __name__=='__main__':
    if '--build-tools' in sys.argv:prepare_tool()
    stage()
