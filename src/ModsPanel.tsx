import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize from "rehype-sanitize";
import { Package, DownloadCloud, Search, ShieldCheck, Trash2, RefreshCw, ArrowLeft, CheckCircle2, Info, ExternalLink, Library, Filter, X, Server } from "lucide-react";
import type { Instance } from "./Features";
import "./discover-page.css";

type ModrinthHit = {
  project_id: string; slug: string; title: string; description: string;
  icon_url: string | null; downloads: number; project_type: string; author?:string; categories?:string[];
};
type ModVersion = {id:string;name:string;version_number:string;game_versions:string[];loaders:string[];date_published:string};
type ModDetails = {project:{title:string;body:string;description:string;icon_url:string|null;downloads:number;project_type:string;categories:string[]};versions:ModVersion[]};
type InstalledMod = {filename: string; enabled: boolean; bytes: number};
type PackType = "resourcepack" | "shader";
type ModUpdate = {filename:string;projectId:string;versionId:string;versionNumber:string};
const categoryLists:Record<string,{id:string;label:string}[]>={
  modpack:[
    {id:"adventure",label:"Пригоди"},{id:"challenging",label:"Складні"},{id:"combat",label:"Бої"},
    {id:"kitchen-sink",label:"Усе разом"},{id:"lightweight",label:"Легкі"},{id:"magic",label:"Магія"},
    {id:"multiplayer",label:"Мультиплеєр"},{id:"optimization",label:"Оптимізація"},
    {id:"quests",label:"Квести"},{id:"technology",label:"Технології"}],
  mod:[
    {id:"adventure",label:"Пригоди"},{id:"decoration",label:"Декорації"},{id:"economy",label:"Економіка"},
    {id:"equipment",label:"Спорядження"},{id:"food",label:"Їжа"},{id:"game-mechanics",label:"Механіки"},
    {id:"library",label:"Бібліотеки"},{id:"magic",label:"Магія"},{id:"mobs",label:"Моби"},
    {id:"optimization",label:"Оптимізація"},{id:"storage",label:"Сховище"},
    {id:"technology",label:"Технології"},{id:"utility",label:"Утиліти"},{id:"worldgen",label:"Генерація світу"}],
  resourcepack:[
    {id:"16x",label:"16x"},{id:"32x",label:"32x"},{id:"64x",label:"64x"},
    {id:"realistic",label:"Реалізм"},{id:"simplistic",label:"Мінімалізм"},
    {id:"themed",label:"Тематичні"},{id:"vanilla-like",label:"Стандартний стиль"}],
  shader:[{id:"realistic",label:"Реалізм"},{id:"fantasy",label:"Фентезі"},{id:"vanilla-like",label:"Vanilla+"}],
  datapack:[{id:"adventure",label:"Пригоди"},{id:"game-mechanics",label:"Механіки"},{id:"mobs",label:"Моби"},
    {id:"utility",label:"Утиліти"},{id:"worldgen",label:"Генерація світу"}]
};

export default function ModsPanel({version, instanceId, initialProject, onServers}: {version: string; instanceId: string; initialProject?: ModrinthHit | null; onServers?:()=>void}) {
  const [query,setQuery]=useState("");
  const [view,setView]=useState<"browse"|"library">("browse");
  const [kind,setKind]=useState("modpack");
  const [categoryFilter,setCategoryFilter]=useState("");
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
  const [detailTab,setDetailTab]=useState<"about"|"versions">("about");
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
        projectType:kind,offset:pageOffset,sort,category:categoryFilter||null,
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
  },[query,kind,categoryFilter,version,onlyCompatible,selectedInstance?.loader,sort]);

  async function openProject(hit:ModrinthHit, preferredVersionId?:string) {
    setOpened(hit);setDetails(null);setChosenVersion("");setShowAllVersions(false);setDetailTab("about");setDetailBusy(true);setError("");
    try {
      const data=await invoke<ModDetails>("modrinth_project_details",{projectId:hit.project_id});
      setDetails(data);
      const matches=data.versions.filter(v=>v.game_versions.includes(version)&& (hit.project_type!=="mod"||v.loaders.includes(selectedInstance?.loader||"fabric")));
      setChosenVersion(matches.find(v=>v.id===preferredVersionId)?.id||matches[0]?.id||"");
    } catch(e){setError(String(e));}
    finally{setDetailBusy(false);}
  }
  useEffect(()=>{
    if(!initialProject)return;
    setView("browse");
    setKind(initialProject.project_type);
    void openProject(initialProject);
    // Navigating to a different home discovery card changes the project ID.
  },[initialProject?.project_id]);
    const compatibleVersions=details?.versions.filter(v=>v.game_versions.includes(version)&&
    (opened?.project_type!=="mod"||v.loaders.includes(selectedInstance?.loader||"fabric")))??[];
  const shownVersions=showAllVersions?details?.versions??[]:compatibleVersions;
  async function installSelected() {
    if(!opened||!selectedInstance||!instanceId){setError("Спочатку обери збірку.");return;}
    if(!chosenVersion){setError("Немає сумісної версії для обраної збірки.");return;}
    setBusy(true);setError("");setMessage("");
    try {
      if(opened.project_type==="mod") {
        if(selectedInstance.loader!=="fabric")throw Error("Зараз автоматичне встановлення модів підтримує Fabric. Обери Fabric-збірку.");
        const update=modUpdates.find(u=>u.projectId===opened.project_id && u.versionId===chosenVersion);
        if(update && installed.some(m=>m.filename===update.filename && m.enabled)){
          throw Error(`Спочатку вимкни старий мод «${update.filename}» у вкладці «Моя бібліотека», потім встановлюй новий.`);
        }
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

  return <section className="panel fre-modrinth-page">
    <div className="fre-discover-heading">
      <div><h2>{view==="library"?"Встановлений контент":opened?"Проєкт Modrinth":"Пошук контенту"}</h2>
        <p>{view==="library"?"Файли активної збірки":"Моди, модпаки, ресурс-паки, датапаки та шейдери з Modrinth"}</p></div>
      <button className="fre-discover-installed" onClick={()=>{setView(view==="library"?"browse":"library");setOpened(null);}}>
        <Library size={16}/> {view==="library"?"Пошук контенту":`Встановлено ${instanceId?"("+(installed.length+resourcepacks.length+shaderpacks.length)+")":""}`}
      </button>
    </div>
    {error&&<div className="error">{error}</div>}
    {message&&<div className="notice"><ShieldCheck size={16}/>{message}</div>}
    {view==="browse"&&<>
    {!opened&&<nav className="fre-discover-categories" aria-label="Категорії Modrinth">
      {[{id:"modpack",label:"Модпаки"},{id:"mod",label:"Моди"},{id:"resourcepack",label:"Ресурс-паки"},{id:"datapack",label:"Датапаки"},{id:"shader",label:"Шейдери"}].map(category=>
        <button key={category.id} className={kind===category.id?"active":""} onClick={()=>{setKind(category.id);setCategoryFilter("");}}>{category.label}</button>)}
      {onServers&&<button onClick={onServers}><Server size={13}/> Сервери</button>}
    </nav>}
    {!opened&&<div className="toolbar fre-discover-search">
      <div className="search"><Search size={18}/><input placeholder="Шукати контент..." value={query} onChange={e=>setQuery(e.target.value)} onKeyDown={e=>{if(e.key==="Enter")void search(0);}}/></div>
      <button className="primary" onClick={()=>void search(0)} disabled={loading}>{loading?"Шукаю...":"Знайти"}</button>
    </div>}
    {!opened&&<div className="fre-discover-filters">
      <select value={sort} onChange={e=>setSort(e.target.value)} aria-label="Сортування результатів">
        <option value="relevance">За відповідністю</option>
        <option value="downloads">Найпопулярніші</option>
        <option value="updated">Нещодавно оновлені</option>
        <option value="newest">Нові проєкти</option>
        <option value="follows">За підписками</option>
      </select>
      <label className="fre-filter-chip"><input type="checkbox" checked={onlyCompatible} onChange={e=>setOnlyCompatible(e.target.checked)}/> {onlyCompatible?"Сумісні з "+(version||"Minecraft"):"Усі версії Minecraft"}</label>
      <span className="fre-discover-filter-count">{loading?"Пошук…":hits.length+" результатів"}</span>
      <button className="fre-discover-link" onClick={()=>window.open("https://www.curseforge.com/minecraft/search?search="+encodeURIComponent(query.trim()||"mods"),"_blank","noopener,noreferrer")}>CurseForge <ExternalLink size={13}/></button>
    </div>}
    {!opened&&!instanceId&&<p className="muted">Щоб установити проєкт, спершу обери збірку в бібліотеці.</p>}
    {opened&&<section className="fre-mod-detail">
      <button className="secondary" onClick={()=>{setOpened(null);setDetails(null);setError("");}}><ArrowLeft size={16}/> До каталогу</button>
      <div className="fre-mod-detail-hero">
        {opened.icon_url&&<img src={opened.icon_url} alt="" className="fre-mod-detail-icon"/>}
        <div><h2>{details?.project.title??opened.title}</h2><p>{details?.project.description??opened.description}</p>
          <span className="muted"><DownloadCloud size={14}/> {(details?.project.downloads??opened.downloads).toLocaleString("uk-UA")} завантажень · {opened.project_type==="modpack"?"Модпак":opened.project_type==="resourcepack"?"Ресурс-пак":opened.project_type==="datapack"?"Датапак":opened.project_type==="shader"?"Шейдер":"Мод"}</span></div>
      </div>
      {detailBusy&&<p className="muted">Завантажуємо опис і версії з Modrinth…</p>}
      {details&&<>
        <nav className="fre-mod-detail-tabs" aria-label="Перегляд проєкту">
          <button className={detailTab==="about"?"active":""} onClick={()=>setDetailTab("about")}>Опис</button>
          <button className={detailTab==="versions"?"active":""} onClick={()=>setDetailTab("versions")}>Версії ({details.versions.length})</button>
        </nav>
        <div className="fre-mod-detail-layout">
          <div className="fre-mod-detail-description">
            {detailTab==="about"?<><h3><Info size={17}/> Про проєкт</h3><div className="fre-prose"><ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize]} components={{a:({node: _node,...props})=><a {...props} target="_blank" rel="noopener noreferrer"/>}}>{details.project.body||details.project.description}</ReactMarkdown></div></>:
              <><h3>Вибір версії</h3><p>Вибери праворуч версію, сумісну з Minecraft та модлоадером твоєї збірки.</p></>}
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
        <div className="mod-copy"><button className="fre-mod-open" onClick={()=>void openProject(hit)}><h3>{hit.title}</h3></button>{hit.author&&<div className="fre-mod-author">від {hit.author}</div>}<p>{hit.description}</p><small><DownloadCloud size={13}/>{hit.downloads.toLocaleString("uk-UA")} завантажень {hit.categories?.slice(0,2).map(cat=><span className="fre-mod-tag" key={cat}>{cat}</span>)}</small></div>
        <div className="fre-mod-card-actions">
          <button className="secondary" onClick={()=>void openProject(hit)}>Відкрити</button>
          {(kind==="mod"||kind==="resourcepack"||kind==="shader")
            ? <button className="secondary" disabled={busy || !selectedInstance || (kind==="mod"&&selectedInstance.loader!=="fabric")} title={kind==="mod"&&selectedInstance?.loader!=="fabric"?"Потрібна Fabric-збірка":"Встановити"} onClick={()=>void install(hit)}>{busy?"...":"Встановити"}</button>
            : <span className="muted">Перегляд</span>}
        </div>
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
        <button className="secondary" disabled={busy} onClick={()=>void toggle(mod)}>{mod.enabled?"Вимкнути":"Увімкнути"}</button>
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
