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
    for item in ("prepare.py","feature_local_accounts.py","feature_skin_notice.py","feature_offline_skins.py","feature_local_skin_preview.py","feature_ui_polish.py","feature_guest_launch.py","feature_java_nickname_lookup.py","feature_remove_modrinth_login.py","feature_brand_presence.py","feature_curseforge.py","feature_horizon.py","feature_fre_rust.py","feature_rust_polish.py","feature_ukrainian.py","verify.py"):
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
    from feature_ukrainian import POLISH_EXISTING_APP, POLISH_EXISTING_UI
    for key,expected_uk in POLISH_EXISTING_APP.items():
        assert app_uk[key]["message"]==expected_uk, f"Ukrainian app polish missing: {key}"
    for key,expected_uk in POLISH_EXISTING_UI.items():
        assert ui_uk[key]["defaultMessage"]==expected_uk, f"Ukrainian UI polish missing: {key}"


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
        "apps/app-frontend/src/features/onboarding/checklist.vue":['v-if="isReady && showChecklist && !hasCreatedInstance"'],
        "apps/app-frontend/src/features/settings/app-settings-modal/index.vue":['tab.category !== tabCategories.account'],
        "apps/app-frontend/src/app/shell/sidebar/index.vue":['v-if="false && showFriendsList"'],
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
    assert re.search(r"javaUnavailable:\s*\{[^\n]*\},\s*\n\s*guestAlreadySaved:", accounts_vue), "Missing comma after Java nickname message breaks Vue compiler"

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
    assert 'v-tooltip.right="profileButtonTooltip"' not in app, "Modrinth account login button still present in launcher"
    assert '<ModrinthAccountRequiredModal ref="modrinthLoginModal"' in app, "Keep internal Modrinth auth flow available for platform-only features"
    onboarding=(root/"apps/app-frontend/src/features/onboarding/checklist.vue").read_text(encoding="utf-8")
    assert "id: 'login-modrinth'," not in onboarding.split("const steps = computed(",1)[1].split("const accordionButtonClass",1)[0], "Onboarding must not require Modrinth login"

    # CurseForge never borrows Modrinth's authentication and never silently
    # installs content from incompatible Minecraft/modloader combinations.
    cf_backend=(root/"apps/app/src/api/curseforge.rs").read_text(encoding="utf-8")
    cf_frontend=(root/"apps/app-frontend/src/components/ui/CurseForgeBrowser.vue").read_text(encoding="utf-8")
    cf_browse=(root/"apps/app-frontend/src/pages/Browse.vue").read_text(encoding="utf-8")
    java_instance_layout=(root/"apps/app-frontend/src/pages/instance/layout.vue").read_text(encoding="utf-8")
    cf_mod=(root/"apps/app/src/api/mod.rs").read_text(encoding="utf-8")
    cf_main=(root/"apps/app/src/main.rs").read_text(encoding="utf-8")
    cf_build=(root/"apps/app/build.rs").read_text(encoding="utf-8")
    cf_cargo=(root/"apps/app/Cargo.toml").read_text(encoding="utf-8")
    cf_capabilities=json.loads((root/"apps/app/capabilities/plugins.json").read_text(encoding="utf-8"))
    assert "fre-curseforge:default" in cf_capabilities["permissions"], "CurseForge Tauri commands must be explicitly permitted"
    for label, source, snippets in (
        ("CurseForge backend", cf_backend, ["FRE_CURSEFORGE_API_KEY", "x-api-key", "allow_mod_distribution", "is_available", "validated_api_key",
            "gameVersion", "modLoaderType", "file_length", "verify_hash", "add_project_from_path",
            "sync_content_files", "cf_search", "cf_install", "loader_type", "MINECRAFT_MODPACK_CLASS_ID", "category_id"]),
        ("CurseForge browser", cf_frontend, ["plugin:fre-curseforge|cf_search",
            "plugin:fre-curseforge|cf_install", "instanceId", "apiKey", "contentType", "isModpack", "v-if=\"!isModpack\"",
            "async function doSearch()", "const sequence = ++searchSequence",
            "class=", "results.value = found"]),
        ("Browse source selector", cf_browse, ["CurseForgeBrowser", "freContentSource",
            "route.query.source === 'curseforge'", "Modrinth</button>", "CurseForge</button>",
            "projectType === 'modpack'", ":content-type"]),
        ("CurseForge Rust API module", cf_mod, ["pub mod curseforge;"]),
        ("CurseForge Rust plugin", cf_main, [".plugin(api::curseforge::init())"]),
        ("CurseForge Tauri permissions", cf_build, ['"fre-curseforge"', '"cf_search"', '"cf_install"']),
        ("CurseForge hash library", cf_cargo, ["sha1 = { workspace = true }"]),
    ):
        for snippet in snippets:
            assert snippet in source, f"{label}: missing {snippet}"
    assert "CurseForge" in cf_frontend
    assert "openUrl(" not in cf_frontend, "The in-app catalog must not redirect search to a website"
    assert "curseForgeWebsiteSearchUrl" not in cf_frontend, "Website fallback is no longer supported"
    assert "browseWithoutKey" not in cf_frontend, "Searching inside the launcher must remain in-app"
    assert "api.curseforge.com" in cf_backend
    assert "https://api.curseforge.com" in cf_backend
    assert "npm install" not in cf_backend
    for key in ("fre-launch.cf.source", "fre-launch.cf.api-key", "fre-launch.cf.install-error",
                "fre-launch.cf.for-instance", "fre-launch.cf.pack-intro", "fre-launch.cf.pack-search",
                "fre-launch.cf.direct-intro", "fre-launch.cf.site-search", "fre-launch.cf.import-file"):
        assert key in app_uk and key in app_en, f"CurseForge translation missing: {key}"

    # The new implementation does not invoke a foreign Minecraft launcher.
    rust_backend=(root/"apps/app/src/api/fre_rust.rs").read_text(encoding="utf-8")
    rust_setup=(root/"apps/app/src/api/fre_rust_setup.rs").read_text(encoding="utf-8")
    rust_mods=(root/"apps/app-frontend/src/components/ui/RustNativeMods.vue").read_text(encoding="utf-8")
    rust_quick=(root/"apps/app-frontend/src/components/ui/QuickInstanceSwitcher.vue").read_text(encoding="utf-8")
    rust_panel=(root/"apps/app-frontend/src/components/ui/FreRustPanel.vue").read_text(encoding="utf-8")
    rust_library=(root/"apps/app-frontend/src/components/ui/FreRustLibrary.vue").read_text(encoding="utf-8")
    unified_rust=(root/"apps/app-frontend/src/components/ui/library/instance-group/RustInstanceGrid.vue").read_text(encoding="utf-8")
    rust_detail=(root/"apps/app-frontend/src/pages/RustInstancePage.vue").read_text(encoding="utf-8")
    rust_files=(root/"apps/app-frontend/src/components/ui/RustInstanceFiles.vue").read_text(encoding="utf-8")
    rust_routes=(root/"apps/app-frontend/src/routes.js").read_text(encoding="utf-8")
    unified_group=(root/"apps/app-frontend/src/components/ui/library/instance-group/index.vue").read_text(encoding="utf-8")
    rust_stage=(root/"packages/ui/src/components/flows/creation-flow-modal/components/SetupTypeStage.vue").read_text(encoding="utf-8")
    rust_index=(root/"apps/app-frontend/src/pages/Index.vue").read_text(encoding="utf-8")
    rust_main=(root/"apps/app/src/main.rs").read_text(encoding="utf-8")
    rust_build=(root/"apps/app/build.rs").read_text(encoding="utf-8")
    rust_app=(root/"apps/app-frontend/src/App.vue").read_text(encoding="utf-8")
    rust_permissions=json.loads((root/"apps/app/capabilities/plugins.json").read_text(encoding="utf-8"))
    assert "fre-rust:default" in rust_permissions["permissions"]
    assert "const MAX_ICON_BYTES:" in rust_backend and "fn prepare_icon(" in rust_backend
    assert "icon_path: Option<String>" in rust_backend and "icon_bytes: Option<Vec<u8>>" in rust_backend
    assert 'profile.icon_path = ["icon.png", "icon.jpg", "icon.webp"]' in rust_backend
    # Old Horizon adapter may remain installed for archival compatibility,
    # but it must never be invoked by the active FRE Rust UI.
    assert "fre-horizon:default" in rust_permissions["permissions"]
    assert 'pub mod fre_rust;' in (root/"apps/app/src/api/mod.rs").read_text(encoding="utf-8")
    assert "pub mod horizon;" in (root/"apps/app/src/api/mod.rs").read_text(encoding="utf-8")
    for source, markers in (
        (rust_backend, [
            'Builder::new("fre-rust")', "async fn rust_create_profile",
            "async fn rust_import_resources", "async fn rust_launch", "async fn rust_rename_profile", "async fn rust_delete_profile", "async fn rust_profiles", "async fn rust_runtime",
            "async fn rust_logs", '"fre-rust-client.exe"', 'Command::new(client)',
            '"--resources"', '"--home"', '"--game-dir"', '"--session-stdin"',
            "theseus::minecraft_auth::fre_rust_default_credentials", "online_profile_fresh",
            "verify_native_client", "Sha256::new", "RUNNING_PROFILES",
            "profile_id_valid", "read_profile", "safe_game_folder", "rust_open_folder", "rust_list_files", "list_game_files", "explorer.exe",
            "rust_import_mod", "owned_mods", "ensure_rust_icon", "default_rust_icon",
        ]),
        (rust_panel, [
            "plugin:fre-rust|rust_import_resources", "plugin:fre-rust|rust_create_profile", "plugin:fre-rust|rust_launch",
            "plugin:fre-rust|rust_profiles", "plugin:fre-rust|rust_runtime",
            "fre-rust-library-refresh", "chooseSourceFolder", "@tauri-apps/plugin-dialog", "openProfileFiles", "rust_open_folder",
        ]),
        (rust_library, [
            "plugin:fre-rust|rust_profiles", "plugin:fre-rust|rust_launch",
            "showFreRustWizard", "fre-rust-library-refresh", "fre-rust.library.empty", "openFiles", "rust_list_files", "rust_open_folder",
        ]),
        (rust_index, ["<LibrarySection :instances=\"instances\" />"]),
        (rust_app, [
            "import FreRustPanel", "<FreRustPanel", "showFreRustWizard",
            "provide('showFreRustWizard', showFreRustWizard)",
            "handleFreRustOrJavaCreate", "plugin:fre-rust|rust_create_profile",
            ":available-loaders=", "'rust'", "@create=\"handleFreRustOrJavaCreate\"",
        ]),
        (rust_stage, ["Minecraft (Rust)"]),
        (rust_main, [".plugin(api::fre_rust::init())", ".plugin(api::horizon::init())"]),
        (rust_build, ['"fre-rust"', '"fre-horizon"', '"rust_create_profile"', '"rust_rename_profile"', '"rust_delete_profile"', '"rust_import_mod"', '"rust_launch"',
            '"rust_open_folder"', '"rust_list_files"', '"horizon_open_folder"', '"horizon_list_files"']),
    ):
        for marker in markers:
            assert marker in source, f"Standalone Rust Minecraft integration missing: {marker}"
    # Public-API boundary: Tauri cannot access Theseus's private state
    # module or pool. Keep the account refresh inside the Theseus auth API.
    auth_api = (root / "packages/app-lib/src/api/minecraft_auth.rs").read_text(encoding="utf-8")
    assert "pub async fn fre_rust_default_credentials() -> crate::Result<Option<Credentials>>" in auth_api
    assert "Credentials::get_default_credential(&state.pool).await" in auth_api
    assert "theseus::state::" not in rust_backend and "state.pool" not in rust_backend
    assert "use theseus::{State" not in rust_backend
    assert rust_backend.count("theseus::minecraft_auth::fre_rust_default_credentials()") == 2
    assert 'standalone_setup::prepare(' in rust_backend
    assert 'prepare_exe_sha256: String' in rust_backend
    assert 'preparer_hash.finalize()' in rust_backend
    assert 'root.join("fre-rust-prepare.exe").is_file()' in rust_backend
    assert 'fre-rust-prepare.exe' in rust_setup
    assert 'piston-meta.mojang.com' in rust_setup and 'resources.download.minecraft.net' in rust_setup
    assert 'horizon_install_launcher' not in rust_backend and 'prepare_for_native_rust' not in rust_backend
    assert 'native_mods_for_profile' not in rust_backend
    for native_ui in (unified_rust, rust_detail, rust_files, rust_quick, rust_mods, rust_panel):
        assert 'plugin:fre-horizon|' not in native_ui, "Rust UI must not require the Horizon Launcher"
    assert 'plugin:fre-rust|rust_import_mod' in rust_mods
    for key in (
        "fre-rust.setup.title", "fre-rust.setup.create", "fre-rust.setup.error",
        "fre-rust.setup.import", "fre-rust.setup.source", "fre-rust.setup.browse",
        "fre-rust.library.title", "fre-rust.library.play", "fre-rust.library.error", "fre-rust.library.empty",
        "fre-launch.rust.library.empty", "fre-rust.setup.files", "fre-rust.library.files",
        "fre-launch.rust.library.files", "fre-rust.library.mods-info",
    ):
        assert key in app_uk and key in app_en, f"FRE Rust localization missing: {key}"
    # Runtime is either entirely absent (Play remains disabled) or a
    # complete private local installation. Never silently ship half a game.
    rust_bundle=json.loads((root/"apps/app/tauri.conf.json").read_text(encoding="utf-8"))["bundle"]
    rust_mapping=rust_bundle.get("resources", {})
    runtime_source=Path(__file__).resolve().parent/"fre-rust-runtime"
    if runtime_source.is_dir():
        assert rust_mapping["fre-rust-runtime/BUILD-INFO.json"] == "fre-rust/BUILD-INFO.json"
        assert rust_mapping["fre-rust-runtime/fre-rust-client.exe"] == "fre-rust/fre-rust-client.exe"
        assert rust_mapping["fre-rust-runtime/fre-rust-prepare.exe"] == "fre-rust/fre-rust-prepare.exe"
        assert rust_mapping["fre-rust-runtime/crates/horizon-prepare/"] == "fre-rust/crates/horizon-prepare/"
        assert (runtime_source/"crates/horizon-prepare/Cargo.toml").is_file()
        assert (runtime_source/"crates/horizon-prepare/build.rs").is_file()
        assert len(list((runtime_source/"crates/horizon-prepare/src").glob("*.rs"))) >= 10
        assert rust_mapping.get("fre-rust-runtime/resources/") == (
            "fre-rust/resources/" if (runtime_source/"resources/horizon-resources.json").is_file() else None
        )
    else:
        assert "fre-rust-runtime/fre-rust-client.exe" not in rust_mapping

    # Legacy folders and adapter remain for backup only. They are NOT presented
    # as FRE Rust profiles, and no worlds are removed or silently migrated.
    assert (root/"apps/app/src/api/horizon.rs").exists()
    assert (root/"apps/app-frontend/src/components/ui/HorizonRustPanel.vue").exists()
    legacy_library = (root/"apps/app-frontend/src/components/ui/HorizonLibrary.vue").read_text(encoding="utf-8")
    # Same virtualized grid and card positions: Rust cards follow Java cards
    # inside the ungrouped library, not in a second section below the list.
    for marker in ('<RustInstanceGrid', ':start-index="instanceGroup.instances.length"',
                   '@count-change="rustCardCount = $event"',
                   'props.instanceGroup.instances.length + (isRustGridGroup.value ? rustCardCount.value : 0)',
                   'class="relative" :style="{ height:', 'v-if="isRustGridGroup"'):
        assert marker in unified_group, f"Rust card grid is not integrated: {marker}"
    rust_library_shell=(root/"apps/app-frontend/src/components/ui/library/index.vue").read_text(encoding="utf-8")
    rust_grouping=(root/"apps/app-frontend/src/components/ui/library/rust-library-group.ts").read_text(encoding="utf-8")
    for key in ("group:none", "None:None", "Loader:Rust", "Game version:26.3", "Instance type:Rust"):
        assert key in rust_grouping, f"Rust profile missing from Java library grouping: {key}"
    assert "rustLibraryGroupId(displayState.value.group)" in unified_group
    assert "rustLibraryGroupId(displayState.value.group)" in rust_library_shell
    assert "rustProfileNames" in rust_library_shell
    assert "fre-rust-library-refresh" in rust_library_shell
    # Native Rust uses the EXACT same visual card and page-level building
    # blocks as Java. No bespoke library card, modal or Java game dispatch.
    for marker in ('<InstanceCardView', ':instance="viewInstance(card)"',
                   'plugin:fre-rust|rust_profiles', 'plugin:fre-rust|rust_launch',
                   'data-library-instance-card', "'/rust-instance/'"):
        assert marker in unified_rust, f"Shared Java card integration missing: {marker}"
    assert '<Teleport' not in unified_rust and 'role="dialog"' not in unified_rust, (
        "Rust profile must open a Java-like full page, not a self-built modal"
    )
    for marker in ('<PageHeader', '<PageHeaderActions', '<PageHeaderMetadata',
                   '<NavTabs', '<RustInstanceFiles', '<ConfirmDeleteInstanceModal',
                   '<TabbedModal', 'query="tab"', 'fre-rust-tab', 'rust_rename_profile', 'rust_delete_profile',
                   'useRootBreadcrumb',
                   'plugin:fre-rust|rust_launch', 'plugin:fre-rust|rust_import_resources'):
        assert marker in rust_detail, f"Java-like Rust profile page incomplete: {marker}"
    for marker in ('<FilePageLayout', 'provideFileManager',
                   'plugin:fre-rust|rust_list_files', 'plugin:fre-rust|rust_open_folder',
                   'isReadOnly: () => true'):
        assert marker in rust_files, f"Rust shared file browser unavailable: {marker}"
    assert "path: '/rust-instance/:kind(native|horizon)/:id'" in rust_routes
    assert "component: () => import('@/pages/RustInstancePage.vue')" in rust_routes
    assert "plugin:profile|launch" not in rust_detail and "plugin:profile|launch" not in unified_rust
    rust_custom_setup=(root/"packages/ui/src/components/flows/creation-flow-modal/components/CustomSetupStage.vue").read_text(encoding="utf-8")
    rust_context=(root/"packages/ui/src/components/flows/creation-flow-modal/creation-flow-context.ts").read_text(encoding="utf-8")
    for marker in ("selectedLoader.value === 'rust'", "value: '26.3'", "fre-rust.create.requirement"):
        assert marker in rust_custom_setup, f"Rust version chooser must use actual native game: {marker}"
    assert "selectedLoader.value === 'rust'" in rust_context, "Native loader must hide Java loader-version selection"
    assert "ctx.onHorizon?.()" not in rust_stage, "Native Rust should be selected as a loader in the normal Create screen"
    assert "fre-rust.create.requirement" in app_uk and "fre-rust.create.requirement" in app_en
    assert '<HorizonLibrary' not in rust_index and '<FreRustLibrary' not in rust_index, (
        "Do not render separate Horizon or Rust libraries"
    )
    for key in ('open', 'start', 'close', 'play', 'setup', 'settings',
                'open-folder', 'files', 'search-files', 'refresh',
                'reading', 'no-files', 'folder', 'native-required',
                'game-required', 'horizon-required', 'mods-warning',
                'mods', 'resourcepacks', 'shaderpacks', 'saves', 'logs', 'root'):
        translation = 'fre-rust.grid.' + key
        assert translation in app_uk and translation in app_en, (
            f"Rust instance UI not translated into both UA and EN: {translation}"
        )
    for translation in ('fre-rust.page.unavailable', 'fre-rust.page.content-note',
                        'fre-rust.page.no-logs', 'fre-rust.page.retry', 'fre-rust.files.read-only'):
        assert translation in app_uk and translation in app_en
    assert '<WelcomeScreen v-if="isReady && !hasCreatedInstance && rustHomeReady && !hasRustProfiles"' in rust_index, (
        "Preserve original Home/Welcome when no Java or Rust profiles exist"
    )
    assert 'v-else-if="isReady && (hasCreatedInstance || hasRustProfiles)"' in rust_index, (
        "Native Rust-only users must still see the library"
    )
    assert 'plugin:fre-horizon|horizon_open_folder' in legacy_library
    assert 'v-if="profiles.length || executable || error"' in legacy_library, "Legacy Horizon component preserved for existing users"
    assert 'fre-launch.rust.library.empty' in legacy_library
    assert 'v-if="profiles.length > 0"' not in rust_library, "Native Rust library must expose Create before first profile"
    assert "horizon-launcher.exe" not in rust_backend
    assert "horizon-launcher.exe" not in rust_panel
    assert "horizon-launcher.exe" not in rust_library

    splash=(root/"apps/app-frontend/src/components/ui/SplashScreen.vue").read_text(encoding="utf-8")
    assert 'aria-label="FRE LAUNCH"' in splash, "FRE LAUNCH startup branding is missing"
    assert '<svg' not in splash and 'modrinth-app-logo' not in splash, "Upstream Modrinth splash logo remains"
    assert "url('@/assets/loading/cube.png')" not in splash and "url('@/assets/loading/cube-light.webp')" not in splash, "Upstream splash background remains"
    discord_src=(root/"packages/app-lib/src/state/discord.rs").read_text(encoding="utf-8")
    assert 'FRE_LAUNCH_DISCORD_APP_ID' in discord_src, "Standalone Discord application configuration is missing"
    assert 'FRE_DISCORD_APPLICATION_ID.is_none()' in discord_src, "Modrinth Discord RPC must be disabled by default"
    for forbidden in ('1123683254248148992', 'modrinth_simple', 'Modrinth Logo'):
        assert forbidden not in discord_src, f"Upstream Discord identity remains: {forbidden}"
    assert 'description = "FRE LAUNCH is a desktop Minecraft launcher and mod manager"' in (root/"apps/app/Cargo.toml").read_text(encoding="utf-8")

    sidebar_vue=(root/"apps/app-frontend/src/app/shell/sidebar/index.vue").read_text(encoding="utf-8")
    for tag in ("<PrideFundraiserBanner", "<NewsSidebar", '<Promotion v-if="showAd"'):
        assert tag not in sidebar_vue, f"Sidebar still renders a removed promotional element: {tag}"

    assert 'to="/hosting/manage"' not in app
    assert ':show-ad="false"' in app
    assert 'watch(\n\t[stateInitialized, showAd, adConsentAvailable],' not in app
    assert (root/"packages/app-lib/.env").is_file(), "Missing production API environment file"
    assert (root/"FRE_LAUNCH_FORK.md").is_file()
    scripts = json.loads((root/"apps/app-frontend/package.json").read_text(encoding="utf-8"))["scripts"]
    assert "-p tsconfig.app.json" in scripts["tsc:check"], "Type checking must inspect source, not an empty references-only config"
    assert "write_instance_file(" not in cf_backend, "Files API rejects writes to mods/ at the pinned revision"
    from feature_updates import read_channel
    channel = read_channel()
    update_conf = json.loads((root/'apps/app/tauri.conf.json').read_text(encoding='utf-8'))
    assert update_conf['plugins']['updater']['pubkey'] == channel['publicKey']
    assert update_conf['plugins']['updater']['endpoints'] == [
        f'https://github.com/{channel["repository"]}/releases/latest/download/latest.json']
    assert update_conf['bundle']['createUpdaterArtifacts'] is True
    assert 'updater' in update_conf['build']['features']
    assert 'updater' in update_conf['app']['security']['capabilities']
    updater_cap = json.loads((root/'apps/app/capabilities/updater.json').read_text(encoding='utf-8'))
    assert updater_cap['permissions'] == ['updater:allow-check']
    assert 'launcher-files.modrinth.com' not in (root/'apps/app/tauri-release.conf.json').read_text(encoding="utf-8")
    assert 'launcher-files.modrinth.com' not in app
    assert 'watch(stateInitialized' in app and 'freAppUpdater.start()' in app
    assert 'false // Do not overwrite' not in cf_main
    assert json.loads((root/'apps/app-frontend/package.json').read_text(encoding="utf-8"))['version'] == channel['version']
    assert f'version = "{channel["version"]}"' in (root/'apps/app/Cargo.toml').read_text(encoding="utf-8")
    updater = (root/'apps/app/src/updater_impl.rs').read_text(encoding="utf-8")
    assert updater.index('verify_signature(') < updater.index('replace((update, bytes))')
    assert 'MAX_UPDATE_BYTES' in updater
    assert (root / 'apps/app/src/api/fre_rust.rs').is_file()
    assert 'rust_launch' in (root / 'apps/app/build.rs').read_text(encoding="utf-8")
    assert 'horizon_install_launcher' in (root / 'apps/app/build.rs').read_text(encoding="utf-8"), 'Horizon compatibility must survive an update'
    horizon_api = (root / 'apps/app/src/api/horizon.rs').read_text(encoding="utf-8")
    assert 'horizon_open_folder' in horizon_api and 'horizon_list_files' in horizon_api
    assert '0x00000008 | 0x00000200' in horizon_api, 'Horizon client must detach from terminal and Ctrl+C'
    assert 'rust_open_folder' in (root / 'apps/app/src/api/fre_rust.rs').read_text(encoding="utf-8")
    assert 'rust_list_files' in (root / 'apps/app/src/api/fre_rust.rs').read_text(encoding="utf-8")
    # Independent FRE Rust first-run: owned assets fetched from Mojang, not
    # another launcher. Retain compatibility backend on disk only.
    native_backend = (root / 'apps/app/src/api/fre_rust.rs').read_text(encoding="utf-8")
    standalone_setup = (root / 'apps/app/src/api/fre_rust_setup.rs').read_text(encoding="utf-8")
    for marker in ('standalone_setup::prepare(app.clone()', 'join("prepared")',
                   'if resources_valid(&destination)', 'fn resources_valid(root: &Path)',
                   'online_profile_fresh().await', 'rust_import_mod', 'join("horizon-mods")'):
        assert marker in native_backend, f"Independent Rust first-run incomplete: {marker}"
    for marker in ('fre-rust-prepare.exe', 'piston-meta.mojang.com', 'resources.download.minecraft.net',
                   'research-lock.json', '--minecraft-dir', '--complete', 'verified_preparer'):
        assert marker in standalone_setup, f"Bundled native resource preparation incomplete: {marker}"
    native_mod_ui = (root / 'apps/app-frontend/src/components/ui/RustNativeMods.vue').read_text(encoding="utf-8")
    native_page = (root / 'apps/app-frontend/src/pages/RustInstancePage.vue').read_text(encoding="utf-8")
    for marker in ('plugin:fre-rust|rust_mods', 'plugin:fre-rust|rust_import_mod',
                   'async function importFile', 'async function removeMod'):
        assert marker in native_mod_ui, f"Local Rust mod importer incomplete: {marker}"
    assert '<RustNativeMods v-if="kind === \'native\'"' in native_page
    assert 'plugin:fre-horizon|' not in native_page, 'Native Rust page must not call Horizon Launcher'
    assert 'rust_import_mod' in (root / 'apps/app/build.rs').read_text(encoding="utf-8"), "Missing Tauri mod import permission"
    for key in ('fre-rust.mods.title', 'fre-rust.mods.import-file', 'fre-rust.mods.remove',
                'fre-rust.page.first-run-hint', 'fre-rust.page.prep-prepare'):
        assert key in app_en and key in app_uk, f"Rust setup not translated: {key}"
    from feature_security_dependencies import workspace_overrides
    security = json.loads((Path(__file__).parent / 'security-dependencies.json').read_text(encoding="utf-8"))
    lock = (root / 'pnpm-lock.yaml').read_text(encoding='utf-8')
    assert hashlib.sha256(lock.encode()).hexdigest() == security['patchedLockSha256']
    workspace = (root / 'pnpm-workspace.yaml').read_text(encoding='utf-8')
    assert 'minimumReleaseAge: 10080' in workspace
    assert workspace.endswith(workspace_overrides(security)), 'Security overrides differ from the audited lock'
    print(f"Source check passed: {len(app_en)} App messages and {len(ui_en)} UI messages covered in UK + EN.")
    print("Nickname path, unique identifier, advertising and hosting checks passed.")
    print("Note: this does not replace real Rust/Vue/Windows compilation or Minecraft runtime testing.")

if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("source",type=Path)
    args=parser.parse_args()
    verify(args.source)
