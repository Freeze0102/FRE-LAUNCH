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
    for item in ("prepare.py","feature_local_accounts.py","feature_ukrainian.py","verify.py"):
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
        "apps/app/src/api/auth.rs":["create_offline_user","Sign in to Minecraft with Microsoft"],
        "apps/app/build.rs":["create_offline_user"],
        "packages/app-lib/src/api/minecraft_auth.rs":["pub async fn create_offline_user","OfflinePlayer:","md5::compute"],
        "packages/app-lib/src/state/minecraft_auth.rs":["pub fn is_offline_profile","fre-offline:"],
        "packages/app-lib/src/launcher/args.rs":['access_token.starts_with("fre-offline:")'],
        "apps/app-frontend/src/features/minecraft-accounts/accounts-card.vue":["OfflineProfileCreator","onOfflineCreated"],
        "apps/app-frontend/src/features/minecraft-accounts/offline-profile-creator.vue":["v-model.trim","create_offline_user","@submit.prevent"],
        "apps/app-frontend/src/helpers/auth.js":["plugin:auth|create_offline_user"],
        "apps/app-frontend/src/app/shell/title-bar/index.vue":["FRE LAUNCH"],
    }
    for relative,snippets in checks.items():
        data=(root/relative).read_text(encoding="utf-8")
        for snippet in snippets:
            if snippet not in data: raise AssertionError(f"{relative}: missing {snippet}")
    config=json.loads((root/"apps/app/tauri.conf.json").read_text(encoding="utf-8"))
    assert config["identifier"]=="app.frelaunch.modrinthfork"
    assert "ads" not in config["app"]["security"]["capabilities"]
    assert config["productName"]=="FRE LAUNCH"
    app=(root/"apps/app-frontend/src/App.vue").read_text(encoding="utf-8")
    assert 'to="/hosting/manage"' not in app
    assert ':show-ad="false"' in app
    assert 'watch(\n\t[stateInitialized, showAd, adConsentAvailable],' not in app
    assert (root/"FRE_LAUNCH_FORK.md").is_file()
    print(f"Source check passed: {len(app_en)} App messages and {len(ui_en)} UI messages covered in UK + EN.")
    print("Nickname path, unique identifier, advertising and hosting checks passed.")
    print("Note: this does not replace real Rust/Vue/Windows compilation or Minecraft runtime testing.")

if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("source",type=Path)
    args=parser.parse_args()
    verify(args.source)
