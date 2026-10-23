import { Activity, ArrowRight, CircleCheck, CircleDashed, Database, Folder, HardDrive, ShieldAlert, Terminal, Wifi } from "lucide-react";

export type Preflight = {path:string;ready:boolean;detail:string};
export type InstallProgress = {stage:string;current:number;total:number;message:string};

const timeline=[
  {id:"preflight",title:"Диск",hint:"Доступ до папки"},
  {id:"metadata",title:"Маніфест",hint:"Дані Mojang"},
  {id:"client",title:"Клієнт",hint:"Minecraft JAR"},
  {id:"libraries",title:"Бібліотеки",hint:"Java-залежності"},
  {id:"assets",title:"Ресурси",hint:"Звуки й текстури"},
  {id:"natives",title:"Розпакування",hint:"Файли для Windows"},
  {id:"complete",title:"Завершено",hint:"Файли готові"},
];
const stageIndex:Record<string,number>={preflight:0,metadata:1,client:2,libraries:3,assets:4,natives:5,complete:6};

export default function InstallDeck({version,progress,preflight,isInstalling,onCheck,onLogs}:{
  version:string;progress:InstallProgress|null;preflight:Preflight|null;isInstalling:boolean;onCheck:()=>void;onLogs:()=>void
}){
  const index=progress?stageIndex[progress.stage]??0:0;
  const percent=progress?.stage==="complete"?100:progress&&progress.total>0
    ?Math.min(100,Math.floor(progress.current/progress.total*100)):0;
  return <section className="install-deck">
    <div className="install-deck-head">
      <div><span className="install-eyebrow"><Activity size={15}/> FRE / DOWNLOAD ENGINE</span>
        <h2>МІСІЯ: <em>ВСТАНОВИТИ.</em></h2>
        <p>Живі етапи, фактичний прогрес і перевірка диска. Без намальованих відсотків.</p>
      </div>
      <span className={"install-beacon"+(isInstalling?" is-active":"")}>{isInstalling?"У ПРОЦЕСІ":"ОЧІКУВАННЯ"}</span>
    </div>
    <div className="install-deck-body">
      <div className="install-deck-path">
        <div className="install-path-label"><Folder size={16}/> КУДИ ЗБЕРІГАЄМО MINECRAFT</div>
        <strong>{preflight?.path??"Папку даних ще не перевірено"}</strong>
        <div className="install-path-bottom">
          <span className={preflight?.ready?"install-path-ok":"install-path-neutral"}>
            {preflight?.ready?<CircleCheck size={15}/>:<ShieldAlert size={15}/>}
            {preflight?preflight.detail:"Спочатку перевіримо доступ до запису"}
          </span>
          <button onClick={onCheck} disabled={isInstalling}><HardDrive size={15}/> ПЕРЕВІРИТИ ДИСК</button>
        </div>
        <small>Папка гри відрізняється від місця встановлення самого FRE LAUNCH.</small>
      </div>
      <div className="install-deck-sequence">
        <div className="install-sequence-heading"><span>ТЕХНІЧНИЙ МАРШРУТ / 07</span><span>НЕЗАЛЕЖНІ ЕТАПИ</span></div>
        <div className="install-checkpoints">{timeline.map((stage,i)=>{
          const done=progress?.stage==="complete"||!!progress&&i<index;
          const active=!!progress&&i===index&&progress.stage!=="complete";
          return <div key={stage.id} className={"install-checkpoint"+(done?" is-done":"")+(active?" is-current":"")}>
            <span className="install-step-mark">{done?<CircleCheck size={18}/>:active?<Wifi size={18}/>:<CircleDashed size={18}/>}</span>
            <b>{stage.title}</b><small>{stage.hint}</small>
          </div>;
        })}</div>
      </div>
    </div>
    <div className="install-deck-bottom">
      <div className="install-live">
        <span><Database size={17}/> {version||"MINECRAFT JAVA"}</span>
        <strong>{progress?.message??"Вибери версію та натисни «Встановити»"}</strong>
        <small>{progress ? "ПРОГРЕС ПОТОЧНОГО ЕТАПУ" : "ДАНІ ЗАВАНТАЖЕННЯ З'ЯВЛЯТЬСЯ ТУТ"}</small>
      </div>
      <div className="install-live-progress">
        <strong>{progress?percent+"%":"—"}</strong>
        <div className="install-live-track"><span style={{width:percent+"%"}}/></div>
        <small>{progress ? progress.current+" / "+progress.total : "Немає активних файлів"}</small>
      </div>
      <button className="install-diag-link" onClick={onLogs}><Terminal size={17}/> ЖУРНАЛ <ArrowRight size={17}/></button>
    </div>
  </section>;
}
