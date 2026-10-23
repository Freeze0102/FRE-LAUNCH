import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Package, DownloadCloud, Search, ShieldCheck, Trash2, RefreshCw, ArrowLeft, CheckCircle2, Info } from "lucide-react";
import type { Instance } from "./Features";

type ModrinthHit = {
  project_id: string; slug: string; title: string; description: string;
  icon_url: string | null; downloads: number; project_type: string;
};
type ModVersion = {id:string;name:string;version_number:string;game_versions:string[];loaders:string[];date_published:string};
type ModDetails = {project:{title:string;body:string;description:string;icon_url:string|null;downloads:number;project_type:string;categories:string[]};versions:ModVersion[]};
type InstalledMod = {filename: string; enabled: boolean; bytes: number};

export default function ModsPanel({version, instanceId}: {version: string; instanceId: string}) {
  const [query,setQuery]=useState("");
  const [kind,setKind]=useState("mod");
  const [hits,setHits]=useState<ModrinthHit[]>([]);
  const [onlyCompatible,setOnlyCompatible]=useState(true);
  const [offset,setOffset]=useState(0);
  const [opened,setOpened]=useState<ModrinthHit|null>(null);
  const [details,setDetails]=useState<ModDetails|null>(null);
  const [detailBusy,setDetailBusy]=useState(false);
  const [chosenVersion,setChosenVersion]=useState("");
  const [showAllVersions,setShowAllVersions]=useState(false);
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

  async function openProject(hit:ModrinthHit) {
    setOpened(hit);setDetails(null);setChosenVersion("");setShowAllVersions(false);setDetailBusy(true);setError("");
    try {
      const data=await invoke<ModDetails>("modrinth_project_details",{projectId:hit.project_id});
      setDetails(data);
      const matches=data.versions.filter(v=>v.game_versions.includes(version)&& (kind!=="mod"||v.loaders.includes(selectedInstance?.loader||"fabric")));
      setChosenVersion(matches[0]?.id||"");
    } catch(e){setError(String(e));}
    finally{setDetailBusy(false);}
  }
  const compatibleVersions=details?.versions.filter(v=>v.game_versions.includes(version)&&
    (kind!=="mod"||v.loaders.includes(selectedInstance?.loader||"fabric")))??[];
  const shownVersions=showAllVersions?details?.versions??[]:compatibleVersions;
  async function installSelected() {
    if(!opened||!selectedInstance||!instanceId){setError("Спочатку обери збірку.");return;}
    if(!chosenVersion){setError("Немає сумісної версії для обраної збірки.");return;}
    setBusy(true);setError("");setMessage("");
    try {
      if(opened.project_type==="mod") {
        if(selectedInstance.loader!=="fabric")throw Error("Зараз автоматичне встановлення модів підтримує Fabric. Обери Fabric-збірку.");
        const added=await invoke<string[]>("install_fabric_mod",{instanceId,projectId:opened.project_id,versionId:chosenVersion});
        setMessage(`Встановлено: ${added.join(", ")}`);
        await refreshMods();
      } else if(opened.project_type==="resourcepack") {
        setMessage(await invoke<string>("install_resourcepack",{instanceId,projectId:opened.project_id,versionId:chosenVersion}));
      } else {throw Error("Автоматичне встановлення цього типу поки не підтримується.");}
    } catch(e){setError(String(e));}finally{setBusy(false);}
  }

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
    {opened&&<section className="fre-mod-detail">
      <button className="secondary" onClick={()=>{setOpened(null);setDetails(null);setError("");}}><ArrowLeft size={16}/> До каталогу</button>
      <div className="fre-mod-detail-hero">
        {opened.icon_url&&<img src={opened.icon_url} alt="" className="fre-mod-detail-icon"/>}
        <div><h2>{details?.project.title??opened.title}</h2><p>{details?.project.description??opened.description}</p>
          <span className="muted"><DownloadCloud size={14}/> {(details?.project.downloads??opened.downloads).toLocaleString("uk-UA")} завантажень</span></div>
      </div>
      {detailBusy&&<p className="muted">Завантажуємо опис і версії з Modrinth…</p>}
      {details&&<>
        <div className="fre-mod-detail-layout">
          <div className="fre-mod-detail-description"><h3><Info size={17}/> Про мод</h3>
            <p>{details.project.body||details.project.description}</p>
          </div>
          <div className="fre-mod-detail-versions">
            <h3>Версії для {version}</h3>
            <p className="muted">{selectedInstance?.loader??"Fabric"} · {compatibleVersions.length} сумісних версій</p>
            <label className="muted"><input type="checkbox" checked={showAllVersions} onChange={e=>setShowAllVersions(e.target.checked)}/> Показати всі версії</label>
            <div className="fre-mod-version-scroll">{shownVersions.slice(0,70).map(v=>{
              const compatible=compatibleVersions.some(c=>c.id===v.id);
              return <button key={v.id} className={"fre-mod-version"+(chosenVersion===v.id?" selected":"")} disabled={!compatible} onClick={()=>setChosenVersion(v.id)}>
                <span><b>{v.version_number||v.name}</b><small>{v.game_versions.slice(0,4).join(", ")} · {v.loaders.join(", ")}</small></span>
                {chosenVersion===v.id?<CheckCircle2 size={17}/>:null}
              </button>;
            })}</div>
            {!compatibleVersions.length&&<p className="muted">Немає сумісної версії. Можеш переглянути інші, але встановлення несумісних заблоковано.</p>}
            <button className="primary" disabled={busy||!chosenVersion||!selectedInstance||!["mod","resourcepack"].includes(opened.project_type)||(opened.project_type==="mod"&&selectedInstance.loader!=="fabric")} onClick={()=>void installSelected()}>
              <DownloadCloud size={17}/> {busy?"Встановлення…":"Встановити у збірку"}
            </button>
          </div>
        </div>
      </>}
    </section>}
    {!opened&&<div className="mod-grid">
      {hits.map(hit=><article className="mod-card" key={hit.project_id}>
        {hit.icon_url ? <img className="mod-icon" src={hit.icon_url} alt=""/> : <span className="mod-icon fallback"><Package size={24}/></span>}
        <div className="mod-copy"><button className="fre-mod-open" onClick={()=>void openProject(hit)}><h3>{hit.title}</h3></button><p>{hit.description}</p><small><DownloadCloud size={13}/>{hit.downloads.toLocaleString("uk-UA")} завантажень</small></div>
        <button className="secondary" onClick={()=>void openProject(hit)}>Детальніше</button>
        {(kind==="mod"||kind==="resourcepack")
          ? <button className="secondary" disabled={busy || !selectedInstance || (kind==="mod"&&selectedInstance.loader!=="fabric")} title={kind==="mod"&&selectedInstance?.loader!=="fabric"?"Потрібна Fabric-збірка":"Встановити"} onClick={()=>void install(hit)}>{busy?"...":"Встановити"}</button>
          : <span className="muted">Перегляд</span>}
      </article>)}
    </div>
    {!opened&&<div className="toolbar" style={{justifyContent:"space-between",marginTop:12}}>
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
    <div className="notice"><ShieldCheck size={18}/>
      <span><strong>FRE SAFE DOWNLOAD</strong> · Завантаження тільки з офіційного CDN Modrinth через HTTPS, перевірка SHA-512, розміру, формату JAR та сумісності Minecraft/Fabric. Залежності перевіряються перед встановленням. Це перевірка цілісності, а не антивірус: навіть справжній мод може містити шкідливий код.</span>
    </div>
    <p className="muted">Не встановлюй випадкові JAR з Google. Якщо мод відсутній у Modrinth, його не можна автоматично підтвердити через цей каталог. Моди — сторонній виконуваний код; перед запуском перевіряй автора й репутацію проєкту.</p>
  </section>;
}
