import "./instance-library.css";
import {useCallback,useEffect,useMemo,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {
  ArrowLeft,ArrowRight,Boxes,Check,FolderOpen,
  Layers3,Package,Plus,RefreshCw,Search,Settings2,ShieldCheck,Star,Trash2,
  X,Play,FileText,Archive,Grid2X2,List,Upload
} from "lucide-react";
import FabricOptions from "./FabricOptions";
import GameFolders from "./GameFolders";
import BackupsPanel from "./BackupsPanel";

export type Instance = {
  id:string;name:string;version:string;favorite:boolean;memoryMb:number;
  javaPath:string|null;loader:string;loaderVersion:string|null;
};
type Installed={filename:string;enabled:boolean;bytes:number};
type WorldSummary={name:string;lastModified:number;bytesOnDisk:number};
type Category="mods"|"resourcepacks"|"shaderpacks";
type Tab="content"|"worlds"|"files"|"settings"|"logs";
type Props={
  version:string;active:string;setActive:(id:string)=>void;
  setVersion:(v:string)=>void;setMemory:(mb:number)=>void;
  navigateHome:()=>void;navigateMods:()=>void;navigateVersions:()=>void;onPlayInstance:(item:Instance)=>void;requestedId?:string;
  showWorlds:boolean;showFiles:boolean;createRequest:number;onImportPack:()=>void;importBusy:boolean;
};
const categories:{id:Category;label:string}[]=[
  {id:"mods",label:"Моди"},{id:"resourcepacks",label:"Ресурс-паки"},{id:"shaderpacks",label:"Шейдери"}
];
function loaderLabel(instance:Instance){
  return instance.loader==="vanilla"?"Vanilla":instance.loader==="fabric"?"Fabric":"Quilt";
}
function sizeLabel(bytes:number){return bytes>=1048576?(bytes/1048576).toFixed(1)+" MB":Math.round(bytes/1024)+" KB";}

export function InstancesPanel({version,active,setActive,setVersion,setMemory,navigateHome,navigateMods,navigateVersions,onPlayInstance,showWorlds,showFiles,onImportPack,importBusy,createRequest,requestedId}:Props){
  const [instances,setInstances]=useState<Instance[]>([]);
  const [artById]=useState<Record<string,string>>(()=>{
    try{return JSON.parse(localStorage.getItem("fre-play-instance-art")||"{}") as Record<string,string>;}
    catch{return {};}
  });
  const [search,setSearch]=useState("");
  const [sort,setSort]=useState("name");
  const [layout,setLayout]=useState<"grid"|"list">("grid");
  const [selected,setSelected]=useState<string|null>(null);
  const [tab,setTab]=useState<Tab>("content");
  const [category,setCategory]=useState<Category>("mods");
  const [fileFilter,setFileFilter]=useState("");
  const [files,setFiles]=useState<Installed[]>([]);
  const [worlds,setWorlds]=useState<WorldSummary[]>([]);
  const [worldFilter,setWorldFilter]=useState("");
  const [createOpen,setCreateOpen]=useState(false);
  const [createName,setCreateName]=useState("");
  const [createStep,setCreateStep]=useState<"choose"|"custom">("choose");
  const [createVersion,setCreateVersion]=useState(version);
  const [createLoader,setCreateLoader]=useState<"vanilla"|"fabric"|"quilt">("vanilla");
  const [availableVersions,setAvailableVersions]=useState<string[]>([]);
  const [createError,setCreateError]=useState("");
  const [editName,setEditName]=useState("");
  const [editRam,setEditRam]=useState(4096);
  const [editJava,setEditJava]=useState("");
  const [busy,setBusy]=useState(false);
  const [loadingFiles,setLoadingFiles]=useState(false);
  const [error,setError]=useState("");
  const [notice,setNotice]=useState("");

  useEffect(()=>{if(requestedId)setSelected(requestedId);},[requestedId]);
  useEffect(()=>{if(createRequest>0){setSelected(null);setCreateOpen(true);setCreateStep("choose");setCreateError("");}},[createRequest]);
  useEffect(()=>{if((tab==="worlds"&&!showWorlds)||(tab==="files"&&!showFiles))setTab("content");},[tab,showWorlds,showFiles]);
  const current=instances.find(item=>item.id===selected)||null;
  const filtered=useMemo(()=>{
    const items=instances.filter(item=>(item.name+" "+item.version+" "+item.loader).toLowerCase().includes(search.toLowerCase()));
    if(sort==="version")return items.sort((a,b)=>b.version.localeCompare(a.version));
    if(sort==="favorite")return items.sort((a,b)=>Number(b.favorite)-Number(a.favorite)||a.name.localeCompare(b.name));
    return items.sort((a,b)=>a.name.localeCompare(b.name));
  },[instances,search,sort]);

  const refresh=useCallback(async()=>{
    try{setInstances(await invoke<Instance[]>("list_instances"));setError("");}
    catch(err){setError(String(err));}
  },[]);
  useEffect(()=>{void refresh();},[refresh]);
  useEffect(()=>{
    if(!createOpen)return;
    let mounted=true;
    void invoke<{id:string;type:string}[]>("get_versions")
      .then(results=>{if(mounted)setAvailableVersions(results.filter(item=>item.type==="release").slice(0,40).map(item=>item.id));})
      .catch(()=>{});
    return()=>{mounted=false;};
  },[createOpen]);
  const refreshFiles=useCallback(async()=>{
    if(!selected){setFiles([]);return;}
    setLoadingFiles(true);
    try{
      const command=category==="mods"?"list_installed_mods":"list_installed_packs";
      const result=await invoke<Installed[]>(command,category==="mods"?{instanceId:selected}:{instanceId:selected,kind:category==="shaderpacks"?"shader":"resourcepack"});
      setFiles(result);setError("");
    }catch(e){setError(String(e));}
    finally{setLoadingFiles(false);}
  },[selected,category]);
  useEffect(()=>{if(current&&tab==="content")void refreshFiles();},[current?.id,tab,refreshFiles]);
  useEffect(()=>{
    if(!current||tab!=="worlds")return;
    let active=true;
    void invoke<WorldSummary[]>("list_instance_worlds",{instanceId:current.id})
      .then(items=>{if(active)setWorlds(items);})
      .catch(err=>{if(active)setError(String(err));});
    return()=>{active=false;};
  },[current?.id,tab]);

  function open(item:Instance){
    setSelected(item.id);setTab("content");setCategory("mods");setFileFilter("");setWorldFilter("");
    setEditName(item.name);setEditRam(item.memoryMb);setEditJava(item.javaPath||"");
    setActive(item.id);setVersion(item.version);setMemory(item.memoryMb);
  }
  async function create(){
    const targetVersion=createVersion||version;
    if(!createName.trim()||!targetVersion)return;
    setBusy(true);setCreateError("");setError("");setNotice("");
    try{
      let instance=await invoke<Instance>("create_instance",{name:createName.trim(),version:targetVersion});
      if(createLoader!=="vanilla"){
        try{
          const loaders=await invoke<{version:string;stable:boolean}[]>(createLoader==="fabric"?"list_fabric_loaders":"list_quilt_loaders",{gameVersion:targetVersion});
          const loaderVersion=loaders.find(item=>item.stable)?.version||loaders[0]?.version;
          if(!loaderVersion)throw Error("Для цієї версії Minecraft модлоадер не знайдено.");
          instance=await invoke<Instance>("set_instance_loader",{id:instance.id,loader:createLoader,loaderVersion});
        }catch(e){
          setNotice("Збірку створено, але модлоадер не налаштовано: "+String(e)+". Налаштуй його на сторінці збірки.");
        }
      }
      setCreateOpen(false);setCreateName("");setCreateStep("choose");
      setCreateLoader("vanilla");
      await refresh();open(instance);
    }catch(err){setCreateError(String(err));}
    finally{setBusy(false);}
  }
  async function favorite(item:Instance){
    try{
      await invoke("update_instance",{id:item.id,name:item.name,favorite:!item.favorite,memoryMb:item.memoryMb,javaPath:item.javaPath});
      await refresh();
    }catch(err){setError(String(err));}
  }
  async function remove(item:Instance){
    if(!window.confirm(`Видалити «${item.name}» з бібліотеки? Файли гри й світи залишаться на диску.`))return;
    setBusy(true);
    try{
      await invoke("delete_instance",{id:item.id,deleteFiles:false});
      if(item.id===active)setActive("");
      setSelected(null);await refresh();
    }catch(err){setError(String(err));}
    finally{setBusy(false);}
  }
  async function save(item:Instance){
    setBusy(true);setError("");setNotice("");
    try{
      await invoke("update_instance",{
        id:item.id,name:editName.trim(),favorite:item.favorite,
        memoryMb:editRam,javaPath:editJava.trim()||null
      });
      if(active===item.id)setMemory(editRam);
      await refresh();setNotice("Налаштування збірки збережено.");
    }catch(err){setError(String(err));}
    finally{setBusy(false);}
  }
  async function setEnabled(item:Installed){
    if(!selected)return;
    setBusy(true);setError("");
    try{
      if(category==="mods")await invoke("toggle_installed_mod",{instanceId:selected,filename:item.filename});
      else await invoke("toggle_installed_pack",{instanceId:selected,kind:category==="shaderpacks"?"shader":"resourcepack",filename:item.filename});
      await refreshFiles();
    }catch(err){setError(String(err));}
    finally{setBusy(false);}
  }
  async function removeFile(item:Installed){
    if(!selected||!window.confirm(`Видалити файл «${item.filename}» із цієї збірки? Цю дію не можна скасувати.`))return;
    setBusy(true);setError("");
    try{
      if(category==="mods")await invoke("remove_installed_mod",{instanceId:selected,filename:item.filename});
      else await invoke("remove_installed_pack",{instanceId:selected,kind:category==="shaderpacks"?"shader":"resourcepack",filename:item.filename});
      await refreshFiles();
    }catch(err){setError(String(err));}
    finally{setBusy(false);}
  }
  async function openFolder(item:Instance,kind:string){
    try{await invoke("open_game_folder",{instanceId:item.id,version:item.version,kind});}
    catch(err){setError(String(err));}
  }
  return <section className="fre-library-page">
    {error&&<div className="error" role="alert">{error}</div>}
    {notice&&<div className="notice" role="status"><ShieldCheck size={16}/>{notice}</div>}
    {!current?<>

      <div className="fre-library-heading">
        <div><h2>Бібліотека</h2><p>Усі твої профілі Minecraft в одному місці.</p></div>
        <div className="fre-library-new-actions">
          <button className="secondary" disabled={importBusy} onClick={onImportPack}><Upload size={17}/> Імпорт .mrpack</button>
          <button className="primary" onClick={()=>setCreateOpen(true)}><Plus size={17}/> Створити збірку</button>
        </div>
      </div>
      <div className="fre-library-toolbar">
        <div className="search"><Search size={16}/><input aria-label="Пошук збірки" placeholder="Пошук збірок..." value={search} onChange={e=>setSearch(e.target.value)}/></div>
        <select value={sort} onChange={e=>setSort(e.target.value)} aria-label="Сортувати збірки">
          <option value="name">Назва</option><option value="version">Версія</option><option value="favorite">Обрані</option>
        </select>
        <button className={layout==="grid"?"active":""} onClick={()=>setLayout("grid")} title="Плитки" aria-label="Плитки"><Grid2X2 size={18}/></button>
        <button className={layout==="list"?"active":""} onClick={()=>setLayout("list")} title="Список" aria-label="Список"><List size={18}/></button>
        <button onClick={()=>void refresh()} title="Оновити" aria-label="Оновити"><RefreshCw size={18}/></button>
      </div>
      {!filtered.length&&<div className="fre-library-empty"><Boxes size={34}/><strong>{instances.length?"Нічого не знайдено":"Тут поки немає збірок"}</strong><span>Створи профіль і налаштуй Minecraft під себе.</span><button className="secondary" onClick={()=>setCreateOpen(true)}><Plus size={16}/> Створити</button></div>}
      <div className={"fre-library-cards"+(layout==="list"?" is-list":"")}>
        {filtered.map(item=><article className={"fre-library-card"+(active===item.id?" is-chosen":"")} key={item.id}>
          <button className="fre-library-card-main" onClick={()=>open(item)} aria-label={`Відкрити ${item.name}`}>
            <span className={"fre-library-card-art fre-loader-"+item.loader}><Boxes size={42}/>{artById[item.id]&&<img src={artById[item.id]} alt="" className="fre-library-art-img" loading="lazy" onError={e=>{e.currentTarget.style.display="none";}}/>}</span>
            <span className="fre-library-card-info"><strong title={item.name}>{item.name}</strong><small>{item.version} · {loaderLabel(item)}</small><em>{item.memoryMb/1024} GB RAM</em></span>
          </button>
          <div className="fre-library-card-actions">
            <button onClick={()=>open(item)}><FolderOpen size={15}/> Відкрити</button>
            <button className={item.favorite?"is-starred":""} onClick={()=>void favorite(item)} aria-label={item.favorite?"Прибрати з обраного":"В обране"}><Star size={16} fill={item.favorite?"currentColor":"none"}/></button>
          </div>
        </article>)}
      </div>
      {createOpen&&<div className="fre-library-modal-backdrop" role="presentation">
        <div className="fre-library-modal fre-create-wizard" role="dialog" aria-modal="true" aria-label="Створити збірку">
          <header><h2>Створити збірку</h2><button onClick={()=>{setCreateOpen(false);setCreateStep("choose");}} aria-label="Закрити"><X size={19}/></button></header>
          {createStep==="choose"?<>
            <p>Уже знаєш, у що хочеш пограти?</p>
            <button className="fre-wizard-search" onClick={()=>{setCreateOpen(false);navigateMods();}}><Search size={18}/> Шукати моди й модпаки у Modrinth <ArrowRight size={16}/></button>
            <h3>Вибери спосіб створення</h3>
            <button className="fre-wizard-option" onClick={()=>{setCreateStep("custom");setCreateVersion(version);setCreateError("");}}>
              <span><Layers3 size={20}/></span><span><b>Власна збірка</b><small>Створи збірку з потрібною версією Minecraft і модлоадером.</small></span><ArrowRight size={17}/></button>
            <button className="fre-wizard-option" onClick={()=>{setCreateOpen(false);navigateMods();}}>
              <span><Package size={20}/></span><span><b>Знайти мод або модпак</b><small>Переглянути каталог та встановити сумісний модпак.</small></span><ArrowRight size={17}/></button>
            <button className="fre-wizard-option" disabled={importBusy} onClick={()=>{setCreateOpen(false);onImportPack();}}>
              <span><Upload size={20}/></span><span><b>Імпорт .mrpack з комп'ютера</b>
              <small>Вибери готовий архів збірки Modrinth. Створимо новий профіль.</small></span><ArrowRight size={17}/>
            </button>
            <div className="fre-wizard-soon"><ShieldCheck size={16}/> Forge/NeoForge наразі не підтримуються. Імпорт перевіряє файли й хеші.</div>
          </>:<>
            <button className="fre-wizard-back" onClick={()=>setCreateStep("choose")}><ArrowLeft size={15}/> Вибір типу</button>
            <label>Назва</label><input className="input" autoFocus maxLength={48} placeholder="Моя збірка" value={createName} onChange={e=>setCreateName(e.target.value)} onKeyDown={e=>{if(e.key==="Enter")void create();}}/>
            <label>Версія Minecraft</label>
            <select className="fre-wizard-version" value={createVersion} onChange={e=>setCreateVersion(e.target.value)}>
              {[...new Set([createVersion,version,...availableVersions].filter(Boolean))].map(item=><option key={item} value={item}>{item}</option>)}
            </select>
            <label>Модлоадер</label>
            <div className="fre-wizard-loaders">
              {(["vanilla","fabric","quilt"] as const).map(loader=><button key={loader} onClick={()=>setCreateLoader(loader)} className={createLoader===loader?"selected":""}>
                {loader==="vanilla"?"Vanilla":loader==="fabric"?"Fabric":"Quilt"}{createLoader===loader&&<Check size={15}/>}
              </button>)}
            </div>
            <p>Після створення профіль отримає власні папки для світів, модів, ресурс-паків і налаштувань.</p>
            {createError&&<p className="error">{createError}</p>}
            <footer><button className="secondary" onClick={()=>setCreateStep("choose")}>Назад</button><button className="primary" disabled={!createName.trim()||!createVersion||busy} onClick={()=>void create()}><Plus size={16}/> {busy?"Створення…":"Створити профіль"}</button></footer>
          </>}
        </div>
      </div>}
    </>:<>
      <div className="fre-instance-detail">
        <button className="fre-instance-back" onClick={()=>setSelected(null)}><ArrowLeft size={16}/> Бібліотека</button>
        <header className="fre-instance-detail-head">
          <span className={"fre-instance-detail-cover fre-loader-"+current.loader}><Boxes size={38}/>{artById[current.id]&&<img src={artById[current.id]} alt="" className="fre-library-art-img" onError={e=>{e.currentTarget.style.display="none";}}/>}</span>
          <div><h2>{current.name}</h2><p><Layers3 size={14}/> {loaderLabel(current)} {current.version} <span className="fre-instance-head-dot">•</span> {current.memoryMb/1024} GB RAM</p></div>
          <button className="fre-instance-head-settings" onClick={()=>setTab("settings")} title="Налаштування збірки" aria-label="Налаштування збірки"><Settings2 size={21}/></button>
          <button className="fre-instance-head-favorite" onClick={()=>void favorite(current)} title={current.favorite?"Прибрати з обраного":"У вибране"}><Star size={17} fill={current.favorite?"currentColor":"none"}/></button>
          <button className="fre-instance-head-play" onClick={()=>onPlayInstance(current)}><Play size={20} fill="currentColor"/> Грати</button>
        </header>
        <nav className="fre-instance-tabs" aria-label="Налаштування збірки">
          {([
            ["content","Вміст",Package],["files","Файли",FolderOpen],["worlds","Світи",Archive],
            ["logs","Журнали",FileText],["settings","Налаштування",Settings2]
          ] as const).filter(([key])=>(key!=="worlds"||showWorlds)&&(key!=="files"||showFiles)).map(([key,label,Icon])=><button key={key} className={tab===key?"active":""} onClick={()=>setTab(key)}><Icon size={16}/>{label}</button>)}
        </nav>
        {tab==="content"&&<div className="fre-instance-content">
          <div className="fre-instance-content-head"><h3>Додатковий контент</h3><div>
            <button className="secondary" onClick={()=>void openFolder(current,category==="mods"?"mods":category)}><FolderOpen size={15}/> Завантажити файли</button>
            <button className="fre-instance-browse-content" onClick={navigateMods}><Plus size={15}/> Пошук контенту</button>
            <button className="secondary" title="Оновити список" aria-label="Оновити список" onClick={()=>void refreshFiles()} disabled={loadingFiles}><RefreshCw size={16}/></button>
          </div></div>
          <div className="fre-instance-content-filters">{categories.map(x=><button className={category===x.id?"active":""} onClick={()=>setCategory(x.id)} key={x.id}>{x.label}</button>)}</div>
          <div className="fre-instance-content-search"><Search size={16}/><input value={fileFilter} onChange={e=>setFileFilter(e.target.value)} placeholder="Шукати встановлені файли..."/></div>
          <div className="fre-instance-content-table">
            <div className="fre-instance-content-columns"><span>Назва</span><span>Розмір</span><span>Стан</span><span>Дії</span></div>
            {files.filter(file=>file.filename.toLowerCase().includes(fileFilter.toLowerCase())).map(file=><div className="fre-instance-file" key={file.filename}>
              <div><span className="fre-instance-file-icon"><Package size={19}/></span><span><strong title={file.filename}>{file.filename}</strong><small>{category==="mods"?"JAR-мод":category==="resourcepacks"?"Ресурс-пак":"Шейдер"}</small></span></div>
              <span>{sizeLabel(file.bytes)}</span>
              <label className="fre-instance-toggle" title={file.enabled?"Вимкнути":"Увімкнути"}>
                <input type="checkbox" checked={file.enabled} disabled={busy} onChange={()=>void setEnabled(file)}/><span aria-hidden="true"/>
              </label>
              <button title="Видалити файл" aria-label={"Видалити "+file.filename} disabled={busy} onClick={()=>void removeFile(file)}><Trash2 size={17}/></button>
            </div>)}
            {!loadingFiles&&!files.some(x=>x.filename.toLowerCase().includes(fileFilter.toLowerCase()))&&<p className="fre-instance-empty-content">Файлів у цій категорії не знайдено. Натисни «Додати контент» або відкрий папку гри.</p>}
            {loadingFiles&&<p className="fre-instance-empty-content">Завантажуємо список файлів…</p>}
          </div>
          <button className="fre-instance-openfolder" onClick={()=>void openFolder(current,category==="mods"?"mods":category)}><FolderOpen size={15}/> Відкрити папку {category==="mods"?"mods":category==="resourcepacks"?"resourcepacks":"shaderpacks"} у Windows <ArrowRight size={14}/></button>
        </div>}
        {tab==="worlds"&&<div className="fre-instance-tab-body">
          <div className="fre-instance-content-head"><h3>Світи Minecraft</h3><button className="secondary" onClick={()=>void openFolder(current,"saves")}><FolderOpen size={16}/> Папка світів</button></div>
          <div className="fre-instance-content-search"><Search size={16}/><input value={worldFilter} onChange={e=>setWorldFilter(e.target.value)} placeholder="Пошук світів..."/></div>
          <div className="fre-instance-world-list">
            {worlds.filter(world=>world.name.toLowerCase().includes(worldFilter.toLowerCase())).map(world=><div className="fre-instance-world" key={world.name}>
              <div className="fre-instance-world-art"><Layers3 size={25}/></div>
              <div><strong>{world.name}</strong><small>{world.lastModified?new Date(world.lastModified*1000).toLocaleString("uk-UA"):"Час невідомий"} · Minecraft {current.version}</small></div>
              <button className="secondary" onClick={()=>void openFolder(current,"saves")}><FolderOpen size={15}/> Файли</button>
            </div>)}
            {!worlds.some(world=>world.name.toLowerCase().includes(worldFilter.toLowerCase()))&&<p className="fre-instance-empty-content">Світи не знайдено. Створи світ у Minecraft або відкрий папку saves.</p>}
          </div>
          <BackupsPanel instanceId={current.id} instanceName={current.name}/>
        </div>}
        {tab==="files"&&<div className="fre-instance-tab-body"><GameFolders instanceId={current.id} version={current.version}/></div>}
        {tab==="logs"&&<div className="fre-instance-tab-body"><h3>Журнали Minecraft</h3><p>Файли latest.log і fre-launch-game.log допоможуть знайти причину вильоту гри.</p><button className="secondary" onClick={()=>void openFolder(current,"logs")}><FolderOpen size={16}/> Відкрити папку журналів</button></div>}
        {tab==="settings"&&<div className="fre-instance-tab-body fre-instance-settings">
          <h3>Налаштування профілю</h3><label>Назва збірки</label><input className="input" maxLength={48} value={editName} onChange={e=>setEditName(e.target.value)}/>
          <label>Оперативна пам'ять: {(editRam/1024).toFixed(1)} GB</label><input type="range" min={1024} max={16384} step={512} value={editRam} onChange={e=>setEditRam(Number(e.target.value))}/>
          <label>Java — шлях до java.exe (необов'язково)</label><input className="input" value={editJava} onChange={e=>setEditJava(e.target.value)} placeholder="Автовибір Java"/>
          <button className="primary" disabled={busy||!editName.trim()} onClick={()=>void save(current)}><Check size={16}/> Зберегти</button>
          <div className="fre-instance-loader"><h3>Mod loader</h3><FabricOptions key={current.id+"-"+current.loader+"-"+(current.loaderVersion||"")} instance={current} onSaved={refresh}/></div>
          <div className="fre-instance-danger"><p>Видалення зі списку не видаляє файли збірки. Світи залишаться в AppData.</p><button className="secondary" onClick={()=>void remove(current)}><Trash2 size={16}/> Прибрати з бібліотеки</button></div>
        </div>}
      </div>
    </>}
  </section>;
}
