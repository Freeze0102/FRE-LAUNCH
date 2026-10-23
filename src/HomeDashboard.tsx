import { ArrowDownToLine, ArrowRight, Boxes, Check, ChevronRight, Coffee, Download, Layers3, Play, Puzzle, RefreshCw, Settings2, ShieldCheck, SlidersHorizontal, UserRound } from "lucide-react";

type Release = { id:string; type:string; releaseTime:string };
type Destination = "Versions" | "Instances" | "Mods" | "Downloads" | "Settings" | "Accounts";
type Props = {
  versions:Release[];
  loading:boolean;
  selected:string;
  selectedKind:string;
  installed:boolean;
  isBusy:boolean;
  installAction:string;
  activeInstanceId:string;
  memoryMb:number;
  nickname:string;
  java:string;
  recoveryMode:boolean;
  onPage:(destination:Destination)=>void;
  onLaunch:()=>void;
  onVersion:(id:string)=>void;
  onRefresh:()=>void;
};

export default function HomeDashboard({versions,loading,selected,selectedKind,installed,isBusy,installAction,activeInstanceId,memoryMb,nickname,java,recoveryMode,onPage,onLaunch,onVersion,onRefresh}:Props){
  const releases=versions.filter(version=>version.type==="release").slice(0,4);
  const javaState=java.startsWith("Java в PATH")?"Не знайдено":java.startsWith("Перевірка")?"Перевіряємо":"Виявлено";
  return <div className="fre-play-home">
    <div className="fre-play-dashboard">
      <section className="fre-play-primary" aria-label="Запуск Minecraft">
        <div className="fre-play-eyebrow"><span className="fre-play-pulse"/> MINECRAFT: JAVA EDITION <span className="fre-play-type">FRE LAUNCH</span></div>
        <div className="fre-play-welcome">
          <div><h2>Готовий грати?</h2><p>Обери версію Minecraft або свою збірку й запускай гру.</p></div>
          <span className="fre-play-version-badge">JAVA</span>
        </div>
        <div className="fre-play-choices">
          <button className="fre-play-choice" onClick={()=>onPage("Versions")}>
            <span className="fre-play-choice-icon"><Layers3 size={21}/></span>
            <span><small>Версія Minecraft</small><strong>{selected||"Обрати версію"}</strong><em>{selected?selectedKind:"Офіційні релізи Mojang"}</em></span>
            <ChevronRight size={18}/>
          </button>
          <button className="fre-play-choice" onClick={()=>onPage("Instances")}>
            <span className="fre-play-choice-icon"><Boxes size={21}/></span>
            <span><small>Активна збірка</small><strong>{activeInstanceId?"Обрана збірка":"Без окремої збірки"}</strong><em>{activeInstanceId?"Конфігурація із бібліотеки":"Створи або вибери власну збірку"}</em></span>
            <ChevronRight size={18}/>
          </button>
        </div>
        <div className="fre-play-actions">
          <button className="fre-play-button" disabled={!selected||isBusy} onClick={onLaunch}>
            {installed?<Play size={20} fill="currentColor"/>:<ArrowDownToLine size={20}/>}
            {installAction}
          </button>
          <span className="fre-play-action-hint">{!selected?"Спочатку обери версію":installed?"Файли гри готові":'Для цієї версії потрібне завантаження'}</span>
        </div>
      </section>
      <aside className="fre-play-status" aria-label="Параметри гри">
        <div className="fre-play-status-title"><span>Параметри запуску</span><button onClick={()=>onPage("Settings")} aria-label="Змінити налаштування"><Settings2 size={18}/></button></div>
        <div className="fre-play-status-item"><UserRound size={19}/><span><small>Профіль</small><strong>{nickname}</strong></span><button onClick={()=>onPage("Accounts")}>Змінити <ChevronRight size={15}/></button></div>
        <div className="fre-play-status-item"><SlidersHorizontal size={19}/><span><small>Оперативна пам'ять</small><strong>{(memoryMb/1024).toFixed(1)} GB</strong></span><button onClick={()=>onPage("Settings")}>Змінити <ChevronRight size={15}/></button></div>
        <div className="fre-play-status-item"><Coffee size={19}/><span><small>Java</small><strong title={java}>{javaState}</strong></span><button onClick={()=>onPage("Settings")}>Деталі <ChevronRight size={15}/></button></div>
        <div className="fre-play-status-note"><ShieldCheck size={17}/><span>{recoveryMode?"Захисний режим WebView2 активний":"Файли збірок зберігаються локально"}</span></div>
      </aside>
    </div>
    <section className="fre-play-section" aria-label="Останні версії">
      <div className="fre-play-section-heading"><div><h3>Останні релізи</h3><p>Швидкий вибір офіційної версії Minecraft.</p></div><button onClick={()=>onPage("Versions")}>Усі версії <ArrowRight size={16}/></button></div>
      <div className="fre-play-releases">
        {releases.map(version=><button key={version.id} className={"fre-play-release"+(selected===version.id?" is-selected":"")} onClick={()=>onVersion(version.id)}>
          <span className="fre-play-release-icon"><Layers3 size={19}/></span><span><b>{version.id}</b><small>{new Date(version.releaseTime).toLocaleDateString("uk-UA")}</small></span>
          {selected===version.id?<Check size={17}/>:<ChevronRight size={17}/>}
        </button>)}
        {!releases.length&&<div className="fre-play-release-empty">{loading?"Завантаження релізів Mojang…":"Список релізів недоступний."}<button onClick={onRefresh}>Повторити <RefreshCw size={14}/></button></div>}
      </div>
    </section>
    <section className="fre-play-section" aria-label="Бібліотека та вміст">
      <div className="fre-play-section-heading"><div><h3>Бібліотека та вміст</h3><p>Керуй збірками, модами та файлами гри.</p></div></div>
      <div className="fre-play-shortcuts">
        <button onClick={()=>onPage("Instances")}><span className="fre-play-shortcut-icon"><Boxes size={23}/></span><strong>Мої збірки</strong><small>Перегляд і створення збірок</small><ArrowRight size={17}/></button>
        <button onClick={()=>onPage("Mods")}><span className="fre-play-shortcut-icon"><Puzzle size={23}/></span><strong>Моди й ресурси</strong><small>Пошук у каталозі Modrinth</small><ArrowRight size={17}/></button>
        <button onClick={()=>onPage("Downloads")}><span className="fre-play-shortcut-icon"><Download size={23}/></span><strong>Завантаження</strong><small>Файли та прогрес встановлення</small><ArrowRight size={17}/></button>
      </div>
    </section>
  </div>;
}
