import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { motion } from "motion/react";
import { House, Layers3, Puzzle, UserRound, Shirt, Server, Download, ScrollText, Settings2, Archive, Search, RefreshCw, Play, Coffee, ShieldCheck, Sparkles, ChevronDown, HardDriveDownload, Terminal, CheckCircle2, AlertTriangle, Copy } from "lucide-react";
import { create } from "zustand";
import { InstancesPanel, ModsPanel } from "./Features";
import SkinStudio from "./SkinStudio";
import BackupsPanel from "./BackupsPanel";
import AccountsPanel from "./AccountsPanel";
import JavaPanel from "./JavaPanel";
import { persist } from "zustand/middleware";

type Version = { id: string; type: string; url: string; time: string; releaseTime: string };
type Progress = { stage: string; current: number; total: number; message: string };
type Page = "Home" | "Versions" | "Instances" | "Mods" | "Skins" | "Accounts" | "Servers" | "Downloads" | "Logs" | "Backups" | "Settings";
type Task = "idle" | "installing" | "launching" | "running";
type Store = {
  page: Page; setPage: (page: Page) => void;
  selected: string; setSelected: (version: string) => void;
  nickname: string; setNickname: (name: string) => void;
  memoryMb: number; setMemoryMb: (value: number) => void;
  activeInstanceId: string; setActiveInstanceId: (value: string) => void;
  theme: string; setTheme: (value: string) => void;
};
const useApp = create<Store>()(persist((set) => ({
  page: "Home", setPage: (page) => set({page}),
  selected: "", setSelected: (selected) => set({selected}),
  nickname: "Player", setNickname: (nickname) => set({nickname}),
  memoryMb: 4096, setMemoryMb: (memoryMb) => set({memoryMb}),
  activeInstanceId: "", setActiveInstanceId: (activeInstanceId) => set({activeInstanceId}),
  theme: "midnight", setTheme: (theme) => set({theme})
}), {name: "fre-launch-preferences"}));

const menus = [
  {name: "Home", icon: House, label: "Головна"},
  {name: "Versions", icon: Layers3, label: "Версії"},
  {name: "Instances", icon: Layers3, label: "Збірки"},
  {name: "Mods", icon: Puzzle, label: "Моди"},
  {name: "Skins", icon: Shirt, label: "Скіни"},
  {name: "Accounts", icon: UserRound, label: "Акаунти"},
  {name: "Servers", icon: Server, label: "Сервери"},
  {name: "Downloads", icon: Download, label: "Завантаження"},
  {name: "Logs", icon: ScrollText, label: "Логи"},
  {name: "Backups", icon: Archive, label: "Копії"},
  {name: "Settings", icon: Settings2, label: "Налаштування"}
] as const;
const labels: Record<Page, string> = {
  Home: "Головна", Versions: "Версії Minecraft", Instances: "Мої збірки",
  Mods: "Моди та модпаки", Skins: "Студія скінів", Accounts: "Акаунти",
  Servers: "Сервери", Downloads: "Завантаження", Logs: "Логи та діагностика",
  Backups: "Резервні копії", Settings: "Налаштування"
};
const future: Record<string, string> = {
  Instances: "Керування окремими збірками та модлоадерами буде додано після стабілізації запуску.",
  Mods: "Інтеграція Fabric, Forge, NeoForge, Quilt та Modrinth ще не готова.",
  Skins: "3D-перегляд і застосування скінів буде додано в наступних версіях.",
  Servers: "Менеджер серверів перебуває в плані розробки.",
  Backups: "Резервні копії світів та профілів ще не реалізовані."
};

export default function App() {
  const {page, setPage, selected, setSelected, nickname, setNickname, memoryMb, setMemoryMb, activeInstanceId, setActiveInstanceId, theme, setTheme} = useApp();
  const [versions, setVersions] = useState<Version[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("all");
  const [java, setJava] = useState("Перевірка Java...");
  const [task, setTask] = useState<Task>("idle");
  const [installed, setInstalled] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [message, setMessage] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  useEffect(() => {document.documentElement.dataset.theme = theme;}, [theme]);

  async function refresh() {
    setLoading(true);
    setError("");
    try {
      const data = await invoke<Version[]>("get_versions");
      setVersions(data);
      if (!useApp.getState().selected && data.length) {
        setSelected(data.find((v) => v.type === "release")?.id ?? data[0].id);
      }
    } catch (e) {
      setError(String(e));
    } finally { setLoading(false); }
  }

  useEffect(() => {
    void refresh();
    void invoke<string>("java_info").then(setJava).catch((e) => setJava(String(e)));
    const a = listen<Progress>("install-progress", (event) => setProgress(event.payload));
    const b = listen<string>("game-log", (event) => {
      setLogs((prev) => [...prev.slice(-499), event.payload]);
    });
    const c = listen<string>("game-status", (event) => {
      const status = event.payload;
      setMessage(status === "running" ? "Гра запущена" : status);
      setTask(status === "running" ? "running" : "idle");
    });
    return () => { void a.then((stop) => stop()); void b.then((stop) => stop()); void c.then((stop) => stop()); };
  }, []);

  useEffect(() => {
    let alive = true;
    setInstalled(false);
    if (selected) {
      void invoke<boolean>("is_installed", {version: selected}).then((yes) => {if (alive) setInstalled(yes);})
        .catch((e) => {if (alive) setError(String(e));});
    }
    return () => {alive = false;};
  }, [selected]);

  const filtered = useMemo(() => versions.filter((v) =>
    (filter === "all" || v.type === filter) && v.id.toLowerCase().includes(search.toLowerCase())
  ).slice(0, 150), [versions, filter, search]);

  async function installVersion(version: string) {
    if (!version || task !== "idle") return;
    setSelected(version);
    setTask("installing");
    setError("");
    setMessage("");
    setProgress(null);
    try {
      const result = await invoke<string>("install_version", {version});
      setInstalled(true);
      setMessage(result);
    } catch (e) { setError(String(e)); }
    finally { setTask("idle"); }
  }

  async function play() {
    if (!selected || task !== "idle") return;
    if (!installed) { await installVersion(selected); return; }
    setTask("launching");
    setError("");
    setMessage("");
    setLogs([]);
    try {
      const result = await invoke<string>("launch_game", {version: selected, nickname, memoryMb, instanceId: activeInstanceId || null});
      setMessage(result);
    } catch (e) {
      setError(String(e));
      setTask("idle");
    }
  }

  const busy = task === "installing" || task === "launching" || task === "running";
  const playLabel = task === "installing" ? "ВСТАНОВЛЕННЯ..." : task === "launching" ? "ЗАПУСК..." :
    task === "running" ? "ГРА ЗАПУЩЕНА" : installed ? "ГРАТИ" : "ВСТАНОВИТИ";
  return <div className="shell">
    <aside className="sidebar">
      <div className="brand"><div className="brand-icon">F</div><div><strong>FRE <span>LAUNCH</span></strong><small>YOUR MINECRAFT. YOUR RULES.</small></div></div>
      <div className="nav">
        {menus.map(({name, icon: Icon, label}) => <button key={name} className={"nav-item " + (page === name ? "active" : "")} onClick={() => setPage(name)}><Icon size={19}/><span>{label}</span></button>)}
      </div>
      <div className="side-bottom"><div className="status-dot"/> Версія 0.3.0 · Alpha</div>
    </aside>
    <main className="main">
      <header className="top"><div><div className="eyebrow">FRE LAUNCH / {page.toUpperCase()}</div><h1>{labels[page]}</h1></div><button className="user-pill" onClick={() => setPage("Accounts")}><div className="avatar">{nickname[0]?.toUpperCase() || "P"}</div><div><b>{nickname}</b><small>Offline profile</small></div><ChevronDown size={16}/></button></header>
      {error && <div className="error"><AlertTriangle size={17}/> {error}</div>}
      {message && <div className="notice"><CheckCircle2 size={17}/> {message}</div>}
      {task === "installing" && progress && <div className="progress-panel"><div><b>{progress.message}</b><span>{progress.total ? Math.floor(progress.current / progress.total * 100) : 0}%</span></div><div className="progress-bar"><div style={{width: progress.total ? Math.floor(progress.current / progress.total * 100) + "%" : "0%"}}/></div></div>}
      {page === "Home" && <>
        <motion.section className="hero" initial={{opacity: 0, y: 14}} animate={{opacity: 1, y: 0}}>
          <div className="hero-glow"/>
          <div className="hero-content"><span className="pill"><Sparkles size={14}/> FRE LAUNCH ALPHA</span><h2>Твій світ.<br/><em>Твої правила.</em></h2><p>Обери Minecraft Java Edition та вирушай у пригоду.</p>
            <div className="hero-actions"><button className="play" disabled={!selected || busy} onClick={() => void play()}>{installed ? <Play fill="currentColor" size={20}/> : <HardDriveDownload size={20}/>} {playLabel}</button><button className="secondary" onClick={() => setPage("Versions")}>Усі версії</button></div>
          </div><div className="hero-decoration">F<span>RE</span></div>
        </motion.section>
        <div className="grid2">
          <section className="panel"><div className="panel-heading"><h3>Обрана версія</h3><Layers3 size={19}/></div><div className="value">{selected || "Не вибрано"}</div><p>{installed ? "Позначена як встановлена" : "Потрібне завантаження"}</p><p className="muted">{activeInstanceId ? "Окрема збірка активна · власні світи" : "Швидкий запуск · спільний профіль версії"}</p><button className="text-button" onClick={() => setPage("Versions")}>Змінити версію →</button></section>
          <section className="panel"><div className="panel-heading"><h3>Система</h3><Coffee size={19}/></div><div className="system-status">{java}</div><p>Java визначається через PATH. Для різних версій потрібні сумісні JDK.</p><button className="text-button" onClick={() => void invoke<string>("java_info").then(setJava).catch((e) => setJava(String(e)))}>Перевірити Java →</button></section>
        </div>
      </>}
      {page === "Versions" && <section className="panel"><div className="toolbar"><div className="search"><Search size={18}/><input placeholder="Знайти версію, наприклад 1.21..." value={search} onChange={(e) => setSearch(e.target.value)}/></div><select value={filter} onChange={(e) => setFilter(e.target.value)}><option value="all">Усі типи</option><option value="release">Release</option><option value="snapshot">Snapshot</option><option value="old_beta">Old Beta</option><option value="old_alpha">Old Alpha</option></select><button className="icon-button" onClick={() => void refresh()} title="Оновити" disabled={loading}><RefreshCw size={18}/></button></div>
        <p className="muted">{loading ? "Завантаження з Mojang..." : `Знайдено ${versions.length} версій · показано ${filtered.length}`}</p>
        <div className="version-list">{filtered.map((v) => <button className={"version-row " + (selected === v.id ? "chosen" : "")} key={v.id} onClick={() => {setSelected(v.id); setActiveInstanceId("");}}><div className="version-mark"><Layers3 size={19}/></div><div><b>{v.id}</b><small>{v.type.replace("_", " ").toUpperCase()}</small></div><span>{new Date(v.releaseTime).toLocaleDateString("uk-UA")}</span><span className="choose">{selected === v.id ? "Обрано" : "Обрати"}</span></button>)}</div>
        <div className="toolbar version-footer"><b>Обрано: {selected || "нічого"}</b><button className="primary" disabled={!selected || busy} onClick={() => void installVersion(selected)}><HardDriveDownload size={17}/> Встановити / перевірити</button></div>
        <p className="muted">Сумісність історичних версій може відрізнятися. Підтримка окремих модлоадерів ще не готова.</p>
      </section>}
      {page === "Instances" && <InstancesPanel version={selected} active={activeInstanceId} setActive={setActiveInstanceId} setVersion={setSelected} setMemory={setMemoryMb} navigateHome={() => setPage("Home")}/>}
      {page === "Mods" && <ModsPanel version={selected} instanceId={activeInstanceId}/>}
      {page === "Skins" && <SkinStudio/>}
      {page === "Backups" && <BackupsPanel instanceId={activeInstanceId} instanceName={activeInstanceId ? "Вибрана збірка" : ""}/>}
      {page === "Accounts" && <AccountsPanel nickname={nickname} onSelect={setNickname}/>}
      {page === "Settings" && <section className="panel narrow">
        <div className="panel-heading"><h3>FRE THEMES</h3><Settings2 size={19}/></div>
        <p>Зміна теми застосовується миттєво і зберігається після перезапуску лаунчера.</p>
        <div className="theme-options">{[
          {id:"midnight", label:"Midnight",color:"#9063f4"},
          {id:"neon",label:"Neon",color:"#df4dff"},
          {id:"ocean",label:"Ocean",color:"#15b5df"},
          {id:"emerald",label:"Emerald",color:"#35c99d"},
          {id:"light",label:"Light",color:"#7790fa"}
        ].map(t=><button className={"theme-choice"+(theme===t.id?" current":"")} onClick={()=>setTheme(t.id)} key={t.id}>
          <span style={{backgroundColor:t.color}}/>{t.label}{theme===t.id?" ✓":""}</button>)}</div>
        <hr/><div className="panel-heading"><h3>Налаштування гри</h3><Settings2 size={19}/></div>
        <label className="field-label">Максимальна RAM: {memoryMb} MB</label>
        <input className="memory-slider" type="range" min="1024" max="16384" step="512" value={memoryMb} onChange={(e) => setMemoryMb(Number(e.target.value))}/>
        <p>RAM зберігається локально. Окремі збірки можуть мати власні налаштування у FRE INSTANCES.</p>
        <hr/><div className="panel-heading"><h3>Java</h3><Coffee size={19}/></div>
        <p>{java}</p><p className="muted">Для власного шляху до Java відкрий налаштування конкретної збірки.</p>
        <JavaPanel instanceId={activeInstanceId}/>
      </section>}
      {page === "Downloads" && <section className="panel narrow"><div className="panel-heading"><h3>Стан встановлення</h3><Download size={19}/></div>{progress ? <><p>{progress.message}</p><p>{progress.stage}: {progress.current}/{progress.total}</p></> : <p>Наразі немає активних завантажень.</p>}</section>}
      {page === "Logs" && <section className="panel"><div className="panel-heading"><h3>Журнал Minecraft</h3><Terminal size={19}/></div><div className="toolbar"><button className="secondary" disabled={!logs.length} onClick={() => void navigator.clipboard.writeText(logs.join("\n"))}><Copy size={15}/> Копіювати логи</button><button className="secondary" onClick={() => setLogs([])}>Очистити</button></div><pre className="log-view">{logs.length ? logs.join("\n") : "Логи з’являться тут після запуску Minecraft."}</pre></section>}
      {page in future && !["Instances","Mods","Skins","Backups"].includes(page) && <section className="panel empty"><ShieldCheck size={38}/><h3>Модуль у розробці</h3><p>{future[page]}</p><button className="secondary" onClick={() => setPage("Home")}>На головну</button></section>}
      <footer>FRE LAUNCH <span>•</span> Your Minecraft. Your Rules. <span>•</span> Alpha build</footer>
    </main>
  </div>;
}
