import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ArrowDownToLine, CheckCircle2, ChevronDown, RefreshCw, ShieldCheck, X, Zap } from "lucide-react";

type UpdateInfo = {currentVersion:string; version:string; notes:string; sizeBytes:number};
type UpdateProgress = {downloadedBytes:number;totalBytes:number;phase:string};
type UpdateState = "checking"|"idle"|"available"|"downloading"|"installing"|"error";
const UPDATE_MODE_KEY="fre-launch-install-mode";

export default function UpdateCenter(){
  const [info,setInfo]=useState<UpdateInfo|null>(null);
  const [state,setState]=useState<UpdateState>("idle");
  const [open,setOpen]=useState(false);
  const [message,setMessage]=useState("");
  const [progress,setProgress]=useState(0);
  const [mode,setMode]=useState(()=>{
    try{return localStorage.getItem(UPDATE_MODE_KEY)==="allUsers"?"allUsers":"currentUser";}
    catch{return "currentUser";}
  });

  const check=useCallback(async(silent=false)=>{
    setState("checking");
    if(!silent)setMessage("");
    try{
      const latest=await invoke<UpdateInfo|null>("check_launcher_update");
      setInfo(latest);
      setState(latest?"available":"idle");
      if(!latest&&!silent)setMessage("Встановлено актуальну версію.");
    }catch(err){
      setState("error");
      // No GitHub Release / network outage must never prevent startup.
      setMessage("Не вдалося перевірити оновлення: "+String(err));
    }
  },[]);

  useEffect(()=>{
    // Wait until the initial launcher window is already visible and responsive.
    const timer=window.setTimeout(()=>void check(true),10000);
    const subscription=listen<UpdateProgress>("launcher-update-progress",e=>{
      const p=e.payload;
      if(p.phase==="verified"){
        setProgress(100);
        setState("installing");
        setMessage("Перевірено SHA-256. Запускаємо інсталятор…");
      }else if(p.totalBytes>0){
        setProgress(Math.max(0,Math.min(99,Math.round(p.downloadedBytes/p.totalBytes*100))));
      }
    });
    return ()=>{
      window.clearTimeout(timer);
      void subscription.then(unlisten=>unlisten());
    };
  },[check]);

  async function install(){
    if(!info||state==="downloading"||state==="installing")return;
    if(!window.confirm(`Встановити FRE LAUNCH ${info.version}? Після завантаження відкриється інсталятор, а лаунчер закриється. Твої світи та налаштування збережуться.`))return;
    try{localStorage.setItem(UPDATE_MODE_KEY,mode);}catch{/* no storage */}
    setState("downloading");setMessage("Завантажуємо перевірений інсталятор…");setProgress(0);
    try{
      const result=await invoke<string>("install_launcher_update",{version:info.version,mode});
      setState("installing");setMessage(result);
    }catch(err){setState("error");setMessage("Помилка оновлення: "+String(err));}
  }
  return <section className={"fre-updates"+(info?" has-release":"")} aria-label="Оновлення FRE LAUNCH">
    <button className="fre-updates-trigger" onClick={()=>setOpen(v=>!v)} aria-expanded={open}>
      {info?<Zap size={15}/>:<RefreshCw size={15}/>}
      <span>{info?`НОВА ВЕРСІЯ ${info.version}`:"ОНОВЛЕННЯ"}</span>
      {state==="checking"&&<span className="fre-updates-spinner">···</span>}
      <ChevronDown size={15}/>
    </button>
    {open&&<div className="fre-updates-body">
      <div className="fre-updates-header"><strong>FRE / UPDATE CENTER</strong><button title="Закрити" onClick={()=>setOpen(false)}><X size={15}/></button></div>
      {info?<>
        <p>FRE LAUNCH <b>{info.version}</b> доступний. Поточна версія: {info.currentVersion}.</p>
        {info.notes&&<p className="fre-updates-notes">{info.notes.slice(0,380)}</p>}
        <label htmlFor="fre-update-mode">ТИП ВСТАНОВЛЕННЯ</label>
        <select id="fre-update-mode" value={mode} disabled={state==="downloading"||state==="installing"} onChange={e=>setMode(e.target.value)}>
          <option value="currentUser">Для мене (звичайний)</option>
          <option value="allUsers">Для всіх (адміністратор)</option>
        </select>
        <small>Обери той самий режим, яким встановлював FRE LAUNCH раніше.</small>
        {(state==="downloading"||state==="installing")&&<div className="fre-updates-progress"><div style={{width:`${progress}%`}}/><span>{progress}%</span></div>}
        <button className="fre-updates-install" disabled={state==="downloading"||state==="installing"} onClick={()=>void install()}>
          <ArrowDownToLine size={16}/> {state==="downloading"?"ЗАВАНТАЖЕННЯ…":state==="installing"?"ВСТАНОВЛЕННЯ…":"ЗАВАНТАЖИТИ Й ОНОВИТИ"}
        </button>
      </>:<>
        <p>Перевірка нових версій через офіційний GitHub Releases.</p>
        <button className="fre-updates-check" disabled={state==="checking"} onClick={()=>void check()}><RefreshCw size={14}/> {state==="checking"?"ПЕРЕВІРЯЄМО":"ПЕРЕВІРИТИ ЗАРАЗ"}</button>
      </>}
      {message&&<p className="fre-updates-message" role="status">{message}</p>}
      <div className="fre-updates-verified"><ShieldCheck size={14}/> SHA-256 перевіряється до запуску інсталятора.</div>
    </div>}
  </section>;
}
