"""Import and verify the complete, byte-exact MIT engine snapshot, without Git at build time."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import tempfile
import zipfile

REVISION = "975c07d9151847e6af13afc4b5ab37b6bbe78ef5"
SOURCE_TREE = "85268b86aaa57415fbb2c8d784d7a375b4b44d82"
ORIGIN = "https://github.com/horizon-rust/horizon"
MANIFEST = "FRE-VENDORED-SOURCE.json"


def git_blob(data):
    return hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()


def tree_hash(entries):
    tree = {}
    for name, info in entries.items():
        parts = PurePosixPath(name).parts
        if not parts or any(p in ("..", ".", ".git") for p in parts) or name.startswith("/") or "\\" in name:
            raise RuntimeError(f"Unsafe snapshot path: {name}")
        node = tree
        for part in parts[:-1]:
            node = node.setdefault(part, {})
        node[parts[-1]] = (info["mode"], info["blob"])

    def digest(node):
        content = bytearray()
        for name in sorted(node, key=lambda n: (n + ("/" if isinstance(node[n], dict) else "")).encode()):
            item = node[name]
            mode, oid = ("40000", digest(item)) if isinstance(item, dict) else item
            content += mode.encode() + b" " + name.encode() + b"\0" + bytes.fromhex(oid)
        return hashlib.sha1(b"tree " + str(len(content)).encode() + b"\0" + content).hexdigest()
    return digest(tree)


def verify(source):
    source = Path(source).resolve()
    meta = json.loads((source / MANIFEST).read_text(encoding="utf-8-sig"))
    if meta.get("source_commit") != REVISION or meta.get("engine_license") != "MIT":
        raise RuntimeError("Native source provenance differs from the approved MIT revision")
    entries = meta["files"]
    if len(entries) < 2500 or tree_hash(entries) != SOURCE_TREE or meta["source_tree"] != SOURCE_TREE:
        raise RuntimeError("Native source Git tree manifest is incomplete or corrupt")
    for name, info in entries.items():
        file = source / name
        # Windows without Developer Mode materializes Git symlinks as their target text.
        data = os.readlink(file).encode() if file.is_symlink() else file.read_bytes()
        if git_blob(data) != info["blob"]:
            raise RuntimeError(f"Native source differs from pinned commit: {name}")
    if not (source / "LICENSE").read_text().startswith("MIT License"):
        raise RuntimeError("Native source MIT attribution is missing")
    print(f"Verified {len(entries)} native source files at {REVISION}; Git tree {meta['source_tree']}")
    return meta


def import_source(checkout, destination):
    checkout, destination = Path(checkout).resolve(), Path(destination).resolve()
    if destination.exists():
        raise RuntimeError("Source destination already exists; no files were overwritten")
    def git(*args):
        return subprocess.check_output(["git", "-C", str(checkout), *args])
    if git("rev-parse", "HEAD").decode().strip() != REVISION:
        raise RuntimeError("Wrong native source checkout revision")
    entries = {}
    for entry in git("ls-tree", "-rz", "--full-tree", REVISION).split(b"\0"):
        if not entry:
            continue
        header, name = entry.split(b"\t", 1)
        mode, kind, oid = header.decode().split()
        if kind != "blob":
            raise RuntimeError("Snapshot contains an unvendored submodule")
        entries[name.decode()] = {"mode": mode, "blob": oid}
    meta = {
        "source_commit": REVISION, "source_tree": git("rev-parse", "HEAD^{tree}").decode().strip(),
        "source_origin": ORIGIN, "engine_license": "MIT",
        "proprietary_minecraft_assets_included": False, "files": entries,
    }
    if tree_hash(entries) != meta["source_tree"]:
        raise RuntimeError("Cannot reconstruct the pinned Git tree")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="fre-source-", dir=destination.parent) as temp:
        archive = Path(temp) / "source.zip"
        subprocess.run(["git", "-c", "core.autocrlf=false", "-C", str(checkout), "archive", "--format=zip", "-o", str(archive), REVISION], check=True)
        with zipfile.ZipFile(archive) as snapshot:
            destination.mkdir(parents=True)
            for name in entries:
                file = destination / name
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_bytes(snapshot.read(name))
                if os.name != "nt":
                    if entries[name]["mode"] == "120000":
                        target = file.read_text()
                        file.unlink()
                        file.symlink_to(target)
                    elif entries[name]["mode"] == "100755":
                        file.chmod(0o755)
    (destination / MANIFEST).write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8", newline="\n")
    return verify(destination)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--import-checkout", type=Path)
    args = parser.parse_args()
    if args.import_checkout:
        import_source(args.import_checkout, args.source)
    else:
        verify(args.source)
