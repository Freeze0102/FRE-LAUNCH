import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Package, DownloadCloud, Search, ShieldCheck, Trash2, RefreshCw } from "lucide-react";
import type { Instance } from "./Features";

type ModrinthHit = {
  project_id: string; slug: string; title: string; description: string;
  icon_url: string | null; downloads: number; project_type: string;
};
type InstalledMod = {filename: string; enabled: boolean; bytes: number};

export default function ModsPanel({version, instanceId}: {version: string; instanceId: string}) {
  const [query,setQuery]=useState("");
  const [kind,setKind]=useState("mod");
  const [hits,setHits]=useState<ModrinthHit[]>([]);
  const [onlyCompatible,setOnlyCompatible]=useState(false);
  const [offset,setOffset]=useState(0);
  const [selectedInstance,setSelectedInstance]=useState<Instance|null>(null);
  const [installed,setInstalled]=useState<InstalledMod[]>([]);
  const [loading,setLoading]=useState(false);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState("");
  const [message,setMessage]=useState("");

  async function refreshMods() {
    if (!instanceId) {setSelectedInstance(null);setInstalled([]);return;}
    try {
      const profiles=await invoke<Instance[]>("list_instances");
      setSelectedInstance(profiles.find(p=>p.id===instanceId)??null);
      setInstalled(await invoke<InstalledMod[]>("list_installed_mods",{instanceId}));
    } catch(err){setError(String(err));}
  }
  useEffect(()=>{void refreshMods();},[instanceId]);

  async function search(pageOffset=0) {
    setLoading(true);setError("");
    try {
      const results=await invoke<ModrinthHit[]>("search_modrinth", {
        query:query.trim(),version:onlyCompatible ? version || null : null,projectType:kind,offset:pageOffset,
      });
      setHits(results);
      setOffset(pageOffset);
    } catch(err){setError(String(err));setHits([]);}
    finally{setLoading(false);}
  }
  useEffect(()=>{void search(0);},[kind,version,onlyCompatible]);

  async function install(hit: ModrinthHit) {
    if (!instanceId || !selectedInstance){setError("Спочатку створи та обери Minecraft-збірку.");return;}
    setBusy(true);setError("");setMessage("");
    try {
      if (kind==="resourcepack") {
        setMessage(await invoke<string>("install_resourcepack",{instanceId,projectId:hit.project_id}));
      } else if (kind==="mod") {
        if (selectedInstance.loader!=="fabric")throw Error("Для встановлення модів вибери Fabric у налаштуваннях збірки.");
        const added=await invoke<string[]>("install_fabric_mod",{instanceId,projectId:hit.project_id});
        setMessage(`Встановлено ${added.length} модів разом із потрібними залежностями: ${added.join(", ")}`);
        await refreshMods();
      }
    } catch(err){setError(String(err));}
    finally{setBusy(false);}
  }
  async function toggle(mod:InstalledMod){
    setBusy(true);setError("");setMessage("");
    try {
      await invoke("toggle_installed_mod",{instanceId,filename:mod.filename});
      await refreshMods();
    } catch(err){setError(String(err));}
    finally{setBusy(false);}
  }
  async function remove(mod:InstalledMod){
    if (!window.confirm(`Видалити мод «${mod.filename}»? Ігрові світи не будуть видалені.`))return;
    setBusy(true);setError("");setMessage("");
    try {
      await invoke("remove_installed_mod",{instanceId,filename:mod.filename});
      await refreshMods();
    } catch(err){setError(String(err));}
    finally{setBusy(false);}
  }

  return <section className="panel">
    <div className="panel-heading"><h3>FRE MODRINTH</h3><Package size={20}/></div>
    <p>Офіційний каталог Modrinth: моди, ресурс-паки, шейдери й модпаки. Моди Fabric та ресурс-паки можна встановлювати з перевіркою SHA-512.</p>
    <div className="toolbar">
      <div className="search"><Search size={18}/><input placeholder="Знайти Sodium, Lithium, Fabric API..." value={query} onChange={e=>setQuery(e.target.value)} onKeyDown={e=>{if(e.key==="Enter")void search(0);}}/></div>
      <select value={kind} onChange={e=>setKind(e.target.value)}>
        <option value="mod">Моди</option>
        <option value="resourcepack">Ресурс-паки</option>
        <option value="shader">Шейдери</option>
        <option value="modpack">Модпаки</option>
      </select>
      <button className="primary" onClick={()=>void search(0)} disabled={loading}>{loading?"Шукаю...":"Знайти"}</button>
    </div>
    <label className="muted" style={{display:"flex",alignItems:"center",gap:8,marginTop:12}}>
      <input type="checkbox" checked={onlyCompatible} onChange={e=>setOnlyCompatible(e.target.checked)}/>
      Показувати лише проєкти для Minecraft {version||"обраної версії"}
    </label>
    <p className="muted">Пошук у всьому каталозі Modrinth. Без галочки показуються також проєкти для інших версій, але встановлення потребує сумісного релізу.</p>
    <p className="muted">{instanceId ? `Збірка: ${selectedInstance?.name??"Обрана"} · ${selectedInstance?.loader??"Vanilla"}` : "Спочатку обери окрему збірку для встановлення."} · Minecraft {version||"—"}</p>
    {error&&<div className="error">{error}</div>}
    {message&&<div className="notice"><ShieldCheck size={16}/>{message}</div>}
    <div className="mod-grid">
      {hits.map(hit=><article className="mod-card" key={hit.project_id}>
        {hit.icon_url ? <img className="mod-icon" src={hit.icon_url} alt=""/> : <span className="mod-icon fallback"><Package size={24}/></span>}
        <div className="mod-copy"><h3>{hit.title}</h3><p>{hit.description}</p><small><DownloadCloud size={13}/>{hit.downloads.toLocaleString("uk-UA")} завантажень</small></div>
        {(kind==="mod"||kind==="resourcepack")
          ? <button className="secondary" disabled={busy || !selectedInstance || (kind==="mod"&&selectedInstance.loader!=="fabric")} title={kind==="mod"&&selectedInstance?.loader!=="fabric"?"Потрібна Fabric-збірка":"Встановити"} onClick={()=>void install(hit)}>{busy?"...":"Встановити"}</button>
          : <span className="muted">Перегляд</span>}
      </article>)}
    </div>
    <div className="toolbar" style={{justifyContent:"space-between",marginTop:12}}>
      <button className="secondary" disabled={loading||offset===0} onClick={()=>void search(Math.max(0,offset-24))}>← Назад</button>
      <span className="muted">{hits.length ? `Результати ${offset+1}–${offset+hits.length}` : "Нічого не знайдено. Спробуй прибрати фільтр версії."}</span>
      <button className="secondary" disabled={loading||hits.length<24||offset>=984} onClick={()=>void search(offset+24)}>Далі →</button>
    </div>
    {!!instanceId&&<div className="installed-mods">
      <div className="panel-heading"><h3>Встановлені моди</h3><button className="icon-button" title="Оновити" onClick={()=>void refreshMods()}><RefreshCw size={16}/></button></div>
      {installed.length===0&&<p className="muted">Немає встановлених модів у цій збірці.</p>}
      {installed.map(mod=><div className="installed-mod-row" key={mod.filename}>
        <div><b>{mod.filename}</b><span>{(mod.bytes/1024/1024).toFixed(1)} MB · {mod.enabled?"Увімкнено":"Вимкнено"}</span></div>
        <button className="secondary" disabled={busy} onClick={()=>void toggle(mod)}>{mod.enabled?"Вимкнути":"Увімкнути"}</button>
        <button className="icon-button danger" title="Видалити" disabled={busy} onClick={()=>void remove(mod)}><Trash2 size={16}/></button>
      </div>)}
    </div>}
    <p className="muted">Моди — сторонній виконуваний код. Встановлюй лише перевірені проєкти. Вимкнення необхідної залежності може завадити запуску гри.</p>
  </section>;
}
