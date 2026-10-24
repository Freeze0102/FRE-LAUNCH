import {useEffect,useMemo,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {
  ArrowRight,BookOpen,Check,ChevronDown,Clock3,Compass,Download,
  FolderOpen,Grid2X2,Layers3,List,MoreHorizontal,Package,Palette,
  Play,Plus,Search,Settings2,ShieldCheck,SlidersHorizontal,Trash2,Upload,UserRound,X
} from "lucide-react";
import "./play-page.css";

export type DiscoverHit={project_id:string;title:string;description:string;icon_url:string|null;downloads:number;author?:string;project_type:string;slug?:string};
type Instance={id:string;name:string;version:string;loader:string;memoryMb:number;favorite:boolean;loaderVersion:string|null};
type Release={id:string;type:string;releaseTime:string};
type Destination="Versions"|"Instances"|"Mods"|"Servers"|"Downloads"|"Settings"|"Accounts";
type Props={
  versions:Release[];loading:boolean;selected:string;selectedKind:string;
  installed:boolean;isBusy:boolean;installAction:string;activeInstanceId:string;
  memoryMb:number;nickname:string;java:string;recoveryMode:boolean;
  microsoftConnected:boolean;
  onPage:(destination:Destination)=>void;onLaunch:()=>void;onVersion:(id:string)=>void;
  onSelectInstance:(id:string,version:string,memory:number)=>void;onRefresh:()=>void;
  onOpenProject:(project:DiscoverHit)=>void;
  onLaunchInstance:(instance:{id:string;version:string;memoryMb:number})=>void;
  onOpenInstance:(id:string,version:string,memory:number)=>void;
  onCreateInstance:()=>void;
  onImportPack:()=>void;importBusy:boolean;
};
type IconSpec={symbol:string;color:string};
type Group={id:string;name:string};
const COLOR_OPTIONS=["#26a269","#4489df","#b04ce2","#e78a3c","#e04976","#5362e4","#9ba94b","#434954"];
const ICON_OPTIONS=["⛏","🧱","🌿","⚔","✨","🌋","❄","🐉","🎮","☄","🌈","🔮"];
function readJson<T>(key:string,fallback:T):T{
  try{const v=localStorage.getItem(key);return v?JSON.parse(v) as T:fallback;}catch{return fallback;}
}
function prettyLoader(item:Instance){return item.loader==="vanilla"?"Vanilla":item.loader==="fabric"?"Fabric":"Quilt";}
function instanceIcon(instance:Instance,overrides:Record<string,IconSpec>):IconSpec{
  return overrides[instance.id]||{
    symbol:ICON_OPTIONS[[...instance.name].reduce((n,c)=>n+c.charCodeAt(0),0)%ICON_OPTIONS.length],
    color:COLOR_OPTIONS[[...instance.id].reduce((n,c)=>n+c.charCodeAt(0),0)%COLOR_OPTIONS.length]
  };
}
function Art({instance,overrides,sources,large=false}:{instance:Instance;overrides:Record<string,IconSpec>;sources:Record<string,string>;large?:boolean}){
  const art=instanceIcon(instance,overrides);
  const url=!overrides[instance.id]?sources[instance.id]:null;
  return <span className={"freplay-art"+(large?" large":"")} style={{background:art.color}} aria-hidden="true">
    {url?<img className="freplay-art-image" src={url} alt="" loading="lazy"/>:<span>{art.symbol}</span>}
  </span>;
}
export default function PlayPage({
  versions,loading,selected,installed,isBusy,installAction,activeInstanceId,nickname,microsoftConnected,
  onPage,onLaunch,onVersion,onSelectInstance,onRefresh,onOpenProject,onLaunchInstance,onOpenInstance,onCreateInstance,onImportPack,importBusy
}:Props){
  const [instances,setInstances]=useState<Instance[]>([]);
  const [filter,setFilter]=useState("");
  const [sort,setSort]=useState("name");
  const [layout,setLayout]=useState<"grid"|"list">("grid");
  const [groups,setGroups]=useState<Group[]>(()=>readJson("fre-play-groups",[]));
  const [members,setMembers]=useState<Record<string,string>>(()=>readJson("fre-play-group-members",{}));
  const [icons,setIcons]=useState<Record<string,IconSpec>>(()=>readJson("fre-play-icons",{}));
  const [packArt]=useState<Record<string,string>>(()=>readJson("fre-play-instance-art",{}));
  const [selectedGroup,setSelectedGroup]=useState("");
  const [showNewGroup,setShowNewGroup]=useState(false);
  const [newGroupName,setNewGroupName]=useState("");
  const [editIconId,setEditIconId]=useState("");
  const [menuFor,setMenuFor]=useState("");
  const [featured,setFeatured]=useState<DiscoverHit[]>([]);
  const [error,setError]=useState("");
  const refresh=()=>{
    void invoke<Instance[]>("list_instances").then(setInstances).catch(e=>setError(String(e)));
  };
  useEffect(()=>{refresh();},[]);
  useEffect(()=>{
    let valid=true;
    void invoke<DiscoverHit[]>("search_modrinth",{query:"",version:null,projectType:"modpack",offset:0,loader:null,sort:"downloads"})
      .then(hits=>{if(valid)setFeatured(hits.slice(0,4));}).catch(()=>{});
    return()=>{valid=false;};
  },[]);
  useEffect(()=>{try{localStorage.setItem("fre-play-groups",JSON.stringify(groups));}catch{}},[groups]);
  useEffect(()=>{try{localStorage.setItem("fre-play-group-members",JSON.stringify(members));}catch{}},[members]);
  useEffect(()=>{try{localStorage.setItem("fre-play-icons",JSON.stringify(icons));}catch{}},[icons]);

  const visible=useMemo(()=>{
    const q=filter.trim().toLowerCase();
    return instances.filter(x=>(
      (!selectedGroup||members[x.id]===selectedGroup)&&
      (x.name+" "+x.version+" "+x.loader).toLowerCase().includes(q)
    )).sort((a,b)=>sort==="favorites"?
      Number(b.favorite)-Number(a.favorite)||a.name.localeCompare(b.name):
      sort==="version"?b.version.localeCompare(a.version):
      a.name.localeCompare(b.name));
  },[instances,selectedGroup,members,filter,sort]);
  const jump=instances.slice(0,5);
  const latest=versions.filter(x=>x.type==="release").slice(0,3);
  const editing=instances.find(x=>x.id===editIconId);
  const editingIcon=editing?instanceIcon(editing,icons):null;
  function createGroup(){
    const name=newGroupName.trim();
    if(!name||name.length>45||groups.some(x=>x.name.toLowerCase()===name.toLowerCase()))return;
    const id=Date.now().toString(36)+"-"+Math.random().toString(36).slice(2,7);
    setGroups(current=>[...current,{id,name}]);setSelectedGroup(id);
    setShowNewGroup(false);setNewGroupName("");
  }
  return <div className="freplay-layout">
    <div className="freplay-content">
      <section className="freplay-jump">
        <header className="freplay-section-head"><h2>Швидкий запуск</h2><span>{jump.length} / 5</span></header>
        {jump.length?<div className="freplay-jump-list">
          {jump.map(item=><article key={item.id} className={"freplay-jump-row"+(activeInstanceId===item.id?" active":"")}>
            <button className="freplay-jump-main" onClick={()=>onOpenInstance(item.id,item.version,item.memoryMb)}>
              <Art instance={item} overrides={icons} sources={packArt}/>
              <span className="freplay-jump-name"><b>{item.name}</b><small>{prettyLoader(item)} {item.version} · <Clock3 size={11}/> Збірка Minecraft</small></span>
            </button>
            <button className="freplay-jump-play" disabled={isBusy} onClick={()=>onLaunchInstance(item)}>
              <Play size={15} fill="currentColor"/> Грати
            </button>
            <button className="freplay-icon-action" title="Відкрити налаштування збірки" onClick={()=>onOpenInstance(item.id,item.version,item.memoryMb)}><MoreHorizontal size={19}/></button>
          </article>)}
        </div>:<div className="freplay-first">
          <Package size={28}/><strong>Тут з'являться твої збірки</strong>
          <p>Створи першу збірку, щоб запускати Minecraft безпосередньо з головної сторінки.</p>
          <div className="freplay-first-actions">
          <button className="freplay-green" onClick={onCreateInstance}><Plus size={16}/> Створити збірку</button>
          <button className="freplay-import-btn" disabled={importBusy} onClick={onImportPack}><Upload size={16}/> Імпортувати .mrpack</button>
          </div>
        </div>}
      </section>
      <section className="freplay-library">
        <div className="freplay-section-head"><h2>Бібліотека</h2><div className="freplay-header-actions">
          <button disabled={importBusy} onClick={onImportPack}><Upload size={14}/> Імпорт .mrpack</button>
          <button onClick={()=>setShowNewGroup(true)}><Plus size={14}/> Нова група</button>
          <button className="freplay-green" onClick={onCreateInstance}><Plus size={14}/> Нова збірка</button>
        </div></div>
        <div className="freplay-search-row">
          <label className="freplay-search"><Search size={17}/><input value={filter} onChange={e=>setFilter(e.target.value)} placeholder="Пошук збірок..." aria-label="Пошук збірок"/></label>
          <button className="freplay-refresh" onClick={refresh} title="Оновити бібліотеку" aria-label="Оновити бібліотеку"><BookOpen size={17}/></button>
        </div>
        <div className="freplay-filter-row">
          <label className="freplay-sort"><SlidersHorizontal size={15}/><select value={sort} onChange={e=>setSort(e.target.value)} aria-label="Сортування збірок">
            <option value="name">За назвою</option><option value="version">За версією</option><option value="favorites">Обрані першими</option>
          </select><ChevronDown size={12}/></label>
          <button className={layout==="grid"?"selected":""} onClick={()=>setLayout("grid")} title="Плитки" aria-label="Плитки"><Grid2X2 size={16}/></button>
          <button className={layout==="list"?"selected":""} onClick={()=>setLayout("list")} title="Список" aria-label="Список"><List size={17}/></button>
          <span className="freplay-count">{visible.length} збірок</span>
        </div>
        {!!groups.length&&<div className="freplay-group-pills">
          <button className={!selectedGroup?"selected":""} onClick={()=>setSelectedGroup("")}>Усі</button>
          {groups.map(group=><button key={group.id} className={selectedGroup===group.id?"selected":""} onClick={()=>setSelectedGroup(group.id)}>{group.name} <small>{instances.filter(x=>members[x.id]===group.id).length}</small></button>)}
        </div>}
        {visible.length?<div className={"freplay-cards "+(layout==="list"?"list":"")}>
          {visible.map(item=><article className="freplay-card" key={item.id}>
            <button className="freplay-card-open" onClick={()=>onOpenInstance(item.id,item.version,item.memoryMb)}>
              <Art instance={item} overrides={icons} sources={packArt} large/>
              <span className="freplay-card-label"><b>{item.name}</b><small>{prettyLoader(item)} {item.version}</small></span>
            </button>
            <div className="freplay-card-toolbar">
              <button title="Змінити значок" onClick={()=>setEditIconId(item.id)}><Palette size={14}/></button>
              <button title="Відкрити збірку" onClick={()=>onOpenInstance(item.id,item.version,item.memoryMb)}><FolderOpen size={14}/></button>
              <div className="freplay-menu-wrap"><button title="Групи збірки" onClick={()=>setMenuFor(x=>x===item.id?"":item.id)}><MoreHorizontal size={15}/></button>
                {menuFor===item.id&&<div className="freplay-menu"><small>Перемістити до групи</small>
                  <button onClick={()=>{setMembers(p=>({...p,[item.id]:""}));setMenuFor("");}}>Без групи</button>
                  {groups.map(group=><button key={group.id} onClick={()=>{setMembers(p=>({...p,[item.id]:group.id}));setMenuFor("");}}>{members[item.id]===group.id&&<Check size={12}/>} {group.name}</button>)}
                  {!groups.length&&<button onClick={()=>{setShowNewGroup(true);setMenuFor("");}}>Створити групу…</button>}
                </div>}
              </div>
            </div>
          </article>)}
        </div>:<div className="freplay-nomatches"><BookOpen size={25}/><b>{instances.length?"Нічого не знайдено":"Тут буде твоя бібліотека"}</b>
          <p>{instances.length?"Зміни пошук або фільтр групи.":"Створи збірку для Vanilla, Fabric або Quilt."}</p>
          <button onClick={()=>{setSelectedGroup("");setFilter("");onCreateInstance();}}>Створити збірку <ArrowRight size={14}/></button>
        </div>}
      </section>
      <section className="freplay-discovery">
        <div className="freplay-section-head"><h2>Відкрити модпаки</h2><button onClick={()=>onPage("Mods")}>Переглянути всі <ArrowRight size={14}/></button></div>
        <div className="freplay-packs">{featured.map(hit=><button key={hit.project_id} onClick={()=>onOpenProject(hit)} className="freplay-pack">
          <span className="freplay-pack-image">{hit.icon_url?<img src={hit.icon_url} alt=""/>:<Package size={29}/>}</span>
          <b>{hit.title}</b><small>{hit.author?"від "+hit.author:hit.description}</small>
          <em><Download size={12}/> {hit.downloads.toLocaleString("uk-UA")}</em>
        </button>)}
        {!featured.length&&<div className="freplay-no-packs">Проєкти Modrinth відобразяться, коли каталог стане доступним.</div>}</div>
      </section>
    </div>
    <aside className="freplay-right">
      <section className="freplay-getting-started">
        <h3>Початок роботи <ChevronDown size={15}/></h3>
        <button className={instances.length?"done":""} onClick={()=>onPage("Instances")}><span>{instances.length?<Check size={13}/>:<span/>}</span> Створити першу збірку <ArrowRight size={13}/></button>
        <button className={microsoftConnected?"done":""} onClick={()=>onPage("Accounts")}><span>{microsoftConnected?<Check size={13}/>:<span/>}</span> Увійти в Minecraft <ArrowRight size={13}/></button>
        <button className={installed?"done":""} onClick={()=>onPage("Versions")}><span>{installed?<Check size={13}/>:<span/>}</span> Встановити Minecraft <ArrowRight size={13}/></button>
      </section>
      <section className="freplay-account-tile">
        <h3>Профіль Minecraft</h3>
        <button onClick={()=>onPage("Accounts")}><span className="freplay-user-avatar">{nickname.charAt(0).toUpperCase()}</span><span><b>{nickname}</b><small>{microsoftConnected?"Microsoft Minecraft":"Локальний профіль"}</small></span><ChevronDown size={14}/></button>
      </section>
      <section className="freplay-shortcuts">
        <h3>Швидкі дії</h3>
        <button onClick={()=>onPage("Versions")}><Layers3 size={16}/> Версії Minecraft <ArrowRight size={13}/></button>
        <button onClick={()=>onPage("Mods")}><Compass size={16}/> Пошук контенту <ArrowRight size={13}/></button>
        <button onClick={()=>onPage("Servers")}><Play size={16}/> Ігрові сервери <ArrowRight size={13}/></button>
        <button onClick={()=>onPage("Settings")}><Settings2 size={16}/> Налаштування <ArrowRight size={13}/></button>
      </section>
      <section className="freplay-info"><ShieldCheck size={16}/><p>FRE LAUNCH — незалежний Minecraft-лаунчер. Світи й моди зберігаються у твоїх локальних збірках.</p></section>
    </aside>
    {showNewGroup&&<div className="freplay-modal-bg" onMouseDown={e=>{if(e.target===e.currentTarget)setShowNewGroup(false);}}>
      <div className="freplay-modal" role="dialog" aria-modal="true" aria-label="Нова група збірок">
        <header><h3>Створити групу</h3><button onClick={()=>setShowNewGroup(false)} aria-label="Закрити"><X size={18}/></button></header>
        <p>Групи організовують бібліотеку, не переміщуючи файли Minecraft.</p>
        <label>Назва групи</label><input autoFocus maxLength={45} value={newGroupName} placeholder="Мої модпаки" onChange={e=>setNewGroupName(e.target.value)} onKeyDown={e=>{if(e.key==="Enter")createGroup();}}/>
        <footer><button onClick={()=>setShowNewGroup(false)}>Скасувати</button><button className="freplay-green" disabled={!newGroupName.trim()||groups.some(g=>g.name.toLowerCase()===newGroupName.trim().toLowerCase())} onClick={createGroup}><Plus size={15}/> Створити</button></footer>
      </div>
    </div>}
    {editing&&editingIcon&&<div className="freplay-modal-bg" onMouseDown={e=>{if(e.target===e.currentTarget)setEditIconId("");}}>
      <div className="freplay-modal freplay-icon-editor" role="dialog" aria-modal="true" aria-label="Редактор значка збірки">
        <header><h3>Значок: {editing.name}</h3><button onClick={()=>setEditIconId("")} aria-label="Закрити"><X size={18}/></button></header>
        <div className="freplay-icon-preview"><Art instance={editing} overrides={icons} sources={packArt} large/></div>
        <label>Колір значка</label><div className="freplay-colors">{COLOR_OPTIONS.map(color=><button key={color} style={{background:color}} className={editingIcon.color===color?"chosen":""} title={color} onClick={()=>setIcons(old=>({...old,[editing.id]:{...editingIcon,color}}))}/>)}</div>
        <label>Символ</label><div className="freplay-symbols">{ICON_OPTIONS.map(symbol=><button className={editingIcon.symbol===symbol?"chosen":""} key={symbol} onClick={()=>setIcons(old=>({...old,[editing.id]:{...editingIcon,symbol}}))}>{symbol}</button>)}</div>
        <footer><button onClick={()=>setIcons(old=>{const next={...old};delete next[editing.id];return next;})}><Trash2 size={14}/> Стандартний</button><button className="freplay-green" onClick={()=>setEditIconId("")}><Check size={15}/> Готово</button></footer>
      </div>
    </div>}
    {error&&<div className="freplay-error" role="alert">{error}<button onClick={()=>setError("")}><X size={14}/></button></div>}
  </div>;
}
