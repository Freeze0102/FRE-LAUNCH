"""Own signed Windows update channel; applied only to the exact Modrinth pin."""
import base64
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
CHANNEL = HERE.parents[1] / 'update-channel.json'


def read_channel():
    value = json.loads(CHANNEL.read_text(encoding='utf-8'))
    if not re.fullmatch(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)', value['version']):
        raise ValueError('The Windows stable update channel requires a stable SemVer')
    if not re.fullmatch(r'[\w-]+/[\w.-]+', value['repository']):
        raise ValueError('Invalid release repository')
    key = base64.b64decode(value['publicKey'], validate=True).decode('utf-8')
    if not key.startswith('untrusted comment: minisign public key:') or len(key.splitlines()) != 2:
        raise ValueError('Invalid Tauri updater public key')
    return value


def apply(root):
    from prepare import edit
    channel = read_channel()
    version = channel['version']
    repository = channel['repository']
    app = root / 'apps/app'
    frontend = root / 'apps/app-frontend'
    package = frontend / 'package.json'
    settings = json.loads(package.read_text(encoding='utf-8'))
    if settings['version'] != '1.0.0-local':
        raise RuntimeError('App version anchor changed')
    settings['version'] = version
    package.write_text(json.dumps(settings, indent=2) + '\n', encoding='utf-8')
    edit(app / 'Cargo.toml', 'version = "1.0.0-local"', f'version = "{version}"')
    edit(app / 'Cargo.toml', 'base64 = { workspace = true }',
         'base64 = { workspace = true }\nminisign-verify = "0.2.4"')
    edit(app / 'Cargo.toml', 'default = ["custom-protocol"]', 'default = ["custom-protocol", "updater"]')
    edit(root / 'Cargo.lock', 'name = "theseus_gui"\nversion = "1.0.0-local"',
         f'name = "theseus_gui"\nversion = "{version}"')
    edit(app / 'src/main.rs', 'mod api;', 'mod api;\nmod fre_update_channel;\n#[cfg(feature = "updater")]\nmod fre_update_policy;')
    edit(app / 'src/main.rs', 'false // Do not overwrite this fork with official Modrinth releases',
         'cfg!(all(feature = "updater", target_os = "windows")) && !cfg!(debug_assertions)')
    for source, destination in {
        'fre_updater_impl.rs': app / 'src/updater_impl.rs',
        'fre_update_policy.rs': app / 'src/fre_update_policy.rs',
        'fre-update-test.json': app / 'src/fre-update-test.json',
        'fre-app-updater.ts': frontend / 'src/helpers/fre-app-updater.ts',
    }.items():
        destination.write_text((HERE / source).read_text(encoding='utf-8'), encoding='utf-8')
    (app / 'src/fre_update_channel.rs').write_text(
        f'pub const REPOSITORY: &str = {json.dumps(repository)};\n'
        f'pub const PUBLIC_KEY: &str = {json.dumps(channel["publicKey"])};\n', encoding='utf-8')
    config = app / 'tauri.conf.json'
    settings = json.loads(config.read_text(encoding='utf-8'))
    settings['build']['features'] = ['updater']
    settings['bundle']['createUpdaterArtifacts'] = True
    settings['plugins']['updater'] = {
        'pubkey': channel['publicKey'],
        'endpoints': [f'https://github.com/{repository}/releases/latest/download/latest.json'],
        'windows': {'installMode': 'passive'},
    }
    settings['app']['security']['capabilities'].append('updater')
    config.write_text(json.dumps(settings, indent=2) + '\n', encoding='utf-8')
    # The old release override contained Modrinth's key, host and signing service.
    (app / 'tauri-release.conf.json').write_text(json.dumps({
        'bundle': {'createUpdaterArtifacts': True},
        'build': {'features': ['updater']},
        'plugins': {'updater': settings['plugins']['updater']},
    }, indent=2) + '\n', encoding='utf-8')
    (app / 'capabilities/updater.json').write_text(json.dumps({
        'identifier': 'updater', 'description': 'Check the signed FRE LAUNCH release channel',
        'local': True, 'windows': ['main'], 'permissions': ['updater:allow-check'],
    }, indent=2) + '\n', encoding='utf-8')
    component = frontend / 'src/App.vue'
    text = component.read_text(encoding='utf-8')
    if text.count('const appUpdateDownload = {') != 1 or text.count('async function openModrinthProjectLinkInApp') != 1:
        raise RuntimeError('App updater block anchor drift')
    start = text.index('const appUpdateDownload = {')
    end = text.index('async function openModrinthProjectLinkInApp', start)
    adapter = (HERE / 'fre-update-ui.js').read_text(encoding='utf-8').replace(
        '__FRE_RELEASES_URL__', f'https://github.com/{repository}/releases')
    component.write_text(text[:start] + adapter + '\n' + text[end:], encoding='utf-8')
    edit(component, "import { getVersion } from '@tauri-apps/api/app'",
         "import { getVersion } from '@tauri-apps/api/app'\nimport { createAppUpdater } from '@/helpers/fre-app-updater'")
    edit(component, '\n\tcheckUpdates()\n', '\n')
    edit(component, '\tclearDelayedUpdatePopup()', '\tfreAppUpdater.stop()')
    edit(component, '\tawait unlistenUpdateDownload?.()\n', '')
    edit(component, '\tuseFormatBytes,\n', '')
    edit(component, 'const formatBytes = useFormatBytes()\n', '')
    for unused in ['getNextAppUpdatePopupTime', 'markAppUpdateActionable', 'markAppUpdatePopupShown']:
        edit(component, f'\t{unused},\n', '')
    pill = frontend / 'src/components/ui/app-update-button/index.vue'
    edit(pill, ':disabled="isUpdateDownloading"', ':disabled="isUpdateDownloading || restarting"')
    edit(pill, '() => isUpdateVisible.value && (finishedDownloading.value || metered.value)',
         '() => isUpdateVisible.value')
    edit(pill, '\tmetered,\n', '')
    edit(pill, 'if (isUpdateDownloading.value) {\n\t\treturn\n',
         'if (isUpdateDownloading.value || restarting.value) {\n\t\treturn\n')
    translations = {
        'app.action-bar.reload-to-update': ('Перезапустити й оновити', 'Restart and update'),
        'app.update-popup.reload': ('Перезапустити й оновити', 'Restart and update'),
        'app.update-popup.download-complete': ('Оновлення готове', 'Update ready'),
        'app.update-popup.body.download-complete': (
            'FRE LAUNCH v{version} завантажено й перевірено. Перезапусти лаунчер, щоб оновити, або грай далі — оновлення встановиться після закриття лаунчера.',
            'FRE LAUNCH v{version} is downloaded and verified. Restart to update, or keep playing and it will install when you close the launcher.'),
        'app.update-popup.body.metered': (
            'Доступний FRE LAUNCH v{version}. З’єднання має ліміт трафіку — завантаж оновлення, коли буде зручно.',
            'FRE LAUNCH v{version} is available. Download it when you are ready to use data on this metered connection.'),
        'fre-launch.update.download': ('Завантажити оновлення', 'Download update'),
    }
    for locale, index in [('uk-UA', 0), ('en-US', 1)]:
        path = frontend / f'src/locales/{locale}/index.json'
        data = json.loads(path.read_text(encoding='utf-8'))
        for key, values in translations.items():
            data[key] = {'message': values[index]}
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
