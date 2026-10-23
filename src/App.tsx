import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { motion } from "motion/react";
import { create } from "zustand";
import { persist } from "zustand/middleware";
import {
  ArrowDownToLine, ArrowRight, AlertTriangle, Boxes, Check, ChevronRight, CircleHelp,
  Coffee, Copy, Download, FolderOpen, HardDriveDownload, House, Images, Layers3,
  ListFilter, MonitorDot, Orbit, PauseCircle, Play, Puzzle, RefreshCw, Search,
  Server, Settings2, ShieldCheck, Shirt, SlidersHorizontal, Terminal, UserRound,
  Wrench, X, Zap, Archive, ScrollText
} from "lucide-react";
import WorldArt from "./WorldArt";
import InstallDeck, {type Preflight} from "./InstallDeck";
import { LAST_UI_CRASH } from "./ErrorBoundary";
import { InstancesPanel } from "./Features";
import ModsPanel from "./ModsPanel";
import SkinStudio from "./SkinStudio";
import BackupsPanel from "./BackupsPanel";
import AccountsPanel from "./AccountsPanel";
import JavaPanel from "./JavaPanel";
import ServersPanel from "./ServersPanel";
import ScreenshotsPanel from "./ScreenshotsPanel";

type Version = { id: string; type: string; url: string; time: string; releaseTime: string };
type Progress = {stage: string; current: number; total: number; message: string};
type Diagnostics = {logPath:string; contents:string};
type Task = "idle" | "installing" | "launching" | "running";
type Page = "Home" | "Versions" | "Instances" | "Mods" | "Skins" | "Accounts" |
  "Servers" | "Downloads" | "Logs" | "Backups" | "Screenshots" | "Settings";
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
}),{name:"fre-launch-preferences"}));

const menus = [
  {name:"Home",label:"Пункт запуску",icon:House,index:"01"},
  {name:"Versions",label:"Версії Minecraft",icon:Layers3,index:"02"},
  {name:"Instances",label:"Мої збірки",icon:Boxes,index:"03"},
  {name:"Mods",label:"Моди / ресурси",icon:Puzzle,index:"04"},
  {name:"Skins",label:"Скін-студія",icon:Shirt,index:"05"},
  {name:"Servers",label:"Мережа серверів",icon:Server,index:"06"},
  {name:"Screenshots",label:"Кадри зі світів",icon:Images,index:"07"},
  {name:"Backups",label:"Архів світів",icon:Archive,index:"08"},
  {name:"Accounts",label:"Профілі",icon:UserRound,index:"09"},
  {name:"Downloads",label:"Завантаження",icon:Download,index:"10"},
  {name:"Logs",label:"Діагностика",icon:ScrollText,index:"11"},
  {name:"Settings",label:"Параметри",icon:Settings2,index:"12"}
] as const;
const titles:Record<Page,string>={
  Home:"ПУНКТ ЗАПУСКУ",Versions:"КАТАЛОГ ВЕРСІЙ",Instances:"МОЇ ЗБІРКИ",
  Mods:"МАЙСТЕРНЯ МОДІВ",Skins:"СКІН-СТУДІЯ",Servers:"СЕРВЕРНА МЕРЕЖА",
  Screenshots:"СКРИНШОТИ",Backups:"АРХІВ СВІТІВ",Accounts:"МОЇ ПРОФІЛІ",
  Downloads:"ЗАВАНТАЖЕННЯ",Logs:"ЖУРНАЛ ПОДІЙ",Settings:"ПАРАМЕТРИ"
};
const taglines:Record<Page,string>={
  Home:"Твій наступний світ починається звідси.",
  Versions:"Від старої класики до найновіших пригод.",
  Instances:"Кожному світові — власне місце.",
  Mods:"Збирай гру під себе, не навпаки.",
  Skins:"Намалюй героя своєї наступної історії.",
  Servers:"Знайди, де зараз твої друзі.",
  Screenshots:"Збережені миті зі своїх пригод.",
  Backups:"Твій світ заслуговує на запасну копію.",
  Accounts:"Обери, ким ти заходиш у гру.",
  Downloads:"Жодного удаваного прогресу. Лише справжні файли.",
  Logs:"Тут видно, що насправді робить лаунчер.",
  Settings:"Налаштуй свій простір так, як хочеш."
};
const LAST_INSTALL_KEY = "fre-launch-last-install";
function rememberInstall(version:string,status:"installing"|"complete"|"error"|"interrupted",message="") {
  try {localStorage.setItem(LAST_INSTALL_KEY,JSON.stringify({version,status,message,time:new Date().toISOString()}));}
  catch{/* Storage may be disabled; never interrupt downloads */}
}
const stages:Record<string,string>={
  metadata:"МЕТАДАНІ",client:"КЛІЄНТ",libraries:"БІБЛІОТЕКИ",
  assets:"РЕСУРСИ",fabric:"FABRIC",quilt:"QUILT",complete:"ГОТОВО"
};
const themes=[
  {id:"midnight",label:"FIELD NOTES",color:"#c9f079"},
  {id:"neon",label:"AFTER DARK",color:"#ed9275"},
  {id:"ocean",label:"DEEP WATER",color:"#79d9db"},
  {id:"emerald",label:"WILDERNESS",color:"#98d885"},
  {id:"light",label:"PAPER TRAIL",color:"#f5f1e5"}
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
  const [preflight,setPreflight]=useState<Preflight|null>(null);
  const [lastUiCrash,setLastUiCrash]=useState(()=>{try{return localStorage.getItem(LAST_UI_CRASH)||"";}catch{return "";}});
  const busyRef=useRef(false);
  const activeVersionRef=useRef("");
  const [showMore,setShowMore]=useState(false);
  const isBusy=task==="installing"||task==="launching"||task==="running";
  const activeLabel=menus.find(item=>item.name===page)?.index??"01";

  useEffect(()=>{document.documentElement.dataset.theme=theme;},[theme]);
  async function loadDiagnostics() {
    try{setDiagnostics(await invoke<Diagnostics>("read_diagnostics"));}
    catch(e){setDiagnostics({logPath:"Недоступний файл журналу",contents:"Не вдалося прочитати журнал: "+humanError(e)});}
  }
  async function checkDisk() {
    try {
      const result=await invoke<Preflight>("install_preflight");
      setPreflight(result);
      return result;
    } catch(e) {
      const fail:Preflight={path:"Невідомий каталог",ready:false,detail:humanError(e)};
      setPreflight(fail);
      return fail;
    }
  }
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
          setError(`Попередній сеанс обірвався під час встановлення Minecraft ${version}. Відкрито діагностику: скопіюй журнал для перевірки.`);
          setPage("Logs");
          rememberInstall(version,"interrupted","Unexpected process exit during download");
          void loadDiagnostics();
        }
      }
    }catch{/* Invalid cached state must not stop application startup */}
    void checkDisk();
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
    setSelected(version);
    setTask("installing");
    setError("");setMessage("");setProgress({stage:"metadata",current:0,total:1,message:"Починаємо встановлення Minecraft..."});
    try{
      const check=await checkDisk();
      if(!check.ready)throw Error("Диск недоступний: "+check.detail+". Каталог: "+check.path);
      rememberInstall(version,"installing");
      const result=await invoke<string>("install_version",{version});
      rememberInstall(version,"complete",result);
      setMessage(result);
      if(useApp.getState().selected===version)setInstalled(true);
    }catch(e){
      rememberInstall(version,"error",humanError(e));
      setError(humanError(e));
      setPage("Downloads");
      void loadDiagnostics();
    }finally{
      busyRef.current=false;
      setTask("idle");
    }
  }
  async function launch() {
    if(!selected||busyRef.current||task==="running")return;
    if(!installed){await installVersion(selected);return;}
    busyRef.current=true;
    setTask("launching");setError("");setMessage("");setLogs([]);
    try{
      const response=await invoke<string>("launch_game",{version:selected,nickname,memoryMb,instanceId:activeInstanceId||null});
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
        <span className="fre-identity-mark">F<span>↗</span></span>
        <span className="fre-identity-word"><b>FRE.</b><small>LAUNCH / JAVA EDITION</small></span>
      </button>
      <div className="fre-rail-divider"><span>НАВІГАЦІЯ / 12</span><span>•••</span></div>
      <nav className="fre-nav" aria-label="Розділи лаунчера">
        {menus.map(({name,label,icon:Icon,index})=><button key={name}
          className={"fre-nav-link"+(page===name?" is-active":"")}
          onClick={()=>setPage(name)}
          aria-current={page===name?"page":undefined}
          title={label}>
          <span className="fre-nav-index">{index}</span><Icon size={18} strokeWidth={1.8}/>
          <span className="fre-nav-label">{label}</span>
          {page===name&&<ChevronRight size={15} strokeWidth={2}/>}
        </button>)}
      </nav>
      <div className="fre-rail-foot">
        <span className="fre-status-live"><i/> СИСТЕМА ONLINE</span>
        <strong>ALPHA / 0.5</strong>
        <small>НЕОФІЦІЙНИЙ MINECRAFT LAUNCHER</small>
      </div>
    </aside>
    <div className="fre-workspace">
      <header className="fre-commandbar">
        <div className="fre-command-left"><span className="fre-command-square"><Orbit size={17}/></span>
          <span>FRE / CONTROL ROOM</span><span className="fre-command-slash">/</span><b>{titles[page]}</b>
        </div>
        <div className="fre-command-right">
          <button className="fre-command-java" onClick={()=>setPage("Settings")} title="Стан Java"><Coffee size={16}/><span>{java.startsWith("Java в PATH")?"JAVA НЕ ЗНАЙДЕНА":"JAVA READY"}</span></button>
          <button className="fre-command-account" onClick={()=>setPage("Accounts")} title="Вибрати профіль">
            <span className="fre-avatar">{nickname[0]?.toUpperCase()||"P"}</span><span>{nickname}</span><ChevronRight size={15}/>
          </button>
        </div>
      </header>
      <main className="fre-main">
        <div className="fre-page-intro">
          <div className="fre-kicker"><span className="fre-small-line"/> РОЗДІЛ {activeLabel} / 12 <span className="fre-kicker-dot">✦</span> FRE LAUNCH</div>
          <div className="fre-heading-row"><div><h1>{titles[page]}</h1><p>{taglines[page]}</p></div>
            {page!=="Home"&&<button className="fre-return" onClick={()=>setPage("Home")}><House size={16}/> ПУНКТ ЗАПУСКУ <ArrowRight size={15}/></button>}
          </div>
        </div>
        {error&&<div className="fre-alert" role="alert"><AlertTriangle size={20}/><div><strong>ПОМИЛКА ОПЕРАЦІЇ</strong><p>{error}</p></div><button onClick={()=>{setPage("Logs");void loadDiagnostics();}}>ДІАГНОСТИКА <ArrowRight size={15}/></button><button className="fre-alert-close" onClick={()=>setError("")} title="Закрити"><X size={18}/></button></div>}
        {message&&<div className="fre-flash" role="status"><Check size={17}/><span>{message}</span><button onClick={()=>setMessage("")} title="Закрити"><X size={16}/></button></div>}

        {page==="Home"&&<>
          <div className="fre-home-layout">
            <motion.section className="fre-feature" initial={{opacity:0,y:14}} animate={{opacity:1,y:0}} transition={{duration:.35}}>
              <div className="fre-feature-top"><span><span className="fre-feature-star">✳</span> FRE / OVERWORLD COLLECTION</span><span>001 — INFINITE</span></div>
              <div className="fre-feature-content"><div className="fre-cover-label">НЕ ПРОСТО ЛАУНЧЕР.</div>
                <h2>ТУТ<br/>ПОЧИНАЄТЬСЯ<br/><em>ТВІЙ СВІТ<span>↗</span></em></h2>
                <p>Вибери версію. Збери власні правила. А далі — тільки ти та нескінченний світ блоків.</p>
              </div>
              <div className="fre-feature-art"><WorldArt/></div>
              <div className="fre-feature-foot"><span>NO. 001 / WORLD SELECT</span><span>CRAFT YOUR OWN WAY <ArrowRight size={14}/></span></div>
            </motion.section>
            <aside className="fre-console">
              <div className="fre-console-heading"><div><span>01 // LAUNCH CONTROL</span><h3>ГОТОВИЙ<br/>ДО СТАРТУ?</h3></div><div className="fre-console-orbit"><Zap size={21}/></div></div>
              <div className="fre-console-divider"/>
              <span className="fre-field-label">ОБРАНА ВЕРСІЯ</span>
              <button className="fre-version-selection" onClick={()=>setPage("Versions")}><span className="fre-version-symbol"><Layers3 size={24}/></span>
                <span><strong>{selected||"ВИБЕРИ ВЕРСІЮ"}</strong><small>{selectedKind.toUpperCase()} · JAVA EDITION</small></span><ChevronRight size={18}/></button>
              <div className="fre-console-mini"><span>ПРОФІЛЬ</span><button onClick={()=>setPage("Accounts")}><UserRound size={15}/> {nickname} <ChevronRight size={14}/></button></div>
              <div className="fre-console-mini"><span>БІБЛІОТЕКА</span><button onClick={()=>setPage("Instances")}><Boxes size={15}/> {activeInstanceId?"ВИБРАНА ЗБІРКА":"ШВИДКИЙ РЕЖИМ"} <ChevronRight size={14}/></button></div>
              <div className="fre-console-mini"><span>ПАМ'ЯТЬ</span><button onClick={()=>setPage("Settings")}><SlidersHorizontal size={15}/> {Math.round(memoryMb/1024*10)/10} GB RAM <ChevronRight size={14}/></button></div>
              <button className="fre-launch-now" disabled={!selected||isBusy} onClick={()=>void launch()}>
                <span>{installed?<Play size={22} fill="currentColor"/>:<ArrowDownToLine size={22}/>}</span>
                <strong>{installAction}</strong><ArrowRight size={22}/>
              </button>
              <div className="fre-console-bottom">{installed?"ФАЙЛИ ГРИ ВЖЕ ВСТАНОВЛЕНО":"СПОЧАТКУ ЗАВАНТАЖИМО ОФІЦІЙНІ ФАЙЛИ"} <span>✳</span></div>
            </aside>
          </div>
          <div className="fre-quick-links">
            <button onClick={()=>setPage("Instances")}><span>01 / BUILD</span><div><Boxes size={27}/><b>ВЛАСНІ ЗБІРКИ</b><ArrowRight size={20}/></div><p>Твої світи. Твої модлоадери.</p></button>
            <button onClick={()=>setPage("Mods")}><span>02 / EXPLORE</span><div><Puzzle size={27}/><b>МОДИ І РЕСУРСИ</b><ArrowRight size={20}/></div><p>Відкривай нові можливості.</p></button>
            <button onClick={()=>setPage("Skins")}><span>03 / CREATE</span><div><Shirt size={27}/><b>СТУДІЯ СКІНІВ</b><ArrowRight size={20}/></div><p>Створи того, ким станеш.</p></button>
          </div>
          <div className="fre-system-strip"><span><i/> ЛАУНЧЕР ГОТОВИЙ</span><span>JAVA: {truncate(java,38)}</span><button onClick={()=>setPage("Logs")}>ДІАГНОСТИКА <ArrowRight size={15}/></button></div>
        </>}

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

        {page==="Instances"&&<div className="fre-module"><InstancesPanel version={selected} active={activeInstanceId} setActive={setActiveInstanceId} setVersion={setSelected} setMemory={setMemoryMb} navigateHome={()=>setPage("Home")}/></div>}
        {page==="Mods"&&<div className="fre-module"><ModsPanel version={selected} instanceId={activeInstanceId}/></div>}
        {page==="Skins"&&<div className="fre-module"><SkinStudio/></div>}
        {page==="Servers"&&<div className="fre-module"><ServersPanel/></div>}
        {page==="Screenshots"&&<div className="fre-module"><ScreenshotsPanel instanceId={activeInstanceId}/></div>}
        {page==="Backups"&&<div className="fre-module"><BackupsPanel instanceId={activeInstanceId} instanceName={activeInstanceId?"ВИБРАНА ЗБІРКА":""}/></div>}
        {page==="Accounts"&&<div className="fre-module"><AccountsPanel nickname={nickname} onSelect={setNickname}/></div>}

        {page==="Settings"&&<div className="fre-settings-stack">
          <section className="fre-control-paper"><div className="fre-module-label"><span>01 / ВИГЛЯД</span><Wrench size={18}/></div>
            <h2>ТВОЯ АТМОСФЕРА.</h2><p>П'ять настроїв FRE LAUNCH. Вибери свій — усе зберігається автоматично.</p>
            <div className="fre-theme-grid">{themes.map(item=><button key={item.id} className={"fre-theme-choice"+(theme===item.id?" is-selected":"")} onClick={()=>setTheme(item.id)}>
              <span style={{background:item.color}}/><b>{item.label}</b>{theme===item.id?<Check size={18}/>:<ArrowRight size={17}/>}</button>)}</div>
          </section>
          <section className="fre-control-paper"><div className="fre-module-label"><span>02 / РЕСУРСИ</span><SlidersHorizontal size={18}/></div>
            <h2>ПОТУЖНІСТЬ ГРИ.</h2><p>Виділена RAM: <strong>{memoryMb} MB</strong></p>
            <input type="range" min={1024} max={16384} step={512} value={memoryMb} onChange={e=>setMemoryMb(Number(e.target.value))} className="fre-ram-range"/>
            <div className="fre-ram-ends"><span>1 GB</span><span>16 GB</span></div>
            <p>Збірки можуть мати власні Java та RAM.</p>
          </section>
          <section className="fre-control-paper"><div className="fre-module-label"><span>03 / JAVA</span><Coffee size={18}/></div><JavaPanel instanceId={activeInstanceId}/></section>
        </div>}

        {page==="Downloads"&&<InstallDeck
          version={selected}
          progress={progress}
          preflight={preflight}
          isInstalling={task==="installing"}
          onCheck={()=>{void checkDisk();}}
          onLogs={()=>{void loadDiagnostics();setPage("Logs");}}
        />}

        {page==="Logs"&&<section className="fre-diag-sheet">
          <div className="fre-module-label"><span>CRASH REPORT / LOCAL ONLY</span><ShieldCheck size={19}/></div>
          <h2>ЖУРНАЛ ЗАПУСКУ.</h2><p>Тут можна знайти помилку, навіть якщо встановлення зупинилося. Журнал зберігається лише на комп'ютері.</p>
          <div className="fre-log-toolbar"><button onClick={()=>void loadDiagnostics()}><RefreshCw size={16}/> ЧИТАТИ ЖУРНАЛ</button>
            <button disabled={!diagnostics?.contents} onClick={()=>void copyText(diagnostics?.contents??"")}><Copy size={16}/> КОПІЮВАТИ ПОМИЛКИ</button>
            <button disabled={!logs.length} onClick={()=>void copyText(logs.join("\n"))}><Copy size={16}/> КОПІЮВАТИ ЛОГ ГРИ</button>
            <button onClick={()=>setLogs([])}><X size={16}/> ОЧИСТИТИ ЕКРАН</button></div>
          {diagnostics&&<div className="fre-diagnostic-location"><FolderOpen size={16}/><span>{diagnostics.logPath}</span></div>}
          <div className="fre-log-label">FRE / INSTALL DIAGNOSTICS</div>
          <pre className="fre-log-output">{diagnostics?.contents||"Натисни «Читати журнал», щоб показати історію встановлень."}</pre>
          <div className="fre-log-label">INTERFACE / LAST JS FAILURE</div>
          <pre className="fre-log-output">{lastUiCrash||"Інтерфейс не зберіг повідомлень про попередні JavaScript-помилки."}</pre>
          {lastUiCrash&&<div className="fre-log-toolbar"><button onClick={()=>void copyText(lastUiCrash)}><Copy size={16}/> КОПІЮВАТИ ЗБІЙ UI</button><button onClick={()=>{try{localStorage.removeItem(LAST_UI_CRASH)}catch{}setLastUiCrash("");}}><X size={16}/> ОЧИСТИТИ ЗБЕРЕЖЕНУ ПОМИЛКУ</button></div>}
          <div className="fre-log-label">MINECRAFT / LIVE OUTPUT</div>
          <pre className="fre-log-output">{logs.length?logs.join("\n"):"Minecraft ще не надсилав повідомлень."}</pre>
          <p>Якщо програма закрилася під час встановлення, після повторного запуску відкрий цю сторінку та скопіюй журнал.</p>
        </section>}

        <footer className="fre-footer"><span>FRE LAUNCH © ALPHA 0.5</span><span>YOUR MINECRAFT / YOUR RULES</span><button onClick={()=>{setShowMore(v=>!v);}}>ПРО ПРОЄКТ <CircleHelp size={14}/></button></footer>
        {showMore&&<div className="fre-disclaimer"><button onClick={()=>setShowMore(false)}><X size={16}/></button>FRE LAUNCH — неофіційний Minecraft Java лаунчер. Не пов'язаний з Mojang чи Microsoft. Перевірка реального запуску на всіх Windows-конфігураціях ще триває.</div>}
      </main>
    </div>
  </div>;
}
