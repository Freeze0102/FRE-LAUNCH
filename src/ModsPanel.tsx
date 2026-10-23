import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Package, DownloadCloud, Search, ShieldCheck, Trash2, RefreshCw, ArrowLeft, CheckCircle2, Info, ExternalLink } from "lucide-react";
import type { Instance } from "./Features";

type ModrinthHit = {
  project_id: string; slug: string; title: string; description: string;
  icon_url: string | null; downloads: number; project_type: string;
};
type ModVersion = {id:string;name:string;version_number:string;game_versions:string[];loaders:string[];date_published:string};
type ModDetails = {project:{title:string;body:string;description:string;icon_url:string|null;downloads:number;project_type:string;categories:string[]};versions:ModVersion[]};
type InstalledMod = {filename: string; enabled: boolean; bytes: number};
type PackType = "resourcepack" | "shader";
type ModUpdate = {filename:string;projectId:string;versionId:string;versionNumber:string};

export default function ModsPanel({version, instanceId}: {version: string; instanceId: string}) {
  const [query,setQuery]=useState("");
  const [view,setView]=useState<"browse"|"library">("browse");
  const [kind,setKind]=useState("mod");
  const [hits,setHits]=useState<ModrinthHit[]>([]);
  const [onlyCompatible,setOnlyCompatible]=useState(true);
  const [sort,setSort]=useState("relevance");
  const searchSequence=useRef(0);
  const [offset,setOffset]=useState(0);
  const [opened,setOpened]=useState<ModrinthHit|null>(null);
  const [details,setDetails]=useState<ModDetails|null>(null);
  const [detailBusy,setDetailBusy]=useState(false);
  const [chosenVersion,setChosenVersion]=useState("");
  const [showAllVersions,setShowAllVersions]=useState(false);
  const [selectedInstance,setSelectedInstance]=useState<Instance|null>(null);
  const [installed,setInstalled]=useState<InstalledMod[]>([]);
  const [resourcepacks,setResourcepacks]=useState<InstalledMod[]>([]);
  const [shaderpacks,setShaderpacks]=useState<InstalledMod[]>([]);
  const [modUpdates,setModUpdates]=useState<ModUpdate[]>([]);
  const [checkingUpdates,setCheckingUpdates]=useState(false);
  const [loading,setLoading]=useState(false);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState("");
  const [message,setMessage]=useState("");

  async function refreshMods() {
    if (!instanceId) {setSelectedInstance(null);setInstalled([]);setResourcepacks([]);setShaderpacks([]);setModUpdates([]);return;}
    try {
      const [profiles,mods,resources,shaders]=await Promise.all([
        invoke<Instance[]>("list_instances"),
        invoke<InstalledMod[]>("list_installed_mods",{instanceId}),
        invoke<InstalledMod[]>("list_installed_packs",{instanceId,kind:"resourcepack"}),
        invoke<InstalledMod[]>("list_installed_packs",{instanceId,kind:"shader"}),
      ]);
      setSelectedInstance(profiles.find(p=>p.id===instanceId)??null);
      setInstalled(mods);setResourcepacks(resources);setShaderpacks(shaders);
    } catch(err){setError(String(err));}
  }
  useEffect(()=>{setModUpdates([]);void refreshMods();},[instanceId]);

  async function search(pageOffset=0) {
    const request=++searchSequence.current;
    setLoading(true);setError("");
    try {
      const results=await invoke<ModrinthHit[]>("search_modrinth", {
        query:query.trim(),version:onlyCompatible ? version || null : null,
        projectType:kind,offset:pageOffset,sort,
        loader:onlyCompatible && kind==="mod" && selectedInstance?.loader!=="vanilla"
          ? selectedInstance?.loader || null : null,
      });
      if(request===searchSequence.current){setHits(results);setOffset(pageOffset);}
    } catch(err){
      if(request===searchSequence.current){setError(String(err));setHits([]);}
    } finally{if(request===searchSequence.current)setLoading(false);}
  }
  useEffect(()=>{
    const handle=window.setTimeout(()=>{void search(0);},350);
    return ()=>window.clearTimeout(handle);
  },[query,kind,version,onlyCompatible,selectedInstance?.loader,sort]);

  async function openProject(hit:ModrinthHit, preferredVersionId?:string) {
    setOpened(hit);setDetails(null);setChosenVersion("");setShowAllVersions(false);setDetailBusy(true);setError("");
    try {
      const data=await invoke<ModDetails>("modrinth_project_details",{projectId:hit.project_id});
      setDetails(data);
      const matches=data.versions.filter(v=>v.game_versions.includes(version)&& (kind!=="mod"||v.loaders.includes(selectedInstance?.loader||"fabric")));
      setChosenVersion(matches.find(v=>v.id===preferredVersionId)?.id||matches[0]?.id||"");
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
      } else if(opened.project_type==="resourcepack"||opened.project_type==="shader") {
        const command=opened.project_type==="shader"?"install_shaderpack":"install_resourcepack";
        const response=await invoke<string>(command,{instanceId,projectId:opened.project_id,versionId:chosenVersion});
        setMessage(opened.project_type==="shader"?`${response}. Для роботи шейдерів також потрібен Iris або інший сумісний шейдерний завантажувач.`:response);
        await refreshMods();
      } else {throw Error("Імпорт модпаків поки не підтримується; не встановлюємо збірку частково.");}
    } catch(e){setError(String(e));}finally{setBusy(false);}
  }

  async function install(hit: ModrinthHit) {
    if (!instanceId || !selectedInstance){setError("Спочатку створи та обери Minecraft-збірку.");return;}
    setBusy(true);setError("");setMessage("");
    try {
      if (kind==="resourcepack"||kind==="shader") {
        const command=kind==="shader"?"install_shaderpack":"install_resourcepack";
        const response=await invoke<string>(command,{instanceId,projectId:hit.project_id});
        setMessage(kind==="shader"?`${response}. Для шейдерів потрібен Iris/інший сумісний завантажувач.`:response);
        await refreshMods();
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

  async function checkUpdates(){
    if(!instanceId)return;
    setCheckingUpdates(true);setError("");setMessage("");
    try{
      const found=await invoke<ModUpdate[]>("check_mod_updates",{instanceId});
      setModUpdates(found);
      setMessage(found.length?`Знайдено ${found.length} доступних оновлень. Перед встановленням нового JAR вимкни старий, щоб не виник конфлікт версій.`:"Для відомих Modrinth-модів нових сумісних оновлень не знайдено.");
    }catch(err){setError(String(err));}
    finally{setCheckingUpdates(false);}
  }
  async function browseUpdate(update:ModUpdate){
    setView("browse");setKind("mod");
    setMessage("Перед встановленням нової версії вимкни старий JAR у «Моя бібліотека». Лаунчер не видаляє старі моди автоматично.");
    const hit:ModrinthHit={project_id:update.projectId,slug:"",title:update.filename,
      description:"Перевірене через Modrinth оновлення",icon_url:null,downloads:0,project_type:"mod"};
    await openProject(hit,update.versionId);
  }
  async function togglePack(pack:InstalledMod, packKind:PackType){
    setBusy(true);setError("");setMessage("");
    try{
      await invoke("toggle_installed_pack",{instanceId,kind:packKind,filename:pack.filename});
      await refreshMods();
    }catch(err){setError(String(err));}
    finally{setBusy(false);}
  }
  async function removePack(pack:InstalledMod, packKind:PackType){
    if(!window.confirm(`Видалити «${pack.filename}» з цієї збірки?`))return;
    setBusy(true);setError("");setMessage("");
    try{
      await invoke("remove_installed_pack",{instanceId,kind:packKind,filename:pack.filename});
      await refreshMods();
    }catch(err){setError(String(err));}
    finally{setBusy(false);}
  }

  return <section className="panel">
    <div className="panel-heading"><h3>FRE MODRINTH</h3><Package size={20}/></div>
    <p>Каталог Modrinth: моди Fabric, ресурс-паки та шейдери встановлюються в окрему збірку з перевіркою файлів. Модпаки поки доступні для перегляду.</p>
    <nav className="fre-content-tabs" aria-label="Каталог та встановлені ресурси">
      <button className={view==="browse"?"active":""} onClick={()=>setView("browse")}>Шукати контент</button>
      <button className={view==="library"?"active":""} onClick={()=>setView("library")}>
        Моя бібліотека {instanceId? `(${installed.length+resourcepacks.length+shaderpacks.length})`:""}
      </button>
    </nav>
    {error&&<div className="error">{error}</div>}
    {message&&<div className="notice"><ShieldCheck size={16}/>{message}</div>}
    {view==="browse"&&<>
    <div className="toolbar">
      <div className="search"><Search size={18}/><input placeholder="Знайти Sodium, Lithium, Fabric API..." value={query} onChange={e=>setQuery(e.target.value)} onKeyDown={e=>{if(e.key==="Enter")void search(0);}}/></div>
      <select value={kind} onChange={e=>setKind(e.target.value)}>
        <option value="mod">Моди</option>
        <option value="resourcepack">Ресурс-паки</option>
        <option value="shader">Шейдери</option>
        <option value="modpack">Модпаки</option>
      </select>
      <select value={sort} onChange={e=>setSort(e.target.value)} aria-label="Сортування результатів">
        <option value="relevance">За відповідністю</option>
        <option value="downloads">Найпопулярніші</option>
        <option value="updated">Нещодавно оновлені</option>
        <option value="newest">Нові проєкти</option>
        <option value="follows">За підписками</option>
      </select>
      <button className="primary" onClick={()=>void search(0)} disabled={loading}>{loading?"Шукаю...":"Знайти"}</button>
    </div>
    <div className="toolbar" style={{marginTop:12,alignItems:"center"}}>
      <span className="muted">Джерела:</span>
      <strong>Modrinth · встановлення в лаунчері</strong>
      <button className="secondary" onClick={()=>window.open("https://www.curseforge.com/minecraft/search?search="+encodeURIComponent(query.trim()||"mods"),"_blank","noopener,noreferrer")}>
        CurseForge · знайти більше <ExternalLink size={15}/>
      </button>
    </div>
    <label className="muted" style={{display:"flex",alignItems:"center",gap:8,marginTop:12}}>
      <input type="checkbox" checked={onlyCompatible} onChange={e=>setOnlyCompatible(e.target.checked)}/>
      Автопідбір під Minecraft {version||"обраної версії"}{kind==="mod"&&selectedInstance?.loader&&selectedInstance.loader!=="vanilla"?` · ${selectedInstance.loader}`:""}
    </label>
    <p className="muted">Пошук у всьому каталозі Modrinth. Без галочки показуються також проєкти для інших версій, але встановлення потребує сумісного релізу.</p>
    <p className="muted">{instanceId ? `Збірка: ${selectedInstance?.name??"Обрана"} · ${selectedInstance?.loader??"Vanilla"}` : "Спочатку обери окрему збірку для встановлення."} · Minecraft {version||"—"}</p>

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
            <button className="primary" disabled={busy||!chosenVersion||!selectedInstance||!["mod","resourcepack","shader"].includes(opened.project_type)||(opened.project_type==="mod"&&selectedInstance.loader!=="fabric")} onClick={()=>void installSelected()}>
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
        {(kind==="mod"||kind==="resourcepack"||kind==="shader")
          ? <button className="secondary" disabled={busy || !selectedInstance || (kind==="mod"&&selectedInstance.loader!=="fabric")} title={kind==="mod"&&selectedInstance?.loader!=="fabric"?"Потрібна Fabric-збірка":"Встановити"} onClick={()=>void install(hit)}>{busy?"...":"Встановити"}</button>
          : <span className="muted">Перегляд</span>}
      </article>)}
    </div>}
    {!opened&&<div className="toolbar" style={{justifyContent:"space-between",marginTop:12}}>
      <button className="secondary" disabled={loading||offset===0} onClick={()=>void search(Math.max(0,offset-24))}>← Назад</button>
      <span className="muted">{hits.length ? `Результати ${offset+1}–${offset+hits.length}` : "Нічого не знайдено. Спробуй прибрати фільтр версії."}</span>
      <button className="secondary" disabled={loading||hits.length<24||offset>=984} onClick={()=>void search(offset+24)}>Далі →</button>
    </div>}
    </>}
    {view==="library"&&!instanceId&&<p className="muted">Обери збірку в бібліотеці Minecraft, щоб керувати її встановленими модами й ресурсами.</p>}
    {view==="library"&&!!instanceId&&<div className="installed-mods">
      <div className="panel-heading"><h3>Встановлені моди</h3>
        <div className="fre-content-actions">
          <button className="secondary" disabled={busy||checkingUpdates||selectedInstance?.loader!=="fabric"}
            onClick={()=>void checkUpdates()}>{checkingUpdates?"Перевірка…":"Перевірити оновлення"}</button>
          <button className="icon-button" title="Оновити список файлів" onClick={()=>void refreshMods()}><RefreshCw size={16}/></button>
        </div>
      </div>
      {installed.length===0&&<p className="muted">Немає встановлених модів у цій збірці.</p>}
      {installed.map(mod=><div className="installed-mod-row" key={mod.filename}>
        <div><b>{mod.filename}</b><span>{(mod.bytes/1024/1024).toFixed(1)} MB · {mod.enabled?"Увімкнено":"Вимкнено"}</span>
          {modUpdates.filter(u=>u.filename===mod.filename).map(update=>
            <button className="fre-update-link" key={update.versionId}
              onClick={()=>void browseUpdate(update)}>Доступне оновлення: {update.versionNumber} → Переглянути</button>
          )}
        </div>
        <button className="secondary" disabled={busy} onClick={()=>void toggle(mod)>{mod.enabled?"Вимкнути":"Увімкнути"}</button>
        <button className="icon-button danger" title="Видалити" disabled={busy} onClick={()=>void remove(mod)}><Trash2 size={16}/></button>
      </div>)}
    </div>}
    {view==="library"&&!!instanceId&&([{kind:"resourcepack",title:"Мої ресурс-паки",files:resourcepacks},
      {kind:"shader",title:"Мої шейдери",files:shaderpacks}] as const).map(group=>
      <div className="installed-mods" key={group.kind}>
        <div className="panel-heading"><h3>{group.title}</h3>
          <button className="icon-button" title="Оновити" onClick={()=>void refreshMods()}><RefreshCw size={16}/></button>
        </div>
        {group.files.length===0&&<p className="muted">У цій збірці поки немає файлів.</p>}
        {group.files.map(pack=><div className="installed-mod-row" key={pack.filename}>
          <div><b>{pack.filename}</b><span>{(pack.bytes/1024/1024).toFixed(1)} MB · {pack.enabled?"Увімкнено":"Вимкнено"}</span></div>
          <button className="secondary" disabled={busy} onClick={()=>void togglePack(pack,group.kind)}>{pack.enabled?"Вимкнути":"Увімкнути"}</button>
          <button className="icon-button danger" title="Видалити" disabled={busy} onClick={()=>void removePack(pack,group.kind)}><Trash2 size={16}/></button>
        </div>)}
      </div>
    )}
    {view==="library"&&!!instanceId&&<div className="notice">
      <Info size={16}/> Ресурс-паки вмикаються в налаштуваннях Minecraft. Для шейдерів потрібен Iris або інший сумісний завантажувач.
      <button className="secondary" onClick={()=>{setView("browse");setKind("mod");setQuery("Iris");setOpened(null);}}>Знайти Iris</button>
    </div>}
    <div className="notice"><ShieldCheck size={18}/>
      <span><strong>FRE SAFE DOWNLOAD</strong> · Встановлення з Modrinth використовує HTTPS, SHA-512, обмеження розміру та перевірку ZIP/JAR. Для модів Fabric підтягуються обов’язкові залежності. Лаунчер не є антивірусом і не гарантує відсутності шкідливого Java-коду.</span>
    </div>
    <p className="muted">Власні JAR, завантажені через браузер, можна покласти в папку mods обраної збірки. Вони не отримують позначки перевіреного джерела Modrinth. Моди — сторонній виконуваний код; перед запуском перевіряй автора й репутацію проєкту.</p>
  </section>;
}
