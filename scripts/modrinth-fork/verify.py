#!/usr/bin/env python3
"""Zero-CI smoke checks for generated FRE LAUNCH Modrinth source."""
import argparse
import hashlib
import json
import py_compile
import re
import uuid
from pathlib import Path

def verify(root):
    root=root.resolve()
    path=Path(__file__).resolve().parent
    for item in ("prepare.py","feature_local_accounts.py","feature_skin_notice.py","feature_offline_skins.py","feature_local_skin_preview.py","feature_ui_polish.py","feature_guest_launch.py","feature_java_nickname_lookup.py","feature_ukrainian.py","verify.py"):
        py_compile.compile(str(path/item),doraise=True)
    expected={"Notch":"b50ad385-829d-3141-a216-7e7d7539ba7f",
              "Steve":"5627dd98-e6be-3c21-b8a8-e92344183641"}
    for nickname,wanted in expected.items():
        actual=str(uuid.UUID(bytes=hashlib.md5(("OfflinePlayer:"+nickname).encode()).digest(),version=3))
        assert actual==wanted, (nickname,actual,wanted)

    app_en=json.loads((root/"apps/app-frontend/src/locales/en-US/index.json").read_text(encoding="utf-8"))
    app_uk=json.loads((root/"apps/app-frontend/src/locales/uk-UA/index.json").read_text(encoding="utf-8"))
    ui_en=json.loads((root/"packages/ui/src/locales/en-US/index.json").read_text(encoding="utf-8"))
    ui_uk=json.loads((root/"packages/ui/src/locales/uk-UA/index.json").read_text(encoding="utf-8"))
    for label,english,ukrainian,field in (
        ("App",app_en,app_uk,"message"),("UI",ui_en,ui_uk,"defaultMessage")
    ):
        missing=set(english)-set(ukrainian)
        if missing: raise AssertionError(f"{label}: missing Ukrainian keys: {sorted(missing)}")
        empty=[key for key in english if not str(ukrainian[key].get(field,"")).strip()]
        if empty: raise AssertionError(f"{label}: empty translated strings: {empty}")
    from feature_ukrainian import APP,UI,NEW
    for label,english,ukrainian,mapping,field in (
        ("App",app_en,app_uk,APP,"message"),
        ("UI",ui_en,ui_uk,UI,"defaultMessage"),
    ):
        for key,ua in mapping.items():
            assert ukrainian[key][field]==ua, f"{label}: {key} translation missing"
            en=english[key][field]
            def names(value):
                # ICU plural/select branches also use braces for literal choices.
                # Only outer substitutions correspond to interpolation variables.
                used=set()
                depth=0
                for i,char in enumerate(value):
                    if char=="{":
                        if depth==0:
                            match=re.match(r"[A-Za-z][A-Za-z0-9_]*",value[i+1:])
                            if match:
                                used.add(match.group(0))
                        depth+=1
                    elif char=="}":
                        depth=max(0,depth-1)
                return used
            if names(en)!=names(ua):
                raise AssertionError(f"{key}: ICU variables mismatch: {names(en)} != {names(ua)}")
    for key,(ua,en) in NEW.items():
        assert app_en[key]["message"]==en
        assert app_uk[key]["message"]==ua

    checks={
        "apps/app/src/api/auth.rs":["create_offline_user","save_local_skin","has_local_skin","get_local_skin","save_guest_skin","has_guest_skin","lookup_java_nickname","theseus::offline_skins::save_guest_skin","theseus::minecraft_auth::lookup_java_nickname","Sign in to Minecraft with Microsoft"],
        "apps/app/build.rs":["create_offline_user","save_local_skin","has_local_skin","save_guest_skin","has_guest_skin","instance_run_guest"],
        "packages/app-lib/src/api/minecraft_auth.rs":["pub async fn create_offline_user","pub fn guest_credentials","pub async fn lookup_java_nickname","api.minecraftservices.com/minecraft/profile/lookup/name","StatusCode::NOT_FOUND","OfflinePlayer:","md5::compute","mark_logged_into_minecraft"],
        "packages/app-lib/src/state/minecraft_auth.rs":["pub fn is_offline_profile","fre-offline:"],
        "packages/app-lib/src/launcher/args.rs":['access_token.starts_with("fre-offline:")'],
        "apps/app-frontend/src/features/minecraft-accounts/accounts-card.vue":["OfflineProfileCreator", "fre-launch-offline-account-created", "confirmRemove", "messages.microsoft", "useSavedAccount", "guestNickname", "fre-launch.guest.use-account", "onGuestSkinFilePicked", "plugin:auth|save_guest_skin", "plugin:auth|lookup_java_nickname", "javaNicknameStatus", "javaNickLookupSequence", "guestNicknameValid", "guestNicknameIsSaved", "messages.guestServerCheck"],
        "apps/app-frontend/src/features/minecraft-accounts/offline-profile-creator.vue":["v-model.trim","create_offline_user","@submit.prevent"],
        "apps/app-frontend/src/helpers/auth.js":["plugin:auth|create_offline_user"],
        "apps/app-frontend/src/helpers/instance.ts":["plugin:instance|instance_run_guest","fre-launch-guest-nickname"],
        "apps/app-frontend/src/helpers/skins.ts":["if (skin.texture.startsWith('data:image/png;base64,'))"],
        "apps/app-frontend/src/pages/Skins.vue":["isLocalAccount", "fre-launch.skins.local-notice", "onLocalSkinFileSelected", "plugin:auth|save_local_skin", "plugin:auth|get_local_skin", "localSkinHelpExpanded", "skinFingerprint"],
        "apps/app-frontend/src/components/ui/skin/VirtualSkinSectionList.vue":["allowOfflineSelection"],
        "packages/app-lib/src/api/offline_skins.rs":["pub async fn save_local_skin","pub async fn save_guest_skin","pub async fn has_guest_skin","pub async fn get_local_skin","pub async fn prepare_instance_skin","CustomSkinLoader.fre-launch-backup.json","validate_skin_png"],
        "packages/app-lib/src/launcher/mod.rs":["crate::api::offline_skins::prepare_instance_skin"],
        "packages/app-lib/src/api/instance/run.rs":["pub async fn run_guest","run_internal","guest_credentials: Option<Credentials>"],
        "packages/app-lib/src/api/instance.rs":["run_guest"],
        "apps/app/src/api/instance.rs":["pub async fn instance_run_guest","theseus::instance::run_guest"],
        "packages/app-lib/src/api/mod.rs":["pub mod offline_skins;"],
        "apps/app-frontend/src/app/shell/title-bar/index.vue":["FRE LAUNCH"],
        "apps/app-frontend/src/components/ui/WelcomeScreen.vue":["OfflineProfileCreator", "fre-launch.welcome-screen.title"],
    }
    for relative,snippets in checks.items():
        data=(root/relative).read_text(encoding="utf-8")
        for snippet in snippets:
            if snippet not in data: raise AssertionError(f"{relative}: missing {snippet}")
    assert "theseus::api::offline_skins::" not in (root/"apps/app/src/api/auth.rs").read_text(encoding="utf-8"), "Private Theseus API path causes Rust E0603" 
    # Vue/Vite previously failed at runtime even though preliminary source
    # checks passed: duplicate invoke imports and a missing message comma.
    accounts_vue=(root/"apps/app-frontend/src/features/minecraft-accounts/accounts-card.vue").read_text(encoding="utf-8")
    assert accounts_vue.count("import { invoke } from '@tauri-apps/api/core'")==1, "Duplicate Tauri invoke import breaks Vue compiler"
    assert re.search(r"javaUnavailable:\\s*\\{[^\\n]*\\},\\s*\\n\\s*guestAlreadySaved:", accounts_vue), "Missing comma after Java nickname message breaks Vue compiler"

    nickname_api=(root/"packages/app-lib/src/api/minecraft_auth.rs").read_text(encoding="utf-8")
    assert 'StatusCode::OK => Ok("used")' in nickname_api and 'StatusCode::NOT_FOUND => Ok("no_profile")' in nickname_api, "Public Java lookup must distinguish taken and unregistered profile"
    assert 'StatusCode::TOO_MANY_REQUESTS' not in nickname_api, "Rate limiting must not be mistaken for a free nickname"
    guest_skins=(root/"packages/app-lib/src/api/offline_skins.rs").read_text(encoding="utf-8")
    assert "guest_credentials(nickname)" in guest_skins, "Guest skins must use the same UUID as guest Minecraft launches"
    assert "save_skin_bytes(credentials.offline_profile.id" in guest_skins, "Guest PNG must be keyed by game UUID"
    assert "list.insert(0, local);" in guest_skins, "CustomSkinLoader should prefer the user's local PNG"
    guest_runner=(root/"packages/app-lib/src/api/instance/run.rs").read_text(encoding="utf-8")
    assert "run_internal(instance_id, quick_play_type, Some(guest))" in guest_runner, "Guest launches must use unsaved credentials"
    assert "Credentials::get_default_credential(&state.pool)" in guest_runner, "Account-mode launches must retain default account"
    assert "run_credentials(instance_id, &credentials, quick_play_type).await" in guest_runner, "Guest nickname must reach Minecraft process"
    assert "upsert(" not in guest_runner.split("pub async fn run_guest(", 1)[1].split("async fn run_credentials(",1)[0], "Guest mode must not write credentials to database"
    from feature_ukrainian import NEW
    for key in ("fre-launch.guest.format-ok", "fre-launch.guest.already-saved", "fre-launch.guest.server-check"):
        assert key in NEW, f"Missing guest nickname validation translation: {key}"

    config=json.loads((root/"apps/app/tauri.conf.json").read_text(encoding="utf-8"))
    assert config["identifier"]=="app.frelaunch.modrinthfork"
    assert "ads" not in config["app"]["security"]["capabilities"]
    assert config["productName"]=="FRE LAUNCH"
    app=(root/"apps/app-frontend/src/App.vue").read_text(encoding="utf-8")
    assert 'to="/hosting/manage"' not in app
    assert ':show-ad="false"' in app
    assert 'watch(\n\t[stateInitialized, showAd, adConsentAvailable],' not in app
    assert (root/"packages/app-lib/.env").is_file(), "Missing production API environment file"
    assert (root/"FRE_LAUNCH_FORK.md").is_file()
    print(f"Source check passed: {len(app_en)} App messages and {len(ui_en)} UI messages covered in UK + EN.")
    print("Nickname path, unique identifier, advertising and hosting checks passed.")
    print("Note: this does not replace real Rust/Vue/Windows compilation or Minecraft runtime testing.")

if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("source",type=Path)
    args=parser.parse_args()
    verify(args.source)
