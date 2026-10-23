import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Coffee, RefreshCw, CheckCircle2, DownloadCloud } from "lucide-react";
import type { Instance } from "./Features";

type Runtime = {path:string;version:string;major:number};
type Progress = {phase:string;done:number;total:number;message:string};
const jreVersions=[8,11,17,21,25];

export default function JavaPanel({instanceId}:{instanceId:string}) {
  const [runtimes,setRuntimes]=useState<Runtime[]>([]);
  const [busy,setBusy]=useState(false);
  const [installing,setInstalling]=useState(false);
  const [version,setVersion]=useState(21);
  const [progress,setProgress]=useState<Progress|null>(null);
  const [message,setMessage]=useState("");
  const [error,setError]=useState("");

  async function refresh() {
    setBusy(true);setError("");
    try {setRuntimes(await invoke<Runtime[]>("list_java_runtimes"));}
    catch(e){setError(String(e));}
    finally{setBusy(false);}
  }
  useEffect(()=>{
    void refresh();
    const sub=listen<Progress>("java-install-progress",(event)=>setProgress(event.payload));
    return ()=>{void sub.then(stop=>stop());};
  },[]);

  async function choose(runtime:Runtime) {
    if(!instanceId){setError("Спочатку обери збірку у FRE INSTANCES.");return;}
    setBusy(true);setError("");setMessage("");
    try {
      const list=await invoke<Instance[]>("list_instances");
      const instance=list.find(item=>item.id===instanceId);
      if(!instance)throw Error("Обрана збірка не знайдена");
      await invoke("update_instance",{
        id:instance.id,name:instance.name,favorite:instance.favorite,
        memoryMb:instance.memoryMb,javaPath:runtime.path
      });
      setMessage(`Для збірки «${instance.name}» обрано Java ${runtime.major}.`);
    }catch(e){setError(String(e));}
    finally{setBusy(false);}
  }

  async function install() {
    setInstalling(true);setError("");setMessage("");setProgress(null);
    try {
      const executable=await invoke<string>("install_managed_java",{major:version});
      await refresh();
      setMessage(`Eclipse Temurin ${version} встановлено у: ${executable}`);
      if(instanceId) {
        // The new runtime is validated before use; selecting it requires a real instance.
        const runtime={path:executable,major:version,version:`Eclipse Temurin ${version}`};
        await choose(runtime);
      }
    }catch(e){setError(String(e));}
    finally{setInstalling(false);}
  }

  const percent=progress&&progress.total>0
    ? Math.min(100,Math.round(progress.done/progress.total*100)):0;

  return <div className="java-panel">
    <div className="panel-heading"><h3>FRE JAVA MANAGER</h3><Coffee size={18}/></div>
    <p>Пошук установлених Java у PATH, JAVA_HOME, Minecraft Launcher та сховищі FRE LAUNCH. Кожна збірка може використовувати свою Java.</p>
    <button className="secondary" disabled={busy||installing} onClick={()=>void refresh()}><RefreshCw size={15}/> Оновити список Java</button>
    {error&&<div className="error">{error}</div>}
    {message&&<div className="notice"><CheckCircle2 size={15}/>{message}</div>}
    <div className="java-list">
      {runtimes.map(runtime=><div className="java-row" key={runtime.path}>
        <span className="java-major">{runtime.major}</span>
        <div><b>{runtime.version}</b><small>{runtime.path}</small></div>
        <button className="secondary" disabled={busy||installing||!instanceId} onClick={()=>void choose(runtime)}>Обрати</button>
      </div>)}
    </div>
    {!busy&&runtimes.length===0&&<p className="muted">Java не знайдено на цьому пристрої.</p>}
    <div className="managed-jre">
      <div className="panel-heading"><h3>Встановити Java автоматично</h3><DownloadCloud size={19}/></div>
      <p>Завантаження Eclipse Temurin для Windows x64, із перевіркою SHA-256. Жодних прав адміністратора не потрібно.</p>
      <div className="toolbar">
        <select aria-label="Версія Java" value={version} onChange={e=>setVersion(Number(e.target.value))}>
          {jreVersions.map(major=><option value={major} key={major}>Java {major}</option>)}
        </select>
        <button className="primary" disabled={installing||busy} onClick={()=>void install()}><DownloadCloud size={16}/>{installing?"Встановлення...":"Завантажити Java"}</button>
      </div>
      {installing&&progress&&<div className="progress-panel"><div><b>{progress.message}</b><span>{percent}%</span></div>
        <div className="progress-bar"><div style={{width:percent+"%"}}/></div></div>}
      <p className="muted">Вибирай ту саму Java, яку вимагає твоя версія Minecraft. Для старих релізів часто потрібна Java 8, для сучасних — інша версія.</p>
    </div>
    {!instanceId&&<p className="muted">Щоб прив'язати Java до збірки, спочатку обери її у «Збірках».</p>}
  </div>;
}
