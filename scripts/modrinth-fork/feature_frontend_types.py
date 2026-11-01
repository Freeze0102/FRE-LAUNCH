"""Checked source fixes for the pinned desktop frontend and shared packages."""
import json
from pathlib import Path


def apply(root):
    here = Path(__file__).resolve().parent
    root = root.resolve()
    patches = json.loads((here / 'frontend-type-fixes.json').read_text(encoding='utf-8'))
    updated = {}
    for patch in patches:
        target = (root / patch['path']).resolve()
        if not target.is_relative_to(root):
            raise RuntimeError('Frontend patch escaped the source root')
        text = updated.get(target)
        if text is None:
            text = target.read_text(encoding='utf-8')
        if text.count(patch['old']) != patch['count']:
            raise RuntimeError(f"Frontend fix anchor drift: {patch['path']}: {patch['old'][:70]!r}")
        updated[target] = text.replace(patch['old'], patch['new'])
    for target, text in updated.items():
        target.write_text(text, encoding='utf-8')
    for name in ['fre-query-options.ts']:
        destination = root / 'packages/ui/src/utils' / name
        destination.write_text((here / name).read_text(encoding='utf-8'), encoding='utf-8')
    (root / 'apps/app-frontend/src/fre-compat.d.ts').write_text(
        (here / 'fre-compat.d.ts').read_text(encoding='utf-8'), encoding='utf-8')
