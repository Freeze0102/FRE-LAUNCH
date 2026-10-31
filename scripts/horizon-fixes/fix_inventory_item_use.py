#!/usr/bin/env python3
"""Opt-in Horizon Rust source correction for issue #4472.

This script does not build, download, or patch a compiled Horizon executable.
It changes only one known source-code block in a user-supplied Horizon checkout.
It never touches Minecraft worlds, installed profiles or prepared game assets.

Run without --apply for a dry run. With --apply it creates a one-time backup.
"""
from __future__ import annotations

import argparse
import pathlib
import sys

TARGET = pathlib.Path("crates/horizon-client/src/game/remote.rs")
ANCHOR = """    fn remote_tick_using(&mut self) {
        let Some((slot, ticks)) = self.using else {
            return;
        };
        let stack = self.stack_in(slot).cloned();"""
REPLACEMENT = """    fn remote_tick_using(&mut self) {
        let Some((slot, ticks)) = self.using else {
            return;
        };
        // FRE LAUNCH: a GUI must stop use of food, drink, bows, and other items.
        // A menu suppresses input ticks, so the ordinary release handler does
        // not run; the local integrated server must be told to release too.
        if self.screen != Screen::Playing || self.dialog_open() {
            self.using = None;
            self.send(S::PlayerAction(play::PlayerAction {
                action: 6, // ServerboundPlayerAction RELEASE_USE_ITEM
                pos: [0; 3],
                direction: 0,
                sequence: 0,
            }));
            return;
        }
        let stack = self.stack_in(slot).cloned();"""
MARKER = "// FRE LAUNCH: a GUI must stop use of food"


def patched(source: str) -> tuple[str, bool]:
    if MARKER in source:
        if source.count(MARKER) != 1:
            raise ValueError("Unexpected duplicated fix marker")
        return source, False
    n = source.count(ANCHOR)
    if n != 1:
        raise ValueError(
            f"Horizon remote_tick_using layout changed; found {n} exact anchors. "
            "No modifications made; inspect upstream source before updating this patch."
        )
    result = source.replace(ANCHOR, REPLACEMENT, 1)
    if result.count("action: 6, // ServerboundPlayerAction RELEASE_USE_ITEM") != 1:
        raise ValueError("Release packet missing after transformation")
    return result, True


def self_test() -> None:
    sample = "// Horizon Rust source\n" + ANCHOR + "\n}\n"
    fixed, did_change = patched(sample)
    assert did_change
    assert "self.using = None;" in fixed
    assert "self.screen != Screen::Playing || self.dialog_open()" in fixed
    assert "RELEASE_USE_ITEM" in fixed
    second, second_change = patched(fixed)
    assert second == fixed and not second_change
    try:
        patched(sample.replace("remote_tick_using", "changed_upstream_method"))
    except ValueError:
        pass
    else:
        raise AssertionError("Changed upstream anchor was not rejected")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("horizon_checkout", nargs="?", type=pathlib.Path)
    ap.add_argument("--apply", action="store_true",
                    help="Write patch into checked-out Horizon Rust source; saves a .fre-backup first")
    ap.add_argument("--self-test", action="store_true",
                    help="Test patch rules without a repository or build")
    args = ap.parse_args()
    if args.self_test:
        self_test()
        print("PASS: screen-use patch applies once, is repeat-safe and rejects upstream drift.")
        return 0
    if args.horizon_checkout is None:
        ap.error("Pass the Horizon source checkout, or --self-test")
    root = args.horizon_checkout.resolve()
    if not (root / "Cargo.toml").is_file() or not (root / "crates/horizon-client").is_dir():
        ap.error(f"Not a Horizon Rust source checkout: {root}")
    target = root / TARGET
    try:
        before = target.read_text(encoding="utf-8")
        after, changed = patched(before)
    except (ValueError, OSError) as exc:
        print(f"REFUSED: {exc}", file=sys.stderr)
        return 2
    if not changed:
        print(f"Already patched: {target}")
        return 0
    if not args.apply:
        print(f"DRY RUN OK: patch matches exactly one use handler in {target}")
        print("Add --apply to write. No executable has been changed.")
        return 0
    backup = target.with_name(target.name + ".fre-backup")
    if backup.exists():
        print(f"REFUSED: backup exists: {backup}", file=sys.stderr)
        return 2
    backup.write_text(before, encoding="utf-8")
    try:
        target.write_text(after, encoding="utf-8")
    except OSError:
        target.write_text(before, encoding="utf-8")
        raise
    print(f"PATCHED: {target}")
    print(f"Backup: {backup}")
    print("This is SOURCE ONLY. Rebuild Horizon locally and test in a throwaway world.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
