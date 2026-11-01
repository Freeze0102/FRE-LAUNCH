"""Reproducible security updates on top of the immutable upstream lockfile."""
import hashlib
import json
import re
from pathlib import Path


def workspace_overrides(config):
    exceptions = config.get('minimumReleaseAgeExclude', [])
    if any(not re.fullmatch(r'(?:@[^/@\s]+/)?[^/@\s]+@\d+\.\d+\.\d+', name) for name in exceptions):
        raise RuntimeError('Release-age exceptions must name exact security releases')
    return '\nminimumReleaseAgeExclude:\n' + ''.join(
        '  - ' + json.dumps(name) + '\n' for name in exceptions
    ) + '\noverrides:\n' + ''.join(
        '  ' + json.dumps(name) + ': ' + json.dumps(version) + '\n'
        for name, version in config['overrides'].items()
    )


def apply(root):
    here = Path(__file__).resolve().parent
    config = json.loads((here / 'security-dependencies.json').read_text(encoding='utf-8'))
    lock = root / 'pnpm-lock.yaml'
    original = lock.read_text(encoding='utf-8')
    if hashlib.sha256(original.encode()).hexdigest() != config['baseLockSha256']:
        raise RuntimeError('Security lockfile base drift: use the exact pinned Modrinth source')
    workspace = root / 'pnpm-workspace.yaml'
    settings = workspace.read_text(encoding='utf-8')
    if hashlib.sha256(settings.encode()).hexdigest() != config['baseWorkspaceSha256']:
        raise RuntimeError('Security workspace base drift: use the exact pinned Modrinth source')
    patched = (here / 'security-pnpm-lock.yaml').read_text(encoding='utf-8')
    if (hashlib.sha256(patched.encode()).hexdigest() != config['patchedLockSha256'] or
            len(patched) < 100_000 or 'apps/app-frontend:' not in patched):
        raise RuntimeError('Incomplete security lockfile')
    workspace.write_text(settings + workspace_overrides(config), encoding='utf-8')
    lock.write_text(patched, encoding='utf-8')
