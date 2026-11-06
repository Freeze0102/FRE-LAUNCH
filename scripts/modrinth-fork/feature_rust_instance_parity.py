"""Native instance parity with the existing account, worlds and console UI."""
from pathlib import Path
from feature_fre_rust import exactly

def apply(root: Path):
    here=Path(__file__).parent
    app=root/'apps/app'
    frontend=root/'apps/app-frontend/src'
    for filename in ('RustWorlds.vue','RustLogs.vue'):
        (frontend/'components/ui'/filename).write_bytes((here/filename).read_bytes())
    (app/'src/api/fre_rust_instance.rs').write_bytes((here/'fre_rust_instance.rs').read_bytes())
    exactly(app/'build.rs','"rust_logs", "rust_open_folder"','"rust_logs", "rust_log_sources", "rust_worlds", "rust_server_edit", "rust_open_folder"')
    (app/'src/api/fre_native_build.rs').write_bytes((here/'fre_native_build.rs').read_bytes())
    toolchain=(here.resolve().parents[1]/'vendor/fre-rust-source/launcher/src/rust_toolchain.rs').read_text(encoding='utf-8').split('\n#[cfg(test)]\nmod tests')[0]
    toolchain=toolchain.replace('use crate::backend;', 'use super::backend;').replace('crate::process::','super::process::')
    (app/'src/api/fre_native_toolchain.rs').write_text(toolchain,encoding='utf-8')
    exactly(app/'Cargo.toml','toml = { workspace = true }','toml = { workspace = true }\ntar = { workspace = true }\nflate2 = { workspace = true }')
    exactly(app/'Cargo.toml', 'features = ["rustls-tls", "stream"]', 'features = ["rustls-tls", "stream", "blocking"]')
    worlds=root/'packages/app-lib/src/api/worlds.rs'
    worlds.write_text(worlds.read_text(encoding='utf-8')+'\n'+(here/'fre_native_worlds_bridge.rs').read_text(encoding='utf-8'),encoding='utf-8')
    exactly(root/'packages/app-lib/src/api/instance/synced_servers/mod.rs', 'pub(crate) use self::codec::server_data;', 'pub(crate) use self::codec::{server_data, read_servers, write_servers};')
    item=frontend/'components/ui/world/WorldItem.vue'
    exactly(item,'\t\tworld: World','\t\tnativeInstance?: boolean\n\t\tworld: World')
    exactly(item,"\t\tid: 'edit',", "\t\tid: 'edit',")
    text=item.read_text(encoding='utf-8')
    start=text.index("\t\tid: 'edit',");end=text.index("\t\tid: 'open-folder',",start)
    text=text[:start]+text[start:end].replace('shown: !props.instanceId','shown: !props.instanceId && !props.nativeInstance')+text[end:]
    start=text.index("\t\tid: 'delete',");end=text.index('\n\t},',start)
    text=text[:start]+text[start:end].replace('shown: !props.instanceId',"shown: !props.instanceId && (!props.nativeInstance || props.world.type === 'server')")+text[end:]
    item.write_text(text,encoding='utf-8')
    action=frontend/'app/shell/title-bar/action-bar.vue'
    exactly(action,"import { convertFileSrc } from '@tauri-apps/api/core'", "import { convertFileSrc, invoke } from '@tauri-apps/api/core'\nimport { listen } from '@tauri-apps/api/event'")
    exactly(action,'interface RunningProcess {', 'interface RunningProcess {\n\tfreRustId?: string')
    exactly(action,':to="`/instance/${encodeURIComponent(selectedProcess.instance.id)}`"', ':to="selectedProcess.freRustId ? `/rust-instance/native/${encodeURIComponent(selectedProcess.freRustId)}` : `/instance/${encodeURIComponent(selectedProcess.instance.id)}`"')
    exactly(action,'const refresh = async () => {','let nativeProcessGeneration = 0\nlet nativeProcessDisposed = false\nlet nativeProcessUnlisten: (() => void) | undefined\nlet nativeProcessTimer: ReturnType<typeof setInterval> | undefined\nconst refresh = async () => {\n\tconst request = ++nativeProcessGeneration')
    exactly(action,'\tcurrentProcesses.value = processes','\tconst nextProcesses: RunningProcess[] = processes')
    exactly(action,'\tif (!selectedProcess.value || !currentProcesses.value.includes(selectedProcess.value)) {', '''    try {
        const [ids, profiles] = await Promise.all([
            invoke<string[]>('plugin:fre-rust|rust_running'),
            invoke<Array<{id:string;name:string;iconPath?:string}>>('plugin:fre-rust|rust_profiles'),
        ])
        for (const id of ids) {
            const profile=profiles.find(item=>item.id===id)
            if (profile) nextProcesses.push({ uuid:'rust:'+id, instance_id:'rust:'+id, freRustId:id, instance: {id:'rust:'+id,name:profile.name,loader:'Rust',game_version:'26.3',icon_path:profile.iconPath??null} as unknown as GameInstance })
        }
    } catch { /* Java process controls remain available if native status is unavailable. */ }
    if (nativeProcessDisposed || request !== nativeProcessGeneration) return
    currentProcesses.value = nextProcesses
    selectedProcess.value = currentProcesses.value.find(item=>item.uuid===selectedProcess.value?.uuid) ?? currentProcesses.value[0]
\tif (!selectedProcess.value || !currentProcesses.value.includes(selectedProcess.value)) {''')
    exactly(action,"onMounted(() => {\n\twindow.addEventListener('offline'", """onMounted(() => {
    void listen('fre-rust-process-changed',()=>{void refresh()}).then(unlisten=>{if(nativeProcessDisposed)unlisten();else nativeProcessUnlisten=unlisten}).catch(()=>{})
    nativeProcessTimer=setInterval(()=>{void refresh()},5000)
\twindow.addEventListener('offline'""")
    exactly(action, 'const stop = async (process: RunningProcess) => {\n\ttry {', '''const stop = async (process: RunningProcess) => {
\ttry {
        if (process.freRustId) {
            await invoke('plugin:fre-rust|rust_stop', {profileId:process.freRustId})
            await refresh()
            return
        }''')
    exactly(action,'\trouter.push(`/instance/${encodeURIComponent(selectedInstanceId)}/logs`)','\tif (selectedInstanceId.startsWith("rust:")) {\n\t\tvoid router.push({path:"/rust-instance/native/"+encodeURIComponent(selectedInstanceId.slice(5)),query:{tab:"logs"}})\n\t\treturn\n\t}\n\trouter.push(`/instance/${encodeURIComponent(selectedInstanceId)}/logs`)')
    text=action.read_text(encoding='utf-8')
    marker='onBeforeUnmount(() => {'
    if text.count(marker)!=1:raise RuntimeError('Action bar cleanup anchor drift')
    text=text.replace(marker,marker+'\n    nativeProcessDisposed=true;nativeProcessGeneration++;nativeProcessUnlisten?.();if(nativeProcessTimer)clearInterval(nativeProcessTimer)',1)
    action.write_text(text,encoding='utf-8')
    account=frontend/'features/minecraft-accounts/accounts-card.vue'
    exactly(account,'import { computed,',"import { useRoute } from 'vue-router'\nimport { computed,")
    exactly(account,'const useSavedAccount = ref(',"const freAccountRoute = useRoute()\nconst freNativeAccountPage = computed(() => freAccountRoute.path.startsWith('/rust-instance/'))\nconst useSavedAccount = ref(")
    exactly(account,'''        <label class="flex items-center gap-2 text-sm text-primary cursor-pointer">
            <input v-model="useSavedAccount" type="checkbox" class="accent-brand" />
            <span>{{ formatMessage(messages.useAccount) }}</span>
        </label>''', '''        <div class="flex gap-1" role="group" :aria-label="formatMessage(messages.accountMode)">
            <Button :type="useSavedAccount ? 'colored' : 'quiet'" :color="useSavedAccount ? 'brand' : undefined" @click="useSavedAccount = true">{{ formatMessage(messages.accountMode) }}</Button>
            <Button :type="!useSavedAccount ? 'colored' : 'quiet'" :color="!useSavedAccount ? 'brand' : undefined" @click="useSavedAccount = false">{{ formatMessage(messages.nicknameMode) }}</Button>
        </div>''')
    exactly(account,'<div class="mt-2 rounded-xl border border-solid border-surface-5 bg-button-bg p-3">','<div v-if="!freNativeAccountPage" class="mt-2 rounded-xl border border-solid border-surface-5 bg-button-bg p-3">')
    text=account.read_text(encoding='utf-8').replace('v-if="useSavedAccount && accounts.length === 0"','v-if="(useSavedAccount || freNativeAccountPage) && accounts.length === 0"').replace('v-else-if="useSavedAccount"','v-else-if="useSavedAccount || freNativeAccountPage"')
    text=text.replace('<OfflineProfileCreator />','<OfflineProfileCreator v-if="!freNativeAccountPage" />').replace('v-if="useSavedAccount && accounts.length > 0"','v-if="!freNativeAccountPage && useSavedAccount && accounts.length > 0"')
    text=text.replace("    useAccount: { id: 'fre-launch.guest.use-account', defaultMessage: 'Use saved account' },", "    accountMode: { id: 'fre-launch.account.mode.account', defaultMessage: 'Account' },\n    nicknameMode: { id: 'fre-launch.account.mode.nickname', defaultMessage: 'Nickname' },")
    text=text.replace('accounts.length', 'visibleAccounts.length').replace('account in accounts', 'account in visibleAccounts')
    text=text.replace('const selectedAccount = computed(() =>\n\taccounts.value.find', 'const visibleAccounts = computed(() => freNativeAccountPage.value ? accounts.value.filter(account => !account.access_token?.startsWith("fre-offline:")) : accounts.value)\nconst selectedAccount = computed(() =>\n\tvisibleAccounts.value.find')
    account.write_text(text,encoding='utf-8')
"""
Generator changes remain in the canonical templates, never in pinned vendor.
"""
