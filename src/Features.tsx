import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import FabricOptions from "./FabricOptions";
import GameFolders from "./GameFolders";
import { Boxes, Plus, Star, Trash2, RefreshCw, DownloadCloud, Search, Package, ShieldCheck, Pencil } from "lucide-react";

export type Instance = {
  id: string;
  name: string;
  version: string;
  favorite: boolean;
  memoryMb: number;
  javaPath: string | null;
  loader: string;
  loaderVersion: string | null;
};
type Hit = {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  icon_url: string | null;
  downloads: number;
  project_type: string;
};
type InstancesProps = {
  version: string;
  active: string;
  setActive: (id: string) => void;
  setVersion: (version: string) => void;
  setMemory: (memoryMb: number) => void;
  navigateHome: () => void;
};

export function InstancesPanel({version, active, setActive, setVersion, setMemory, navigateHome}: InstancesProps) {
  const [instances, setInstances] = useState<Instance[]>([]);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [folderFor, setFolderFor] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editJava, setEditJava] = useState("");
  const [editMemory, setEditMemory] = useState(4096);

  async function refresh() {
    setBusy(true);
    try { setInstances(await invoke<Instance[]>("list_instances")); setError(""); }
    catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  }

  useEffect(() => { void refresh(); }, []);
  async function create() {
    if (!name.trim() || !version) return;
    setBusy(true);
    try {
      const created = await invoke<Instance>("create_instance", {name, version});
      setName("");
      setActive(created.id);
      setMemory(created.memoryMb);
      await refresh();
    } catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  }

  async function remove(instance: Instance) {
    if (!window.confirm(`Прибрати збірку «${instance.name}» зі списку? Ігрові файли залишаться на диску.`)) return;
    setBusy(true);
    try {
      await invoke("delete_instance", {id: instance.id, deleteFiles: false});
      if (active === instance.id) setActive("");
      await refresh();
    } catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  }

  async function favorite(instance: Instance) {
    try {
      await invoke("update_instance", {id: instance.id, name: instance.name, favorite: !instance.favorite, memoryMb: instance.memoryMb, javaPath: instance.javaPath});
      await refresh();
    } catch (err) { setError(String(err)); }
  }

  function beginEdit(instance: Instance) {
    setEditing(instance.id);
    setEditName(instance.name);
    setEditMemory(instance.memoryMb);
    setEditJava(instance.javaPath ?? "");
  }

  async function saveEdit(instance: Instance) {
    setBusy(true);
    try {
      await invoke("update_instance", {id: instance.id, name: editName, favorite: instance.favorite, memoryMb: editMemory, javaPath: editJava.trim() || null});
      setEditing(null);
      if (active === instance.id) setMemory(editMemory);
      await refresh();
    } catch (err) {setError(String(err));}
    finally {setBusy(false);}
  }

  return <section className="panel">
    <div className="panel-heading"><h3>FRE INSTANCES</h3><Boxes size={20}/></div>
    <p>Незалежні збірки з окремими світами, ресурс-паками та налаштуваннями Java.</p>
    {error && <div className="error">{error}</div>}
    <div className="toolbar create-form">
      <input className="input" placeholder="Назва нової збірки" value={name} maxLength={48} onChange={(e) => setName(e.target.value)}/>
      <span className="version-chip">Minecraft {version || "—"}</span>
      <button className="primary" disabled={!name.trim() || !version || busy} onClick={() => void create()}><Plus size={17}/> Створити</button>
      <button className="icon-button" disabled={busy} title="Оновити" onClick={() => void refresh()}><RefreshCw size={16}/></button>
    </div>
    {instances.length === 0 && <div className="small-empty"><Boxes size={35}/><h3>Поки немає збірок</h3><p>Створи першу для незалежного зберігання світів і налаштувань.</p></div>}
    <div className="instance-grid">
      {instances.map((instance) => <div key={instance.id} className={"instance-card" + (active === instance.id ? " chosen" : "")}>
        <div className="instance-icon"><Boxes size={24}/></div>
        <div className="instance-info"><b>{instance.name}</b><span>{instance.version} · {instance.loader==="fabric"||instance.loader==="quilt"?`${instance.loader==="fabric"?"Fabric":"Quilt"} ${instance.loaderVersion??"?"}`:"Vanilla"} · {instance.memoryMb} MB RAM</span></div>
        <button className={"icon-button" + (instance.favorite ? " starred" : "")} title="Обране" onClick={() => void favorite(instance)}><Star size={17} fill={instance.favorite ? "currentColor" : "none"}/></button>
        <button className="icon-button" title="Редагувати" onClick={() => editing === instance.id ? setEditing(null) : beginEdit(instance)}><Pencil size={17}/></button>
        <button className="secondary" onClick={()=>setFolderFor(folderFor===instance.id?null:instance.id)}>{folderFor===instance.id?"Сховати папки":"Папки гри"}</button>
        <button className="secondary" onClick={() => {setActive(instance.id); setVersion(instance.version); setMemory(instance.memoryMb); navigateHome();}}>Обрати</button>
        <button className="icon-button danger" title="Прибрати (без видалення світів)" onClick={() => void remove(instance)}><Trash2 size={17}/></button>
        {folderFor===instance.id&&<div className="instance-editor"><GameFolders instanceId={instance.id} version={instance.version}/></div>}
        {editing === instance.id && <div className="instance-editor">
          <label className="field-label">Назва</label>
          <input className="input" value={editName} onChange={(e) => setEditName(e.target.value)} maxLength={48}/>
          <label className="field-label">Пам’ять: {editMemory} MB</label>
          <input className="memory-slider" type="range" min={1024} max={16384} step={512} value={editMemory} onChange={(e) => setEditMemory(Number(e.target.value))}/>
          <label className="field-label">Java executable (необов'язково)</label>
          <input className="input wide" value={editJava} onChange={(e) => setEditJava(e.target.value)} placeholder="C:\\Program Files\\Java\\bin\\java.exe"/>
          <div className="toolbar"><button className="primary" disabled={busy} onClick={() => void saveEdit(instance)}>Зберегти</button><button className="secondary" onClick={() => setEditing(null)}>Скасувати</button></div>
          <FabricOptions instance={instance} onSaved={refresh}/>
        </div>}
      </div>)}
    </div>
    <p className="muted">Видалення збірки зі списку не видаляє світи. Перед ручним видаленням папок зроби резервну копію.</p>
  </section>;
}

export function ModsPanel({version, instanceId}: {version: string; instanceId: string}) {
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState("resourcepack");
  const [hits, setHits] = useState<Hit[]>([]);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  async function search() {
    setLoading(true);
    setError("");
    setMessage("");
    try {
      const results = await invoke<Hit[]>("search_modrinth", {query, version: version || null, projectType: kind});
      setHits(results);
    } catch (err) {setError(String(err));setHits([]);}
    finally {setLoading(false);}
  }
  useEffect(() => { void search(); }, [kind, version]);

  async function installPack(hit: Hit) {
    if (!instanceId) {setError("Спочатку створи та обери збірку у розділі «Збірки»."); return;}
    setInstalling(hit.project_id);
    setError("");
    setMessage("");
    try {
      setMessage(await invoke<string>("install_resourcepack", {instanceId, projectId: hit.project_id}));
    } catch (err) {setError(String(err));}
    finally {setInstalling(null);}
  }

  return <section className="panel">
    <div className="panel-heading"><h3>FRE MODRINTH</h3><Package size={20}/></div>
    <p>Пошук справжніх проєктів Modrinth. Ресурс-паки встановлюються з перевіркою файлів. Підтримку Fabric можна вибрати в налаштуваннях збірки; встановлення модів додамо після перевірки залежностей.</p>
    <div className="toolbar"><div className="search"><Search size={18}/><input placeholder="Назва моду чи ресурс-пака..." value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => {if(e.key==="Enter")void search();}}/></div>
      <select value={kind} onChange={(e) => setKind(e.target.value)}>
        <option value="resourcepack">Ресурс-паки</option><option value="mod">Моди</option><option value="shader">Шейдери</option><option value="modpack">Модпаки</option>
      </select>
      <button className="primary" disabled={loading} onClick={() => void search()}>{loading ? "Шукаю..." : "Знайти"}</button>
    </div>
    {error && <div className="error">{error}</div>}
    {message && <div className="notice"><ShieldCheck size={16}/>{message}</div>}
    <p className="muted">{hits.length} результатів · Minecraft {version || "всі версії"} · Джерело: Modrinth</p>
    <div className="mod-grid">
      {hits.map((hit) => <article className="mod-card" key={hit.project_id}>
        {hit.icon_url ? <img className="mod-icon" src={hit.icon_url} alt=""/> : <span className="mod-icon fallback"><Package size={25}/></span>}
        <div className="mod-copy"><h3>{hit.title}</h3><p>{hit.description}</p><small><DownloadCloud size={13}/> {hit.downloads.toLocaleString("uk-UA")} завантажень</small></div>
        {kind === "resourcepack" ?
          <button className="secondary" disabled={!!installing || !instanceId} onClick={() => void installPack(hit)} title={instanceId ? "Встановити до вибраної збірки" : "Спочатку створи збірку"}>{installing === hit.project_id ? "..." : "Встановити"}</button> :
          <span className="muted">Перегляд</span>}
      </article>)}
    </div>
  </section>;
}
