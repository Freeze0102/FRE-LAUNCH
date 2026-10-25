#!/usr/bin/env python3
"""Customize pinned GPLv3 Modrinth desktop sources into FRE LAUNCH."""
import argparse
import json
import shutil
import struct
import zlib
from pathlib import Path


def edit(path, old, new):
    contents=path.read_text(encoding="utf-8")
    if contents.count(old)!=1:
        raise RuntimeError(f"{path}: expected 1 occurrence of {old[:70]!r}, got {contents.count(old)}")
    path.write_text(contents.replace(old,new,1),encoding="utf-8")


def png(n):
    # Original unbranded geometric F glyph, RGBA PNG encoded using stdlib.
    lines=[]
    for y in range(n):
        line=bytearray([0])
        for x in range(n):
            u=x/n
            v=y/n
            f=(.25<u<.38 and .25<v<.8) or (.25<u<.76 and .25<v<.37) or (.25<u<.65 and .47<v<.58)
            line.extend((40,218,112,255) if f else (23,34,29,255))
        lines.append(bytes(line))
    def chunk(name,data):
        return struct.pack(">I",len(data))+name+data+struct.pack(">I",zlib.crc32(name+data))
    return b"\x89PNG\r\n\x1a\n"+chunk(b"IHDR",struct.pack(">2I5B",n,n,8,6,0,0,0))+chunk(b"IDAT",zlib.compress(b"".join(lines),9))+chunk(b"IEND",b"")


def ico(data):
    return struct.pack("<HHH",0,1,1)+struct.pack("<BBBBHHII",0,0,0,0,1,32,len(data),22)+data


def replace_art(root):
    from PIL import Image, ImageDraw
    svg='<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect width="128" height="128" rx="24" fill="#17221d"/><path d="M32 30h69v15H48v20h37v15H48v30H32z" fill="#28da70"/></svg>'
    def replace(path):
        if path.suffix==".svg":
            path.write_text(svg,encoding="utf-8")
        elif path.suffix==".png":
            path.write_bytes(png(128))
        elif path.suffix==".webp":
            im=Image.new("RGB",(128,128),(23,34,29))
            draw=ImageDraw.Draw(im)
            draw.rectangle((32,30,47,110),fill=(40,218,112))
            draw.rectangle((32,30,100,45),fill=(40,218,112))
            draw.rectangle((32,64,83,79),fill=(40,218,112))
            im.save(path,"WEBP",lossless=True)
        else:
            raise RuntimeError(f"Unrecognized protected brand asset: {path}")
    brand=root/"packages/assets/branding"
    if not brand.is_dir():
        raise RuntimeError("Modrinth brand asset layout changed")
    for path in brand.rglob("*"):
        if path.is_file():
            replace(path)
    for relative in (
        "apps/app-frontend/src/assets/modrinth_app.svg",
        "apps/app-frontend/src/assets/sad-modrinth-bot.webp",
        "apps/app-frontend/src/assets/welcome/modrinth-social-icon.png",
        "apps/app-frontend/src/assets/instance-icons/wrench-rinth.png",
        "apps/app-frontend/src/assets/instance-icons/mr-pack.png",
    ):
        file=root/relative
        if file.exists():
            replace(file)
    icons=root/"apps/app/icons"
    for file in icons.rglob("*"):
        if file.is_file():
            file.unlink()
    (icons/"128x128.png").write_bytes(png(128))
    (icons/"128x128@2x.png").write_bytes(png(256))
    (icons/"icon.ico").write_bytes(ico(png(256)))
    (icons/"icon.png").write_bytes(png(128))
    dmg=root/"apps/app/dmg"
    if dmg.exists():
        shutil.rmtree(dmg)
    for relative in (".github",".idea"):
        folder=root/relative
        if folder.is_dir():
            shutil.rmtree(folder)
        elif folder.exists():
            folder.unlink()


def customize(root,revision):
    app=root/"apps/app-frontend/src/App.vue"
    if not app.exists():
        raise RuntimeError("Expected Modrinth Vue app not found")
    body=app.read_text(encoding="utf-8")
    begin='\t\t\t<NavButton\n\t\t\t\tv-tooltip.right="formatMessage(messages.modrinthHosting)"'
    start=body.find(begin)
    end=body.find("\t\t\t</NavButton>",start)
    if start<0 or end<0:
        raise RuntimeError("Modrinth hosting button layout changed")
    body=body[:start]+body[end+len("\t\t\t</NavButton>"):]
    # Disable ad API calls as well as removing ad UI. The Tauri ads
    # capability is disabled in this fork so these helpers must be no-ops.
    ad_import=("import {\n"
               "\thide_ads_window,\n"
               "\tinit_ads_window,\n"
               "\tperform_ads_consent_action,\n"
               "\trelease_ads_window_hold,\n"
               "\tshould_show_ads_consent_popup,\n"
               "\ttake_ads_window_hold,\n"
               "} from '@/helpers/ads.js'")
    if body.count(ad_import)!=1:
        raise RuntimeError("Ad helper imports changed")
    body=body.replace(ad_import,
        "const take_ads_window_hold = async () => {}\n"
        "const release_ads_window_hold = async () => {}\n"
        "const should_show_ads_consent_popup = async () => false\n"
        "const perform_ads_consent_action = async () => {}")
    original='const showAd = computed(\n\t() => sidebarVisible.value && !hasPlus.value && credentials.value !== undefined,\n)\nconst adConsentAvailable = computed(() => credentials.value !== undefined && !hasPlus.value)'
    if body.count(original)!=1:
        raise RuntimeError("Ad visibility code changed")
    body=body.replace(original,"const showAd = computed(() => false)\nconst adConsentAvailable = computed(() => false)")
    event="useAppEvent('ads_consent_required', handleAdsConsentRequired, appEvents)"
    if body.count(event)!=1:
        raise RuntimeError("Ad consent listener changed")
    body=body.replace(event,"// FRE LAUNCH: advertising consent listener disabled")
    begin=body.find("watch(\n\t[stateInitialized, showAd, adConsentAvailable],")
    end=body.find("\n)\n",begin)
    if begin<0 or end<0:
        raise RuntimeError("Ad window initializer changed")
    body=body[:begin]+"// FRE LAUNCH: advertising window initialization removed\n"+body[end+3:]
    if body.count(':show-ad="showAd"')!=1:
        raise RuntimeError("Sidebar ad option changed")
    body=body.replace(':show-ad="showAd"',':show-ad="false"')
    app.write_text(body,encoding="utf-8")

    sidebar=root/"apps/app-frontend/src/app/shell/sidebar/index.vue"
    text=sidebar.read_text(encoding="utf-8")
    for snippet in (
        '\t\t<Promotion v-if="showAd" @upgrade-height="upgradeHeight = $event" />',
        '\t\t\t\t\t<PrideFundraiserBanner\n\t\t\t\t\t\tclass="p-4 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid"\n\t\t\t\t\t/>',
        '\t\t\t\t\t<NewsSidebar :style="promotionPaddingStyle" />',
    ):
        if text.count(snippet)!=1:
            raise RuntimeError("Promotional sidebar markup changed")
        text=text.replace(snippet,"")
    sidebar.write_text(text,encoding="utf-8")
    routes=root/"apps/app-frontend/src/routes.js"
    text=routes.read_text(encoding="utf-8")
    begin=text.find("\t\t{\n\t\t\tpath: '/hosting/manage/'")
    end=text.find("\t\t{\n\t\t\tpath: '/browse/:projectType'",begin)
    if begin<0 or end<0:
        raise RuntimeError("Hosting routes changed")
    routes.write_text(text[:begin]+text[end:],encoding="utf-8")
    title=root/"apps/app-frontend/src/app/shell/title-bar/index.vue"
    edit(title,"defineMessages, IconButton, TextLogo, useVIntl","defineMessages, IconButton, useVIntl")
    edit(title,'<TextLogo class="h-7 w-auto shrink-0 text-contrast pointer-events-none" />','<span class="shrink-0 select-none text-base font-bold text-contrast">FRE LAUNCH</span>')
    welcome=root/"apps/app-frontend/src/components/ui/WelcomeScreen.vue"
    edit(welcome,"id: 'app.welcome-screen.title',","id: 'fre-launch.welcome-screen.title',")
    edit(welcome,"defaultMessage: 'Welcome to Modrinth',","defaultMessage: 'Welcome to FRE LAUNCH',")
    edit(root/"apps/app-frontend/index.html","<title>Modrinth App</title>","<title>FRE LAUNCH</title>")
    edit(root/"apps/app-frontend/index.html",'\t\t<script src="https://tally.so/widgets/embed.js" async></script>\n',"")
    edit(root/"apps/app/src/main.rs",'fn are_updates_enabled() -> bool {\n    cfg!(feature = "updater")\n        && env::var("MODRINTH_EXTERNAL_UPDATE_PROVIDER").is_err()\n}',"fn are_updates_enabled() -> bool {\n    false // Do not overwrite this fork with official Modrinth releases\n}")
    conf=root/"apps/app/tauri.conf.json"
    settings=json.loads(conf.read_text(encoding="utf-8"))
    if settings["productName"]!="Modrinth App":
        raise RuntimeError("Unexpected app identity")
    settings["productName"]="FRE LAUNCH"
    # The desktop fork omits website/docs apps, so run frontend directly.
    # Avoid root-wide i18n / turbo tasks that require removed workspaces.
    settings["build"]["beforeDevCommand"]="pnpm --filter @modrinth/app-frontend dev"
    settings["build"]["beforeBuildCommand"]="pnpm --filter @modrinth/app-frontend build"
    settings["mainBinaryName"]="FRE LAUNCH"
    settings["identifier"]="app.frelaunch.modrinthfork"
    settings["bundle"]["targets"]=["nsis"]
    settings["bundle"]["icon"]=["icons/128x128.png","icons/128x128@2x.png","icons/icon.ico"]
    settings["bundle"].pop("macOS",None)
    settings["app"]["windows"][0]["title"]="FRE LAUNCH"
    settings["plugins"]["deep-link"]["desktop"]["schemes"]=["frelaunch"]
    caps=settings["app"]["security"].get("capabilities",[])
    settings["app"]["security"]["capabilities"]=[x for x in caps if x!="ads"]
    conf.write_text(json.dumps(settings,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    hooks=root/"apps/app/nsis/hooks.nsi"
    if hooks.exists():
        hooks.write_text(hooks.read_text(encoding="utf-8").replace("the Modrinth App","FRE LAUNCH"),encoding="utf-8")
    # The upstream Rust build and Vite expect packages/app-lib/.env.
    # Use Modrinth's PUBLIC production API endpoints, never personal secrets.
    app_lib=root/"packages/app-lib"
    if not (app_lib/".env").exists():
        production_env=app_lib/".env.prod"
        if not production_env.is_file():
            raise RuntimeError("Missing upstream app-lib/.env.prod configuration")
        (app_lib/".env").write_text(production_env.read_text(encoding="utf-8"),encoding="utf-8")
    # Keep the ENTIRE pinned Modrinth monorepo, including the real Vue/Tauri/Rust
    # engine and all workspace dependencies. This is a source fork, not an imitation.
    # The Windows launcher builds only the desktop app target.
    (root/"FRE_LAUNCH_FORK.md").write_text(
        "# FRE LAUNCH: GPLv3 Modrinth desktop fork\n\n"
        "This is a modified copy of Modrinth App desktop source by Rinth, Inc.\n"
        "Original: https://github.com/modrinth/code\n"
        "Pinned source revision: "+revision+"\n\n"
        "Changes: FRE LAUNCH identity and original neutral artwork; no hosting menu or routes,\n"
        "advertising window initialization, promotion banners or upstream auto-updater.\n"
        "The complete pinned upstream source tree and all original license notices are retained.\n\n"
        "License: GPL-3.0-only for the Modrinth App code, with original copyright and\n"
        "license notices retained. Redistributors must provide corresponding source.\n"
        "Reserved Modrinth brand images have been replaced, not redistributed.\n"
        "This is not an official Modrinth, Minecraft or Microsoft product.\n\n"
        "This fork uses a distinct application identifier and will not migrate or overwrite\n"
        "worlds/profiles from the separate FRE LAUNCH React app automatically.\n"
        "Minecraft account sign-in still needs verification for third-party clients.\n",encoding="utf-8"
    )
    replace_art(root)
    from feature_local_accounts import apply as add_local_accounts
    from feature_ukrainian import apply as finish_ukrainian
    from feature_skin_notice import apply as guard_offline_skins
    add_local_accounts(root)
    guard_offline_skins(root)
    finish_ukrainian(root)
    print("Prepared distinct FRE LAUNCH source fork from",revision)


if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("root",type=Path)
    parser.add_argument("--upstream-sha",required=True)
    args=parser.parse_args()
    customize(args.root.resolve(),args.upstream_sha)
