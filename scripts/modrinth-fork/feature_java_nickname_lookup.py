"""Public Java Edition username lookup in the FRE LAUNCH nick selector.

200 = an active Java profile uses the nickname.
404 = no currently resolvable Java profile; NOT guaranteed claimable.
Never conflate Java profile names with game-server registration records.
"""
from pathlib import Path


def exactly(path: Path, before: str, after: str):
    content = path.read_text(encoding="utf-8")
    occurrences = content.count(before)
    if occurrences != 1:
        raise RuntimeError(f"Java nickname lookup patch source drift: {path}, matches={occurrences}: {before[:100]!r}")
    path.write_text(content.replace(before, after, 1), encoding="utf-8")


def apply(root: Path):
    rust_api = root / "packages/app-lib/src/api/minecraft_auth.rs"
    with rust_api.open("a", encoding="utf-8") as out:
        out.write(r"""

/// Public, unauthenticated Java Edition profile lookup.
/// "no_profile" does NOT guarantee that Microsoft offers the name to claim.
#[tracing::instrument]
pub async fn lookup_java_nickname(name: &str) -> crate::Result<&'static str> {
    use anyhow::{Context, bail};
    use reqwest::StatusCode;
    let name = name.trim();
    if !(3..=16).contains(&name.len())
        || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        bail!("Minecraft Java profile lookup requires 3-16 ASCII letters, numbers or underscores");
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .context("Creating Minecraft profile lookup client")?;
    let response = client
        .get(format!(
            "https://api.minecraftservices.com/minecraft/profile/lookup/name/{name}"
        ))
        .send().await.context("Minecraft Java profile lookup unavailable")?;
    match response.status() {
        StatusCode::OK => Ok("used"),
        StatusCode::NOT_FOUND => Ok("no_profile"),
        status => bail!("Minecraft Java profile lookup failed: HTTP {status}"),
    }
}
""")

    tauri_auth = root / "apps/app/src/api/auth.rs"
    exactly(tauri_auth,
        "            has_guest_skin,\n",
        "            has_guest_skin,\n            lookup_java_nickname,\n")
    with tauri_auth.open("a", encoding="utf-8") as out:
        out.write("""

/// Public Java profile existence check, not a third-party server auth check.
#[tauri::command]
pub async fn lookup_java_nickname(name: String) -> Result<String> {
    Ok(theseus::minecraft_auth::lookup_java_nickname(&name)
        .await?.to_string())
}
""")

    build = root / "apps/app/build.rs"
    exactly(build,
        '                        "has_guest_skin",\n',
        '                        "has_guest_skin",\n                        "lookup_java_nickname",\n')

    vue = root / "apps/app-frontend/src/features/minecraft-accounts/accounts-card.vue"
    exactly(vue,
        """            <p v-if="guestNicknameValid" role="status" class="m-0 text-xs text-brand">""",
        """            <p v-if="javaNicknameStatus === 'checking'" role="status" class="m-0 text-xs text-secondary">
                {{ formatMessage(messages.javaChecking) }}
            </p>
            <p v-else-if="javaNicknameStatus === 'used'" role="status" class="m-0 text-xs text-secondary">
                {{ formatMessage(messages.javaUsed) }}
            </p>
            <p v-else-if="javaNicknameStatus === 'no_profile'" role="status" class="m-0 text-xs text-secondary">
                {{ formatMessage(messages.javaNoProfile) }}
            </p>
            <p v-else-if="javaNicknameStatus === 'unavailable'" role="status" class="m-0 text-xs text-secondary">
                {{ formatMessage(messages.javaUnavailable) }}
            </p>
            <p v-if="guestNicknameValid" role="status" class="m-0 text-xs text-brand">""")
    exactly(vue,
        "const guestNicknameValid = computed(() => /^[A-Za-z0-9_]{1,16}$/.test(guestNickname.value))",
        """const guestNicknameValid = computed(() => /^[A-Za-z0-9_]{1,16}$/.test(guestNickname.value))
type JavaNickStatus = 'idle' | 'checking' | 'used' | 'no_profile' | 'unavailable'
const javaNicknameStatus = ref<JavaNickStatus>('idle')
let javaNickLookupTimer: ReturnType<typeof setTimeout> | undefined
let javaNickLookupSequence = 0
watch([guestNickname, useSavedAccount], ([name, accountMode]) => {
    javaNickLookupSequence++
    if (javaNickLookupTimer !== undefined) clearTimeout(javaNickLookupTimer)
    javaNickLookupTimer = undefined
    javaNicknameStatus.value = 'idle'
    // 1-2-character offline nicks can still launch; licensed Java profiles
    // require 3-16 characters for the public lookup.
    if (accountMode || !/^[A-Za-z0-9_]{3,16}$/.test(name)) return
    javaNicknameStatus.value = 'checking'
    const sequence = javaNickLookupSequence
    javaNickLookupTimer = setTimeout(async () => {
        try {
            const result = await invoke<string>('plugin:auth|lookup_java_nickname', { name })
            if (sequence !== javaNickLookupSequence) return
            javaNicknameStatus.value = result === 'used' || result === 'no_profile'
                ? result : 'unavailable'
        } catch (error) {
            if (sequence !== javaNickLookupSequence) return
            javaNicknameStatus.value = 'unavailable'
            console.warn('Java profile lookup failed:', error)
        }
    }, 700)
}, { immediate: true })
onUnmounted(() => {
    javaNickLookupSequence++
    if (javaNickLookupTimer !== undefined) clearTimeout(javaNickLookupTimer)
})""")
    exactly(vue,
        "    guestFormatOk: { id: 'fre-launch.guest.format-ok', defaultMessage: 'Nickname is valid for offline launch.' },",
        """    guestFormatOk: { id: 'fre-launch.guest.format-ok', defaultMessage: 'Nickname is valid for offline launch.' },
    javaChecking: { id: 'fre-launch.guest.java-checking', defaultMessage: 'Checking Minecraft Java name...' },
    javaUsed: { id: 'fre-launch.guest.java-used', defaultMessage: 'This name belongs to an active Minecraft Java profile.' },
    javaNoProfile: { id: 'fre-launch.guest.java-no-profile', defaultMessage: 'No active Java profile found. Name availability is not guaranteed.' },
    javaUnavailable: { id: 'fre-launch.guest.java-unavailable', defaultMessage: 'Minecraft name lookup is temporarily unavailable.' },""")
    print("Installed debounced Java nickname lookup with honest 200/404/unknown statuses. Guest launch remains independent.")


if __name__ == "__main__":
    raise SystemExit("Import through prepare.py")
