"""Build FRE's own engine tree from the immutable, licensed upstream snapshot.

Never edits the pinned vendor tree or its attribution. Cargo, Java exporters,
resource names, native UI and file names are renamed together, before compiling.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re

import native_source

REPO = Path(__file__).resolve().parents[2]
LEGAL = {"LICENSE", "LICENSE.txt", "THIRD_PARTY_NOTICES.md"}
TOKEN = re.compile(r"horizon(?!tal)", re.I)
STRING = re.compile(r'"(?:\\.|[^"\\])*"', re.S)


def identifier(text):
    text = text.replace("horizonlab", "frelaunch").replace("HorizonLab", "FreLaunch")
    def rename(match):
        old = match[0]
        return "FRE" if old.isupper() else "Fre" if old[0].isupper() else "fre"
    return TOKEN.sub(rename, text)


def transform(text, suffix=""):
    credits = []
    def preserve(match):
        credits.append(match[0])
        return f"__FRE_CREDIT_{len(credits) - 1}__"
    # Third-party package authorship is attribution, not the fork's UI brand.
    patterns = {
        ".toml": r"(?m)^\s*authors\s*=\s*\[[^\]]*\]",
        ".rs": r"(?m)(?:^\s*authors\s*(?::\s*vec!|=\s*)\[[^\]]*\]|assert_eq!\(\s*[\w.]+\.authors\s*,\s*\[[^\]]*\])",
        ".json": r'"authors"\s*:\s*\[[^\]]*\]',
    }
    if suffix in patterns:
        text = re.sub(patterns[suffix], preserve, text)
    text = text.replace("https://github.com/horizon-rust/horizon", "https://github.com/Freeze0102/FRE-LAUNCH")
    text = text.replace("https://playhorizon.dev", "https://github.com/Freeze0102/FRE-LAUNCH/releases")
    text = text.replace("playhorizon.dev", "github.com/Freeze0102/FRE-LAUNCH")
    # A display name may contain spaces; crate names, Java classes, translation
    # keys, environment variables and paths must remain valid identifiers.
    if suffix in {".rs", ".java", ".json", ".toml"}:
        text = STRING.sub(lambda m: re.sub(r"\b(?:Horizon|HORIZON)\b", "FRE LAUNCH", m[0]), text)
    if suffix == ".md":
        text = re.sub(r"\b(?:Horizon|HORIZON)\b", "FRE LAUNCH", text)
    text = identifier(text)
    for index, credit in enumerate(credits):
        text = text.replace(f"__FRE_CREDIT_{index}__", credit)
    return text


def fingerprint():
    from prepare import ico, png
    script = Path(__file__).read_bytes().replace(b"\r\n", b"\n")
    formats = Path(__file__).with_name("native_formats.py").read_bytes().replace(b"\r\n", b"\n")
    return hashlib.sha256(script + formats + native_source.SOURCE_TREE.encode() + png(64) + ico(png(256))).hexdigest()


def patch(text, before, after):
    if text.count(before) != 1:
        raise RuntimeError(f"Native branding anchor drift: {before[:100]!r}")
    return text.replace(before, after, 1)


def build_tree(destination):
    source = REPO / "vendor/fre-rust-source"
    meta = native_source.verify(source)
    destination = Path(destination).resolve()
    if not destination.is_relative_to(REPO / ".audit"):
        raise RuntimeError("Generated native source must stay inside the repository's .audit directory")
    marker = destination / "FRE-BRANDING.json"
    if marker.is_file():
        previous = json.loads(marker.read_text())
        if previous["branding_sha256"] == fingerprint():
            for name, expected in previous["files"].items():
                if hashlib.sha256((destination / name).read_bytes()).hexdigest() != expected:
                    raise RuntimeError(f"Generated native source was edited: {name}")
            print(f"Verified FRE native source: {destination}")
            return destination
    if destination.exists() and not marker.is_file():
        raise RuntimeError("Unrecognized generated directory; preserve it before retrying")
    destination.mkdir(parents=True, exist_ok=True)
    files = {}
    for name, info in meta["files"].items():
        # Upstream attribution and third-party code remain byte-exact.
        legal = Path(name).name in LEGAL or name.startswith("third_party/")
        renamed = name if legal else identifier(name)
        data = (source / name).read_bytes()
        if not legal:
            try:
                text = data.decode("utf-8")
            except UnicodeDecodeError:
                pass
            else:
                text = transform(text, Path(name).suffix)
                if name == "crates/horizon-client/src/discord_presence.rs":
                    text = patch(text, 'pub const DISCORD_APP_ID: &str = "1563857609738690560";',
                        'pub const DISCORD_APP_ID: &str = match option_env!("FRE_DISCORD_APP_ID") { Some(id) => id, None => "" };')
                    text = patch(text, "pub fn start(enabled: bool, automated: bool) {", "pub fn start(enabled: bool, automated: bool) {\n    if DISCORD_APP_ID.is_empty() { return; }")
                if name == "crates/horizon-telemetry/src/lib.rs":
                    text = patch(text, 'resolve(std::env::var("FRE_TELEMETRY_URL").ok().as_deref())', "None")
                if name == "crates/horizon-mods/src/api_level.rs":
                    text = patch(text, "pub const MARK: [u8; 27]", 'pub const MARK: [u8; b"fre-mod-api-level:0000;".len()]')
                    text = patch(text, "let mut i = 25;", "let mut i = mark.len() - 2;")
                    text = patch(text, "let prefix = &MARK[..22];", "let prefix = &MARK[..MARK.len() - 5];")
                    text = patch(text, "w[26] == b';'", "w[MARK.len() - 1] == b';'")
                    text = patch(text, "&w[22..26]", "&w[MARK.len() - 5..MARK.len() - 1]")
                if name == "crates/horizon-client/src/main.rs":
                    text = patch(text, "static MOD_API_LEVEL: [u8; 27]", "static MOD_API_LEVEL: [u8; fre_mods::api_level::MARK.len()]")
                from native_formats import apply as apply_formats
                text = apply_formats(name, text, patch)
                data = text.encode("utf-8")
        # Replace upstream window/taskbar art with FRE's existing original icon.
        if name == "crates/horizon-theme/icons/horizon-64.png":
            from prepare import png
            data = png(64)
        if name == "packaging/icons/horizon.ico":
            from prepare import ico, png
            data = ico(png(256))
        out = destination / renamed
        if renamed in files:
            raise RuntimeError(f"Native branding path collision: {renamed}")
        out.parent.mkdir(parents=True, exist_ok=True)
        if not out.exists() or out.read_bytes() != data:
            out.write_bytes(data)
        if info["mode"] == "100755" and os.name != "nt":
            out.chmod(0o755)
        files[renamed] = hashlib.sha256(data).hexdigest()
    marker.write_text(json.dumps({"source_commit": native_source.REVISION,
        "source_tree": native_source.SOURCE_TREE, "branding_sha256": fingerprint(),
        "files": files}, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"Generated FRE native source: {destination}")
    return destination


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", nargs="?", type=Path)
    parser.add_argument("--fingerprint", action="store_true")
    args = parser.parse_args()
    if args.fingerprint:
        print(fingerprint())
    else:
        build_tree(args.destination or REPO / ".audit/fre-native-source/current")
