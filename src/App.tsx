import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { persist } from "zustand/middleware";
import {
  ArrowRight, AlertTriangle, Boxes, Check, ChevronRight, CircleHelp, Coffee, Copy, Download, FolderOpen, HardDriveDownload, House, Images, Layers3, ListFilter, Orbit, Play, Plus, Puzzle, RefreshCw, Search, Server, Settings2, ShieldCheck, Shirt, SlidersHorizontal, Terminal, UserRound, Wrench, X, Archive, ScrollText
} from "lucide-react";
import InstallDeck from "./InstallDeck";
import GameFolders from "./GameFolders";
import UpdateCenter from "./UpdateCenter";
import PlayPage from "./PlayPage";
import type {DiscoverHit} from "./PlayPage";
import { LAST_UI_CRASH } from "./ErrorBoundary";
import { InstancesPanel } from "./InstanceLibrary";
import ModsPanel from "./ModsPanel";
// Avoid evaluating Three.js/WebGL while the launcher is only showing the home screen.
const SkinStudio = lazy(() => import("./SkinStudio"));
import BackupsPanel from "./BackupsPanel";
import AccountsPanel from "./AccountsPanel";
import type {MicrosoftAccount} from "./AccountsPanel";
import JavaPanel from "./JavaPanel";
import ScreenshotsPanel from "./ScreenshotsPanel";
import SettingsDialog from "./SettingsDialog";
import type {FeatureSettings} from "./SettingsDialog";
import ServersPanel from "./ServersPanel";

type Version = { id: string; type: string; url: string; time: string; releaseTime: string };
type Progress = {stage: string; current: number; total: number; message: string};
type Diagnostics = {logPath:string; contents:string};
type LastInstall = {version:string;status:"installing"|"complete"|"error"|"interrupted";message:string;time:string};
function previousInstall():LastInstall|null {
  try {
    const raw=localStorage.getItem(LAST_INSTALL_KEY);
    if(!raw)return null;
    const data:unknown=JSON.parse(raw);
    if(typeof data!=="object"||!data)return null;
    const saved=data as Partial<LastInstall>;
    if(typeof saved.version!=="string"||!["installing","complete","error","interrupted"].includes(saved.status??""))return null;
    return {version:saved.version,status:saved.status as LastInstall["status"],message:typeof saved.message==="string"?saved.message:"",time:typeof saved.time==="string"?saved.time:""};
  }catch{return null;}
}
type InstallPreflight = {path:string; ready:boolean; detail:string};
type Task = "idle" | "installing" | "launching" | "running";
type Page = "Home" | "Versions" | "Instances" | "Mods" | "Skins" | "Servers" | "Accounts" |
  "Downloads" | "Logs" | "Backups" | "Screenshots" | "Settings";
type Store = {
  page: Page; setPage:(page:Page)=>void;
  selected:string; setSelected:(version:string)=>void;
  nickname:string; setNickname:(name:string)=>void;
  memoryMb:number; setMemoryMb:(mb:number)=>void;
  activeInstanceId:string; setActiveInstanceId:(id:string)=>void;
  theme:string; setTheme:(theme:string)=>void;
};
const useApp = create<Store>()(persist(set=>({
  page:"Home",setPage:(page)=>set({page}),
  selected:"",setSelected:(selected)=>set({selected}),
  nickname:"Player",setNickname:(nickname)=>set({nickname}),
  memoryMb:4096,setMemoryMb:(memoryMb)=>set({memoryMb}),
  activeInstanceId:"",setActiveInstanceId:(activeInstanceId)=>set({activeInstanceId}),
  theme:"midnight",setTheme:(theme)=>set({theme})
}),{
  name:"fre-launch-preferences",
  // Never restore the prior page after an unclean shutdown: skins/Three.js
  // or a heavy page may have been open when Windows terminated WebView2.
  merge:(stored,current)=>{
    const value=(stored&&typeof stored==="object"?stored:{}) as Partial<Store>;
    return {...current,...value,page:"Home",
      theme:themesafe(value.theme),
      nickname:typeof value.nickname==="string"&&/^[A-Za-z0-9_]{1,16}$/.test(value.nickname)?value.nickname:"Player",
      memoryMb:typeof value.memoryMb==="number"&&value.memoryMb>=512&&value.memoryMb<=32768?value.memoryMb:4096,
      selected:typeof value.selected==="string"&&value.selected.length<=64?value.selected:"",
      activeInstanceId:typeof value.activeInstanceId==="string"&&value.activeInstanceId.length<=64?value.activeInstanceId:""
    };
  }
}));
function themesafe(theme:unknown):string {
  return typeof theme==="string"&&["midnight","neon","ocean","emerald","light"].includes(theme)?theme:"midnight";
}

const menus = [
  {name:"Home",label:"Головна",icon:House,index:"01"},
  {name:"Mods",label:"Пошук модів",icon:Puzzle,index:"02"},
  {name:"Skins",label:"Скіни",icon:Shirt,index:"03"},
  {name:"Instances",label:"Бібліотека збірок",icon:Boxes,index:"04"},
  {name:"Servers",label:"Ігрові сервери",icon:Server,index:"05"},
  {name:"Versions",label:"Версії Minecraft",icon:Layers3,index:"06"},
  {name:"Screenshots",label:"Скріншоти",icon:Images,index:"07"},
  {name:"Backups",label:"Резервні копії",icon:Archive,index:"08"},
  {name:"Accounts",label:"Облікові записи",icon:UserRound,index:"09"},
  {name:"Downloads",label:"Завантаження",icon:Download,index:"10"},
  {name:"Logs",label:"Діагностика",icon:ScrollText,index:"11"},
  {name:"Settings",label:"Налаштування",icon:Settings2,index:"12"}
] as const;
const titles:Record<Page,string>={
  Home:"Головна",Versions:"Версії Minecraft",Instances:"Мої збірки",
  Mods:"Пошук модів",Skins:"Скіни",Servers:"Ігрові сервери",
  Screenshots:"Скріншоти",Backups:"Резервні копії",Accounts:"Облікові записи",
  Downloads:"Завантаження",Logs:"Діагностика",Settings:"Налаштування"
};
const taglines:Record<Page,string>={
  Home:"Minecraft Java Edition — швидкий запуск і керування збірками.",
  Versions:"Обери офіційний реліз або snapshot від Mojang.",
  Instances:"Твої збережені збірки Minecraft.",
  Mods:"Знайди та встанови вміст для обраної збірки.",
  Skins:"Переглядай і редагуй скіни персонажа.",
  Servers:"Збережені Minecraft-сервери для BedWars, SkyWars, Survival та інших режимів. Без хостингу.",
  Screenshots:"Знімки екрана з обраної збірки.",
  Backups:"Створення та відновлення резервних копій світів.",
  Accounts:"Minecraft-акаунти й локальні профілі.",
  Downloads:"Стан встановлення та завантаження файлів.",
  Logs:"Журнал роботи, помилки та діагностика Windows.",
  Settings:"Зовнішній вигляд, Java та оперативна пам'ять."
};

const LAST_INSTALL_KEY = "fre-launch-last-install";
function rememberInstall(version:string,status:"installing"|"complete"|"error"|"interrupted",message="") {
  try {localStorage.setItem(LAST_INSTALL_KEY,JSON.stringify({version,status,message,time:new Date().toISOString()}));}
  catch{/* Storage may be disabled; never interrupt downloads */}
}
const stages:Record<string,string>={
  metadata:"МЕТАДАНІ",client:"КЛІЄНТ",libraries:"БІБЛІОТЕКИ",
  assets:"РЕСУРСИ",natives:"РОЗПАКУВАННЯ",fabric:"FABRIC",quilt:"QUILT",complete:"ГОТОВО"
};
const themes=[
  {id:"midnight",label:"Темна",color:"#c9f079"},
  {id:"neon",label:"Контрастна",color:"#ed9275"},
  {id:"ocean",label:"Океан",color:"#79d9db"},
  {id:"emerald",label:"Зелена",color:"#98d885"},
  {id:"light",label:"Світла",color:"#f5f1e5"}
];

function truncate(text:string, max=55) {return text.length>max?text.slice(0,max-1)+"…":text;}
function humanError(error:unknown) {return typeof error==="string"?error:error instanceof Error?error.message:String(error);}
function progressPercent(progress:Progress|null){
  if(!progress)return 0;
  if(progress.stage==="complete")return 100;
  return progress.total>0?Math.min(99,Math.round(progress.current/progress.total*100)):0;
}

export default function App() {
  const {page,setPage,selected,setSelected,nickname,setNickname,memoryMb,setMemoryMb,activeInstanceId,setActiveInstanceId,theme,setTheme}=useApp();
  const [versions,setVersions]=useState<Version[]>([]);
  const [featureSettings,setFeatureSettings]=useState<FeatureSettings>(()=>{
    const defaults={showSkins:true,showScreenshots:true,showWorlds:true,showFiles:true};
    try{
      const item=JSON.parse(localStorage.getItem("fre-launch-feature-tabs")||"null");
      if(item&&typeof item==="object")return {
        showSkins:typeof item.showSkins==="boolean"?item.showSkins:true,
        showScreenshots:typeof item.showScreenshots==="boolean"?item.showScreenshots:true,
        showWorlds:typeof item.showWorlds==="boolean"?item.showWorlds:true,
        showFiles:typeof item.showFiles==="boolean"?item.showFiles:true
      };
    }catch{}
    return defaults;
  });
  const [sidebarInstances,setSidebarInstances]=useState<{id:string;name:string;version:string;memoryMb:number;favorite:boolean}[]>([]);
  const [libraryFocusId,setLibraryFocusId]=useState("");
  const [featuredProject,setFeaturedProject]=useState<DiscoverHit|null>(null);
  const [microsoftAccount,setMicrosoftAccount]=useState<MicrosoftAccount|null>(null);
  const [useMicrosoft,setUseMicrosoft]=useState(false);
  const [loading,setLoading]=useState(false);
  const [error,setError]=useState("");
  const [message,setMessage]=useState("");
  const [task,setTask]=useState<Task>("idle");
  const [installed,setInstalled]=useState(false);
  const [progress,setProgress]=useState<Progress|null>(null);
  const [search,setSearch]=useState("");
  const [filter,setFilter]=useState("release");
  const [java,setJava]=useState("Перевірка Java...");
  const [logs,setLogs]=useState<string[]>([]);
  const [diagnostics,setDiagnostics]=useState<Diagnostics|null>(null);
  const [windowsCrash,setWindowsCrash]=useState("");
  const [diagnosticsBusy,setDiagnosticsBusy]=useState(false);
  const [diagnosticsError,setDiagnosticsError]=useState("");
  const [lastAttempt,setLastAttempt]=useState<LastInstall|null>(previousInstall);
  const [preflight,setPreflight]=useState<InstallPreflight|null>(null);
  const [lastUiCrash,setLastUiCrash]=useState(()=>{try{return localStorage.getItem(LAST_UI_CRASH)||"";}catch{return "";}});
  const busyRef=useRef(false);
  const activeVersionRef=useRef("");
  const [showMore,setShowMore]=useState(false);
  const [recoveryMode,setRecoveryMode]=useState(false);
  const isBusy=task==="installing"||task==="launching"||task==="running";
  const activeLabel=menus.find(item=>item.name===page)?.index??"01";

  useEffect(()=>{document.documentElement.dataset.theme=theme;},[theme]);
  useEffect(()=>{
    let live=true;
    void invoke<{id:string;name:string;version:string;memoryMb:number;favorite:boolean}[]>("list_instances")
      .then(items=>{if(live)setSidebarInstances(items);})
      .catch(()=>{});
    return()=>{live=false;};
  },[page]);

  useEffect(()=>{
    void invoke("mark_frontend_mounted").catch(()=>{});
    void invoke<boolean>("software_rendering").then(setRecoveryMode).catch(()=>{});
    // Clear the crash-recovery marker only when the launcher has remained
    // responsive, not immediately after the initial React frame.
    const timer=window.setTimeout(()=>{void invoke("mark_frontend_ready").catch(()=>{});},20_000);
    return ()=>window.clearTimeout(timer);
  },[]);
  function markInstall(version:string,status:LastInstall["status"],message="") {
    rememberInstall(version,status,message);
    setLastAttempt({version,status,message,time:new Date().toISOString()});
  }
  async function loadDiagnostics() {
    setDiagnosticsBusy(true);setDiagnosticsError("");
    // Native installer logs and the Windows crash report are independent;
    // display either even when the other is not accessible.
    const [native,wer]=await Promise.allSettled([
      invoke<Diagnostics>("read_diagnostics"),
      invoke<string>("read_windows_crash_report")
    ]);
    if(native.status==="fulfilled")setDiagnostics(native.value);
    else {setDiagnosticsError(humanError(native.reason));setDiagnostics({logPath:"",contents:""});}
    setWindowsCrash(wer.status==="fulfilled"?wer.value:"");
    setDiagnosticsBusy(false);
  }
  useEffect(()=>{if(page==="Logs")void loadDiagnostics();},[page]);
  async function refreshVersions(preserveError=false) {
    setLoading(true);if(!preserveError)setError("");
    try{
      const data=await invoke<Version[]>("get_versions");
      setVersions(data);
      if(!useApp.getState().selected && data.length) {
        setSelected(data.find(v=>v.type==="release")?.id??data[0].id);
      }
    }catch(e){setError("Не вдалося отримати список Mojang: "+humanError(e));}
    finally{setLoading(false);}
  }
  useEffect(()=>{
    try {
      const text=localStorage.getItem(LAST_INSTALL_KEY);
      if(text){
        const previous=JSON.parse(text) as {version?:string;status?:string};
        if(previous.status==="installing") {
          const version=previous.version||"невідомої версії";
          markInstall(version,"interrupted","Попереднє встановлення перервалося. Причина невідома, відкрий діагностику.");
          void invoke("record_interrupted_install",{version}).catch(()=>{});
        }
      }
    }catch{/* Invalid cached state must not stop application startup */}
    void invoke<InstallPreflight>("install_preflight").then(setPreflight).catch(e=>setPreflight({path:"",ready:false,detail:humanError(e)}));
    void refreshVersions(true);
    void invoke<string>("java_info").then(setJava).catch(e=>setJava("Java в PATH не знайдена: "+humanError(e)));
    const a=listen<Progress>("install-progress",event=>setProgress(event.payload));
    const b=listen<string>("game-log",event=>setLogs(prev=>[...prev.slice(-499),event.payload]));
    const c=listen<string>("game-status",event=>{
      const status=event.payload;
      setMessage(status==="running"?"Minecraft запущено. Приємної гри!":status);
      setTask(status==="running"?"running":"idle");
      if(status!=="running")busyRef.current=false;
    });
    return ()=>{void a.then(stop=>stop());void b.then(stop=>stop());void c.then(stop=>stop());};
  },[]);
  useEffect(()=>{
    let current=true;
    setInstalled(false);
    if(selected)void invoke<boolean>("is_installed",{version:selected})
      .then(yes=>{if(current)setInstalled(yes);})
      .catch(e=>{if(current)setError(humanError(e));});
    return ()=>{current=false;};
  },[selected]);

  const filtered=useMemo(()=>versions.filter(v=>
    (filter==="all"||v.type===filter)&&v.id.toLowerCase().includes(search.toLowerCase())
  ).slice(0,120),[versions,search,filter]);

  async function installVersion(version:string) {
    if(!version||busyRef.current)return;
    busyRef.current=true;
    activeVersionRef.current=version;
    markInstall(version,"installing");
    setSelected(version);
    setTask("installing");
    setError("");setMessage("");setProgress({stage:"metadata",current:0,total:1,message:"Починаємо встановлення Minecraft..."});
    try{
      const check=await invoke<InstallPreflight>("install_preflight");
      setPreflight(check);
      if(!check.ready)throw new Error("Немає доступу до папки Minecraft: "+check.detail+" ("+check.path+")");
      const result=await invoke<string>("install_version",{version});
      markInstall(version,"complete",result);
      setMessage(result);
      if(useApp.getState().selected===version)setInstalled(true);
    }catch(e){
      markInstall(version,"error",humanError(e));
      setError(humanError(e));
      setPage("Downloads");
      void loadDiagnostics();
    }finally{
      busyRef.current=false;
      setTask("idle");
    }
  }
  function saveFeatureSettings(next:FeatureSettings){
    setFeatureSettings(next);
    try{localStorage.setItem("fre-launch-feature-tabs",JSON.stringify(next));}catch{}
  }
  async function launch(target?:{id:string;version:string;memoryMb:number}) {
    const playVersion=target?.version??selected;
    const playInstance=target?.id??activeInstanceId;
    const playMemory=target?.memoryMb??memoryMb;
    if(!playVersion||busyRef.current||task==="running")return;
    if(target){
      setSelected(playVersion);setActiveInstanceId(playInstance);setMemoryMb(playMemory);
    }
    try{
      const ready=target?await invoke<boolean>("is_installed",{version:playVersion}):installed;
      if(!ready){await installVersion(playVersion);return;}
    }catch(e){setError("Не вдалося перевірити Minecraft: "+humanError(e));return;}
    busyRef.current=true;
    setTask("launching");setError("");setMessage("");setLogs([]);
    try{
      const response=await invoke<string>("launch_game",{version:playVersion,nickname,memoryMb:playMemory,instanceId:playInstance||null,useMicrosoft:useMicrosoft&&!!microsoftAccount});
      setMessage(response);
      setTask("running");
    }catch(e){
      setError(humanError(e));
      setTask("idle");
      busyRef.current=false;
      setPage("Logs");
      void loadDiagnostics();
    }
  }
  async function copyText(text:string){
    try{await navigator.clipboard.writeText(text);setMessage("Скопійовано в буфер обміну.");}
    catch(e){setError("Не вдалося скопіювати: "+humanError(e));}
  }
  const percent=progressPercent(progress);
  const installAction=task==="installing"?"ЗАВАНТАЖЕННЯ...":task==="launching"?"ЗАПУСК...":task==="running"?"ГРА ПРАЦЮЄ":installed?"ГРАТИ ЗАРАЗ":"ВСТАНОВИТИ";
  const selectedKind=versions.find(v=>v.id===selected)?.type??"release";

  return <div className="fre-app">
    <aside className="fre-rail">
      <button className="fre-identity" onClick={()=>setPage("Home")} title="На головну">
        <span className="fre-identity-mark">F</span>
        <span className="fre-identity-word"><b>FRE <em>LAUNCH</em></b><small>Minecraft Java Edition</small></span>
      </button>
      <div className="fre-rail-divider"><span>НАВІГАЦІЯ</span></div>
      <nav className="fre-nav" aria-label="Розділи лаунчера">
        {menus.slice(0,4).filter(item=>item.name!=="Skins"||featureSettings.showSkins).map(({name,label,icon:Icon,index})=><button key={name}
          className={"fre-nav-link"+(page===name?" is-active":"")}
          onClick={()=>{if(name==="Instances")setLibraryFocusId("");if(name==="Mods")setFeaturedProject(null);setPage(name);}}
          aria-current={page===name?"page":undefined}
          title={label}>
          <span className="fre-nav-index">{index}</span><Icon size={19} strokeWidth={1.9}/>
          <span className="fre-nav-label">{label}</span>
        </button>)}
        <div className="fre-sidebar-divider" aria-hidden="true"/>
        {sidebarInstances.slice(0,5).map(item=><button
          key={item.id} className={"fre-nav-link fre-sidebar-instance"+(activeInstanceId===item.id&&page==="Instances"?" is-active":"")}
          onClick={()=>{setActiveInstanceId(item.id);setSelected(item.version);setMemoryMb(item.memoryMb);setLibraryFocusId(item.id);setPage("Instances");}}
          title={item.name+" · "+item.version} aria-label={"Відкрити збірку "+item.name}>
          <Boxes size={20}/>
        </button>)}
        <button className="fre-nav-link fre-sidebar-add" title="Створити збірку" aria-label="Створити збірку"
          onClick={()=>{setLibraryFocusId("");setPage("Instances");}}><Plus size={21}/></button>
        <div className="fre-sidebar-divider" aria-hidden="true"/>
        {menus.slice(4).filter(item=>item.name!=="Screenshots"||featureSettings.showScreenshots).map(({name,label,icon:Icon,index})=><button key={name}
          className={"fre-nav-link fre-sidebar-other"+(page===name?" is-active":"")}
          onClick={()=>setPage(name)}
          aria-current={page===name?"page":undefined}
          title={label}>
          <span className="fre-nav-index">{index}</span><Icon size={19} strokeWidth={1.9}/>
          <span className="fre-nav-label">{label}</span>
        </button>)}
      </nav>
      <UpdateCenter/>
      <div className="fre-rail-foot">
        <span className="fre-status-live"><i/> Minecraft Java Edition</span>
        <div className="fre-build-status"><strong>0.5.7 <span>ALPHA</span></strong><small>ТЕСТОВА ВЕРСІЯ</small></div>
      </div>
    </aside>
    <div className="fre-workspace">
      <header className="fre-commandbar">
        <div className="fre-command-left"><span className="fre-command-square"><Orbit size={17}/></span>
          <span>FRE LAUNCH</span><span className="fre-command-slash">/</span><b>{titles[page]}</b>
        </div>
        <div className="fre-command-right">
          {recoveryMode&&<span title="Відновлення після аварійного завершення: WebView2 без GPU-прискорення" className="fre-safe-mode-tag">RECOVERY</span>}
          <button className="fre-command-java" onClick={()=>setPage("Settings")} title="Стан Java"><Coffee size={16}/><span>{java.startsWith("Java в PATH")?"JAVA НЕ ЗНАЙДЕНА":"JAVA READY"}</span></button>
          <button className="fre-command-account" onClick={()=>setPage("Accounts")} title="Вибрати профіль">
            <span className="fre-avatar">{nickname[0]?.toUpperCase()||"P"}</span><span>{nickname}{useMicrosoft&&microsoftAccount?" · Microsoft":""}</span><ChevronRight size={15}/>
          </button>
        </div>
      </header>
      <main className="fre-main">
        {!["Home","Instances","Mods","Skins"].includes(page)&&<div className="fre-page-intro">
          <div className="fre-kicker"><span className="fre-small-line"/> MINECRAFT JAVA <span className="fre-kicker-dot">•</span> {activeLabel} / 12</div>
          <div className="fre-heading-row"><div><h1>{titles[page]}</h1><p>{taglines[page]}</p></div>
            {page!=="Home"&&<button className="fre-return" onClick={()=>setPage("Home")}><House size={16}/> Грати <ArrowRight size={15}/></button>}
          </div>
        </div>}
        {error&&<div className="fre-incident" role="alert"><AlertTriangle size={16}/><span title={error}>{error}</span><button onClick={()=>setPage("Logs")}><Terminal size={14}/> ЖУРНАЛ <ArrowRight size={13}/></button><button onClick={()=>setError("")} title="Приховати повідомлення" aria-label="Приховати повідомлення"><X size={16}/></button></div>}
        {message&&<div className="fre-flash" role="status"><Check size={17}/><span>{message}</span><button onClick={()=>setMessage("")} title="Закрити"><X size={16}/></button></div>}

        {(page==="Home"||page==="Settings")&&<PlayPage
          versions={versions}
          loading={loading}
          selected={selected}
          selectedKind={selectedKind}
          installed={installed}
          isBusy={isBusy}
          installAction={installAction}
          activeInstanceId={activeInstanceId}
          memoryMb={memoryMb}
          nickname={nickname}
          java={java}
          recoveryMode={recoveryMode}
          onPage={dest=>{if(dest==="Mods")setFeaturedProject(null);setPage(dest);}}
          onLaunch={()=>void launch()}
          onLaunchInstance={item=>void launch(item)}
          onOpenInstance={(id,version,memory)=>{setLibraryFocusId(id);setActiveInstanceId(id);setSelected(version);setMemoryMb(memory);setPage("Instances");}}
          microsoftConnected={!!microsoftAccount}
          onVersion={id=>{setSelected(id);setActiveInstanceId("");}}
          onSelectInstance={(id,version,memory)=>{setActiveInstanceId(id);setSelected(version);setMemoryMb(memory);}}
          onRefresh={()=>void refreshVersions()}
          onOpenProject={hit=>{setFeaturedProject(hit);setPage("Mods");}}
        />}

        {page==="Versions"&&<section className="fre-version-catalog">
          <div className="fre-catalog-toolbar">
            <div className="fre-catalog-search"><Search size={19}/><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="ПОШУК ВЕРСІЇ — 1.21.4..." aria-label="Пошук версії"/></div>
            <div className="fre-catalog-pills">{[{id:"release",label:"RELEASE"},{id:"snapshot",label:"SNAPSHOT"},{id:"all",label:"УСІ"}].map(t=><button key={t.id} className={filter===t.id?"is-current":""} onClick={()=>setFilter(t.id)}>{t.label}</button>)}</div>
            <button className="fre-catalog-refresh" onClick={()=>void refreshVersions()} disabled={loading} title="Оновити з Mojang"><RefreshCw size={18}/></button>
          </div>
          <div className="fre-catalog-counter"><ListFilter size={15}/> {loading?"ОТРИМАННЯ ДАНИХ MOJANG":`ЗНАЙДЕНО ${versions.length} · ПОКАЗАНО ${filtered.length}`} <span>ОФІЦІЙНИЙ КАТАЛОГ</span></div>
          <div className="fre-version-list">{filtered.map((version,index)=><button disabled={isBusy} key={version.id} className={"fre-version-row"+(selected===version.id?" is-chosen":"")} onClick={()=>{setSelected(version.id);setActiveInstanceId("");}}>
            <span className="fre-vrow-no">{String(index+1).padStart(2,"0")}</span><span className="fre-vrow-title"><b>{version.id}</b><small>{version.type.replace("_"," ").toUpperCase()}</small></span>
            <span className="fre-vrow-date">{new Date(version.releaseTime).toLocaleDateString("uk-UA")}</span><span className="fre-vrow-select">{selected===version.id?"ОБРАНО ✓":"ОБРАТИ ↗"}</span>
          </button>)}</div>
          <div className="fre-catalog-bottom"><span>АКТИВНА: <strong>{selected||"НЕ ОБРАНО"}</strong></span>
            <button className="fre-install-cta" disabled={!selected||isBusy} onClick={()=>void installVersion(selected)}><HardDriveDownload size={19}/> {task==="installing"?"ВСТАНОВЛЕННЯ...":"ВСТАНОВИТИ / ПЕРЕВІРИТИ"} <ArrowRight size={18}/></button>
          </div>
        </section>}

        {task==="installing" && progress && <div className="fre-install-activity" aria-live="polite"><div><span>ПРОЦЕС: {stages[progress.stage]??progress.stage.toUpperCase()}</span><strong>{progress.message}</strong></div><span className="fre-install-percent">{percent}%</span><div className="fre-install-track"><div style={{width:percent+"%"}}/></div><button onClick={()=>setPage("Downloads")}>ДЕТАЛІ <ArrowRight size={14}/></button></div>}

        {page==="Instances"&&<div className="fre-module"><InstancesPanel version={selected} active={activeInstanceId} setActive={setActiveInstanceId} setVersion={setSelected} setMemory={setMemoryMb} navigateHome={()=>setPage("Home")} navigateMods={()=>{setFeaturedProject(null);setPage("Mods");}} navigateVersions={()=>setPage("Versions")} onPlayInstance={item=>void launch(item)} showWorlds={featureSettings.showWorlds} showFiles={featureSettings.showFiles} requestedId={libraryFocusId}/></div>}
        {page==="Mods"&&<div className="fre-module"><ModsPanel version={selected} instanceId={activeInstanceId} initialProject={featuredProject?{...featuredProject,slug:featuredProject.slug||""}:null} onServers={()=>setPage("Servers")}/></div>}
        {page==="Skins"&&<div className="fre-module"><Suspense fallback={<div className="fre-log-mini-empty">Завантажуємо 3D-студію скінів…</div>}><SkinStudio/></Suspense></div>}
         {page==="Servers"&&<div className="fre-module"><ServersPanel/></div>}
              {page==="Screenshots"&&<div className="fre-module"><ScreenshotsPanel instanceId={activeInstanceId}/></div>}
        {page==="Backups"&&<div className="fre-module"><BackupsPanel instanceId={activeInstanceId} instanceName={activeInstanceId?"ВИБРАНА ЗБІРКА":""}/></div>}
        {page==="Accounts"&&<div className="fre-module"><AccountsPanel nickname={nickname}
          microsoft={microsoftAccount}
          onSelect={name=>{setNickname(name);setUseMicrosoft(false);}}
          onMicrosoftSelect={account=>{
            setMicrosoftAccount(account);setUseMicrosoft(!!account);
            if(account)setNickname(account.name);
            else setNickname("Player");
          }}/></div>}

        {page==="Settings"&&<SettingsDialog
          onClose={()=>setPage("Home")}
          onNavigate={dest=>setPage(dest)}
          theme={theme}
          onTheme={setTheme}
          features={featureSettings}
          onFeatures={saveFeatureSettings}
          memoryMb={memoryMb}
          onMemory={setMemoryMb}
          instanceId={activeInstanceId}
          recoveryMode={recoveryMode}
          onRecoveryDisabled={()=>void invoke("disable_software_recovery").then(()=>setMessage("Після перезапуску лаунчер спробує звичайну графіку.")).catch(e=>setError(humanError(e)))}
        />}

        {page==="Downloads"&&<>
          <InstallDeck
            version={selected}
            progress={progress}
            preflight={preflight}
            isInstalling={task==="installing"}
            onCheck={async ()=>{
              try {
                const result=await invoke<InstallPreflight>("install_preflight");
                setPreflight(result);
                return result;
              } catch(e){
                const problem={path:preflight?.path??"",ready:false,detail:humanError(e)};
                setPreflight(problem);
                return problem;
              }
            }}
            onLogs={()=>{void loadDiagnostics();setPage("Logs");}}
          />
          {selected&&<GameFolders instanceId={activeInstanceId} version={selected}/>}
        </>}


        {page==="Logs"&&<section className="fre-diag-sheet fre-diagnostics">
          <div className="fre-module-label"><span>FRE / DIAGNOSTICS · LOCAL ONLY</span><ShieldCheck size={19}/></div>
          <div className="fre-diag-header"><div><h2>СИСТЕМНИЙ ЖУРНАЛ.</h2><p>Остання спроба та повідомлення про збої зберігаються локально. Дані не надсилаються нікуди.</p></div><span className="fre-diag-index">D / 01</span></div>
          <div className="fre-diag-overview">
            <div className="fre-diag-metric">
              <span>ОСТАННЯ СПРОБА</span>
              <strong>{lastAttempt ? lastAttempt.status==="complete"?"ГОТОВО":lastAttempt.status==="interrupted"?"ПЕРЕРВАНО":lastAttempt.status==="error"?"ПОМИЛКА":"В ПРОЦЕСІ":"НЕМАЄ"}</strong>
              <small>{lastAttempt ? "Minecraft "+lastAttempt.version : "Встановлень ще не записано"}</small>
            </div>
            <div className="fre-diag-metric">
              <span>ЧАС ОСТАННЬОЇ СПРОБИ</span>
              <strong className="fre-diag-metric-small">{lastAttempt?.time&&!Number.isNaN(Date.parse(lastAttempt.time))?new Date(lastAttempt.time).toLocaleString("uk-UA"):"—"}</strong>
              <small>{lastAttempt?.message||"Дані про спробу з'являться після встановлення"}</small>
            </div>
            <div className="fre-diag-metric">
              <span>JAVA / RUNTIME</span>
              <strong className="fre-diag-metric-small">{java.startsWith("Java в PATH")?"НЕ ЗНАЙДЕНА":"ПЕРЕВІРЕНО"}</strong>
              <small title={java}>{java}</small>
            </div>
          </div>
          <div className="fre-log-toolbar">
            <button disabled={diagnosticsBusy} onClick={()=>void loadDiagnostics()}><RefreshCw size={16}/> {diagnosticsBusy?"ЧИТАЄМО...":"ОНОВИТИ ЖУРНАЛ"}</button>
            <button onClick={()=>void copyText([
              "FRE LAUNCH / 0.5.7 ALPHA",
              "Last attempt: "+(lastAttempt?JSON.stringify(lastAttempt):"none"),
              "Java: "+java,
              "Log path: "+(diagnostics?.logPath||"unknown"),
              "Install diagnostics:", diagnostics?.contents||"(no native records)",
              "Windows crash report:", windowsCrash||"(none available)",
              "UI error:", lastUiCrash||"(none)",
              "Game output:", logs.length?logs.join("\n"):"(none)"
            ].join("\n"))}><Copy size={16}/> КОПІЮВАТИ ЗВІТ</button>
            <button disabled={!logs.length} onClick={()=>void copyText(logs.join("\n"))}><Copy size={16}/> ЛОГ ГРИ</button>
          </div>
          {diagnosticsError&&<div className="fre-diag-warning"><AlertTriangle size={16}/> Не вдалося прочитати файл журналу: {diagnosticsError}</div>}
          {diagnostics?.logPath&&<div className="fre-diagnostic-location"><FolderOpen size={16}/><span>{diagnostics.logPath}</span></div>}
          <div className="fre-log-label">01 / STARTUP & INSTALL LOG</div>
          {diagnosticsBusy&&!diagnostics?<div className="fre-log-placeholder">Читаємо локальний журнал встановлення...</div>:
            diagnostics?.contents?.trim()?
            <pre className="fre-log-output">{diagnostics.contents}</pre>:
            <div className="fre-log-placeholder"><Terminal size={23}/><strong>У ФАЙЛІ ПОКИ НЕМАЄ ЗАПИСІВ</strong><p>Якщо лаунчер закрився до запуску Rust-команди, системний журнал міг не встигнути створитися. Час і статус спроби показано вище — їх зберігає інтерфейс.</p><button onClick={()=>void loadDiagnostics()}>ПЕРЕВІРИТИ ЩЕ РАЗ <RefreshCw size={14}/></button></div>}
          <div className="fre-log-label">02 / INTERFACE · LAST FAILURE</div>
          {lastUiCrash?<><pre className="fre-log-output">{lastUiCrash}</pre><div className="fre-log-toolbar"><button onClick={()=>void copyText(lastUiCrash)}><Copy size={16}/> КОПІЮВАТИ ПОМИЛКУ</button><button onClick={()=>{try{localStorage.removeItem(LAST_UI_CRASH)}catch{}setLastUiCrash("");}}><X size={16}/> ПРИБРАТИ ЗАПИС</button></div></>:
          <div className="fre-log-mini-empty">Помилок інтерфейсу поки не зафіксовано.</div>}
          <div className="fre-log-label">03 / WINDOWS · NATIVE PROCESS CRASH</div>
          {windowsCrash?<pre className="fre-log-output">{windowsCrash}</pre>:
          <div className="fre-log-mini-empty">Звіт Windows про аварійне завершення FRE LAUNCH не знайдено. Це не означає, що збою не було: збереження звітів Windows може бути вимкнене. Додатково перевір «Монітор стабільності Windows» (Win+R → perfmon /rel).</div>}
          <div className="fre-log-label">04 / GAME CONSOLE · LIVE OUTPUT</div>
          {logs.length?<pre className="fre-log-output">{logs.join("\n")}</pre>:
          <div className="fre-log-mini-empty">Гра ще не надсилала повідомлень у цьому сеансі.</div>}
        </section>}

        <footer className="fre-footer"><span>FRE LAUNCH · ALPHA 0.5.7</span><span>MINECRAFT JAVA EDITION</span><button onClick={()=>{setShowMore(v=>!v);}}>ПРО ПРОЄКТ <CircleHelp size={14}/></button></footer>
        {showMore&&<div className="fre-disclaimer"><button onClick={()=>setShowMore(false)}><X size={16}/></button>FRE LAUNCH — неофіційний Minecraft Java лаунчер. Не пов'язаний з Mojang чи Microsoft. Перевірка реального запуску на всіх Windows-конфігураціях ще триває.</div>}
      </main>
    </div>
  </div>;
}
