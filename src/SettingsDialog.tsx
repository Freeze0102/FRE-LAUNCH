import {useEffect,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {
  X,Paintbrush,Lightbulb,SlidersHorizontal,Languages,UserRound,
  Coffee,FolderOpen,Wrench,ShieldCheck,Archive,ChevronRight,
  Monitor,Check,Images,Shirt,Boxes,HardDrive,RefreshCw
} from "lucide-react";
import JavaPanel from "./JavaPanel";
import "./settings-dialog.css";

export type FeatureSettings={showSkins:boolean;showScreenshots:boolean;showWorlds:boolean;showFiles:boolean};
type Section="appearance"|"features"|"behavior"|"language"|"profile"|"java"|"resources"|"troubleshooting";
const sections:{id:Section;label:string;icon:typeof Paintbrush;group:string}[]=[
  {id:"appearance",label:"Вигляд",icon:Paintbrush,group:"ДИСПЛЕЙ"},
  {id:"features",label:"Особливості",icon:Lightbulb,group:"ДИСПЛЕЙ"},
  {id:"behavior",label:"Поведінка",icon:SlidersHorizontal,group:"ДИСПЛЕЙ"},
  {id:"language",label:"Мова",icon:Languages,group:"ДИСПЛЕЙ"},
  {id:"profile",label:"Профіль",icon:UserRound,group:"АКАУНТ"},
  {id:"java",label:"Java та пам'ять",icon:Coffee,group:"ПАРАМЕТРИ ЗБІРОК"},
  {id:"resources",label:"Ресурси",icon:HardDrive,group:"СИСТЕМА"},
  {id:"troubleshooting",label:"Діагностика",icon:Wrench,group:"СИСТЕМА"}
];
const themes=[
  {id:"midnight",label:"Темна",color:"#1b1d23"},
  {id:"emerald",label:"Смарагдова",color:"#22613b"},
  {id:"ocean",label:"Океан",color:"#24516c"},
  {id:"neon",label:"Неон",color:"#503475"},
  {id:"light",label:"Світла",color:"#d9dfe1"}
];
type Props={
  onClose:()=>void;onNavigate:(target:"Accounts"|"Logs"|"Instances"|"Downloads")=>void;
  theme:string;onTheme:(next:string)=>void;
  features:FeatureSettings;onFeatures:(next:FeatureSettings)=>void;
  memoryMb:number;onMemory:(value:number)=>void;instanceId:string;
  recoveryMode:boolean;onRecoveryDisabled:()=>void;
};
function Switch({checked,onChange,label,detail}:{checked:boolean;onChange:(checked:boolean)=>void;label:string;detail?:string}){
  return <label className="fre-settings-option">
    <span><strong>{label}</strong>{detail&&<small>{detail}</small>}</span>
    <span className="fre-settings-switch"><input type="checkbox" checked={checked} onChange={e=>onChange(e.target.checked)} aria-label={label}/><i/></span>
  </label>;
}
export default function SettingsDialog({
  onClose,onNavigate,theme,onTheme,features,onFeatures,memoryMb,onMemory,instanceId,recoveryMode,onRecoveryDisabled
}:Props){
  const [section,setSection]=useState<Section>("appearance");
  const [preflight,setPreflight]=useState<{path:string;ready:boolean;detail:string}|null>(null);
  const [loading,setLoading]=useState(false);
  useEffect(()=>{
    const escape=(event:KeyboardEvent)=>{if(event.key==="Escape")onClose();};
    window.addEventListener("keydown",escape);
    return()=>window.removeEventListener("keydown",escape);
  },[onClose]);
  useEffect(()=>{
    if(section!=="resources")return;
    let active=true;
    setLoading(true);
    void invoke<{path:string;ready:boolean;detail:string}>("install_preflight").then(value=>{if(active)setPreflight(value);}).catch(()=>{if(active)setPreflight(null);}).finally(()=>{if(active)setLoading(false);});
    return()=>{active=false;};
  },[section]);
  const update=(key:keyof FeatureSettings,checked:boolean)=>onFeatures({...features,[key]:checked});
  return <div className="fre-settings-backdrop" onMouseDown={e=>{if(e.target===e.currentTarget)onClose();}}>
    <section className="fre-settings-dialog" role="dialog" aria-modal="true" aria-labelledby="fre-settings-title">
      <header className="fre-settings-dialog-head">
        <h2 id="fre-settings-title">Налаштування</h2>
        <button onClick={onClose} aria-label="Закрити налаштування"><X size={21}/></button>
      </header>
      <div className="fre-settings-dialog-body">
        <nav className="fre-settings-side" aria-label="Категорії налаштувань">
          {sections.map((item,index)=>{
            const Icon=item.icon;
            const groupNew=index===0||sections[index-1].group!==item.group;
            return <div key={item.id}>
              {groupNew&&<span className="fre-settings-group">{item.group}</span>}
              <button className={section===item.id?"active":""} onClick={()=>setSection(item.id)}><Icon size={17}/>{item.label}</button>
            </div>;
          })}
          <div className="fre-settings-side-version"><span className="fre-settings-brand-mark">F</span><span>FRE LAUNCH 0.5.7<small>Windows · Alpha</small></span></div>
        </nav>
        <div className="fre-settings-body-content">
          {section==="appearance"&&<>
            <h3>Вигляд</h3><p>Обери оформлення інтерфейсу FRE LAUNCH. Зміна теми зберігається автоматично.</p>
            <div className="fre-settings-theme-list">{themes.map(item=><button key={item.id} className={theme===item.id?"selected":""} onClick={()=>onTheme(item.id)}>
              <span style={{background:item.color}}/><strong>{item.label}</strong>{theme===item.id&&<Check size={17}/>}
            </button>)}</div>
          </>}
          {section==="features"&&<>
            <h3>Особливості</h3><p>Налаштуй, які вкладки й розділи відображаються. Дані та файли не видаляються.</p>
            <h4>Бібліотека збірок</h4>
            <Switch label="Вкладка «Світи»" detail="Показувати світи Minecraft на сторінці збірки." checked={features.showWorlds} onChange={value=>update("showWorlds",value)}/>
            <Switch label="Вкладка «Файли»" detail="Показувати менеджер папок у збірках." checked={features.showFiles} onChange={value=>update("showFiles",value)}/>
            <h4>Бічна панель</h4>
            <Switch label="Скіни" detail="Показувати перемикач скінів у навігації." checked={features.showSkins} onChange={value=>update("showSkins",value)}/>
            <Switch label="Скріншоти" detail="Показувати галерею скриншотів у навігації." checked={features.showScreenshots} onChange={value=>update("showScreenshots",value)}/>
          </>}
          {section==="behavior"&&<>
            <h3>Поведінка</h3><p>FRE LAUNCH зберігає активну збірку та обрані ресурси між перезапусками.</p>
            <div className="fre-settings-note"><ShieldCheck size={18}/> Гра запускається окремим процесом Java. Налаштування Minecraft та світи залишаються в AppData.</div>
            <div className="fre-settings-note"><Boxes size={18}/> Для кожної збірки можна окремо змінити RAM, Java та модлоадер у вкладці «Налаштування».</div>
            <button className="fre-settings-secondary" onClick={()=>onNavigate("Instances")}>Відкрити бібліотеку збірок <ChevronRight size={15}/></button>
          </>}
          {section==="language"&&<>
            <h3>Мова</h3><p>Мова інтерфейсу цієї збірки — українська. Повне перемикання інших локалізацій поки не реалізовано.</p>
            <div className="fre-settings-language"><span>🇺🇦</span><b>Українська</b><Check size={17}/></div>
          </>}
          {section==="profile"&&<>
            <h3>Акаунт Minecraft</h3><p>Керуй Microsoft-підключенням та офлайн-профілями на окремій сторінці.</p>
            <div className="fre-settings-note"><UserRound size={18}/> Microsoft-вхід потребує Client ID застосунку, дозволеного Minecraft Services. Звичайний нікнейм не дає ліцензійного доступу.</div>
            <button className="fre-settings-primary" onClick={()=>onNavigate("Accounts")}><UserRound size={16}/> Керувати акаунтами</button>
          </>}
          {section==="java"&&<>
            <h3>Java та пам'ять</h3>
            <p>Глобальні налаштування використовуються без окремої збірки. Налаштування активної збірки мають пріоритет.</p>
            <h4>Пам'ять Minecraft</h4>
            <div className="fre-settings-memory"><span>{(memoryMb/1024).toFixed(1)} GB</span><input disabled={!!instanceId} type="range" min={1024} max={16384} step={512} value={memoryMb} onChange={e=>onMemory(Number(e.target.value))}/><small>1 GB <span>16 GB</span></small></div>
            {instanceId&&<div className="fre-settings-note">Поточну RAM змінюй у налаштуваннях вибраної збірки. <button onClick={()=>onNavigate("Instances")}>Відкрити збірку <ChevronRight size={14}/></button></div>}
            <h4>Java Runtime</h4><div className="fre-settings-java"><JavaPanel instanceId={instanceId}/></div>
          </>}
          {section==="resources"&&<>
            <h3>Ресурси</h3><p>Папка даних, світи та встановлені моди зберігаються локально.</p>
            <h4>Сховище Minecraft</h4>
            <div className="fre-settings-path"><FolderOpen size={17}/><span>{loading?"Перевіряємо…":preflight?.path||"Не вдалося визначити шлях"}</span></div>
            <div className="fre-settings-note"><ShieldCheck size={17}/> {preflight?.ready?"Доступ до ігрових даних перевірено":preflight?.detail||"Статус сховища невідомий"}</div>
            <button className="fre-settings-secondary" onClick={()=>onNavigate("Downloads")}>Відкрити менеджер файлів <ChevronRight size={15}/></button>
          </>}
          {section==="troubleshooting"&&<>
            <h3>Діагностика</h3><p>Останні помилки запуску, журнали Minecraft і звіти Windows.</p>
            {recoveryMode&&<div className="fre-settings-note"><Monitor size={17}/> Зараз працює захисний режим WebView2 без GPU-прискорення.</div>}
            {recoveryMode&&<button className="fre-settings-secondary" onClick={onRecoveryDisabled}><RefreshCw size={16}/> Спробувати стандартну графіку після перезапуску</button>}
            <button className="fre-settings-primary" onClick={()=>onNavigate("Logs")}><Wrench size={16}/> Відкрити діагностику</button>
          </>}
        </div>
      </div>
    </section>
  </div>;
}
