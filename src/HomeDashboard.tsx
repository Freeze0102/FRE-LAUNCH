import {useEffect,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {ArrowDownToLine,ArrowRight,Boxes,Check,ChevronRight,Clock3,Coffee,FolderOpen,Gamepad2,Layers3,Package,Play,Plus,RefreshCw,ShieldCheck,SlidersHorizontal,UserRound} from "lucide-react";

type Release={id:string;type:string;releaseTime:string};
type Instance={id:string;name:string;version:string;favorite:boolean;loader:string;loaderVersion:string|null;memoryMb:number};
type Destination="Versions"|"Instances"|"Mods"|"Downloads"|"Settings"|"Accounts";
type Props={
  versions:Release[];loading:boolean;selected:string;selectedKind:string;
  installed:boolean;isBusy:boolean;installAction:string;activeInstanceId:string;
  memoryMb:number;nickname:string;java:string;recoveryMode:boolean;
  onPage:(destination:Destination)=>void;onLaunch:()=>void;onVersion:(id:string)=>void;
  onSelectInstance:(id:string,version:string,memory:number)=>void;onRefresh:()=>void;
};

export default function HomeDashboard({versions,loading,selected,selectedKind,installed,isBusy,installAction,activeInstanceId,memoryMb,nickname,java,recoveryMode,onPage,onLaunch,onVersion,onSelectInstance,onRefresh}:Props){
  const [instances,setInstances]=useState<Instance[]>([]);
  useEffect(()=>{
    let alive=true;
    void invoke<Instance[]>("list_instances").then(items=>{if(alive)setInstances(items);}).catch(()=>{});
    return ()=>{alive=false;};
  },[]);
  const releases=versions.filter(v=>v.type==="release").slice(0,5);
  const active=instances.find(v=>v.id===activeInstanceId);
  const javaState=java.startsWith("Java в PATH")?"Не знайдено":java.startsWith("Перевірка")?"Перевіряємо":"Виявлено";
  return <div className="fl-home">
    <div className="fl-section-kicker"><span className="fl-pulse"/> ЛАУНЧЕР ГОТОВИЙ <span className="fl-kicker-line"/> <span>MINECRAFT JAVA EDITION</span></div>
    <div className="fl-workbench">
      <section className="fl-game-stage">
        <div className="fl-stage-top"><div className="fl-monogram"><Gamepad2 size={22}/></div><span className="fl-runtime-badge">{installed?"ВСТАНОВЛЕНО":"ПОТРІБНЕ ВСТАНОВЛЕННЯ"}</span></div>
        <div className="fl-game-identity">
          <div className="fl-game-art" aria-hidden="true"><div className="fl-pixel-sun"/><div className="fl-pixel-mountain"/><div className="fl-pixel-grass"/><div className="fl-pixel-ground"/></div>
          <div className="fl-game-copy"><span>ВАШ ІГРОВИЙ ПРОФІЛЬ</span><h2>{active?.name||"Minecraft Java"}</h2><p>{selected||"Обери версію"} <span>•</span> {active?active.loader==="vanilla"?"Vanilla":active.loader.toUpperCase():"Звичайний профіль"}</p></div>
        </div>
        <div className="fl-stage-bottom">
          <button className="fl-launch" disabled={!selected||isBusy} onClick={onLaunch}>{installed?<Play size={19} fill="currentColor"/>:<ArrowDownToLine size={19}/>} {installAction}</button>
          <button className="fl-pick-version" onClick={()=>onPage("Versions")}><Layers3 size={16}/> Версія <ChevronRight size={17}/></button>
        </div>
      </section>
      <aside className="fl-library-stage">
        <div className="fl-library-heading"><div><span>LIBRARY</span><h3>Мої збірки</h3></div><button onClick={()=>onPage("Instances")} title="Створити збірку" aria-label="Створити збірку"><Plus size={19}/></button></div>
        <div className="fl-instance-list">
          {instances.slice(0,4).map((item,i)=><button key={item.id} className={"fl-instance"+(item.id===activeInstanceId?" chosen":"")} onClick={()=>onSelectInstance(item.id,item.version,item.memoryMb)}>
            <span className="fl-instance-art"><Boxes size={19}/></span><span className="fl-instance-copy"><strong>{item.name}</strong><small>Minecraft {item.version} · {item.loader==="vanilla"?"Vanilla":item.loader==="fabric"?"Fabric":"Quilt"}</small></span>{item.id===activeInstanceId?<Check size={18}/>:<ChevronRight size={17}/>}
          </button>)}
          {!instances.length&&<div className="fl-empty-library"><Boxes size={25}/><p>Ще немає власних збірок.</p><button onClick={()=>onPage("Instances")}>Створити першу <ArrowRight size={15}/></button></div>}
        </div>
        <button className="fl-library-all" onClick={()=>onPage("Instances")}>Керувати бібліотекою <ArrowRight size={16}/></button>
      </aside>
    </div>

    <div className="fl-stat-grid">
      <button onClick={()=>onPage("Accounts")}><span className="fl-stat-icon"><UserRound size={19}/></span><span><small>ГРАВЕЦЬ</small><strong>{nickname}</strong></span><ChevronRight size={16}/></button>
      <button onClick={()=>onPage("Settings")}><span className="fl-stat-icon"><SlidersHorizontal size={19}/></span><span><small>ПАМ'ЯТЬ</small><strong>{(memoryMb/1024).toFixed(1)} GB RAM</strong></span><ChevronRight size={16}/></button>
      <button onClick={()=>onPage("Settings")}><span className="fl-stat-icon"><Coffee size={19}/></span><span><small>JAVA RUNTIME</small><strong>{javaState}</strong></span><ChevronRight size={16}/></button>
      <button onClick={()=>onPage("Downloads")}><span className="fl-stat-icon"><FolderOpen size={19}/></span><span><small>ФАЙЛИ ГРИ</small><strong>Відкрити папки</strong></span><ChevronRight size={16}/></button>
    </div>

    <div className="fl-bottom-grid">
      <section className="fl-latest">
        <div className="fl-section-heading"><div><span>MOJANG RELEASES</span><h3>Швидкий вибір версії</h3></div><button onClick={()=>onPage("Versions")}>Усі версії <ArrowRight size={15}/></button></div>
        <div className="fl-release-list">
          {releases.map(r=><button key={r.id} className={"fl-release"+(r.id===selected?" active":"")} onClick={()=>onVersion(r.id)}><Layers3 size={17}/><b>{r.id}</b><small><Clock3 size={12}/> {new Date(r.releaseTime).toLocaleDateString("uk-UA")}</small>{r.id===selected?<Check size={16}/>:<ChevronRight size={16}/>}</button>)}
          {!releases.length&&<div className="fl-loading-list">{loading?"Завантажуємо релізи Mojang…":"Дані про версії недоступні."}<button onClick={onRefresh}><RefreshCw size={14}/> Оновити</button></div>}
        </div>
      </section>
      <section className="fl-explore">
        <div className="fl-section-heading"><div><span>DISCOVER</span><h3>Моди та ресурси</h3></div><Package size={20}/></div>
        <p>Збирай власні версії Minecraft. Підтримка Fabric, Quilt, модів та ресурс-паків через Modrinth.</p>
        <div className="fl-explore-steps"><span>01 <b>Створи збірку</b></span><span>02 <b>Обери модлоадер</b></span><span>03 <b>Додай моди</b></span></div>
        <button onClick={()=>onPage("Mods")}><Package size={17}/> Відкрити каталог <ArrowRight size={16}/></button>
        <small><ShieldCheck size={14}/> {recoveryMode?"Режим відновлення WebView2 активний":"Локальні файли та окремі світи для збірок"}</small>
      </section>
    </div>
  </div>;
}