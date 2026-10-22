import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Coffee, RefreshCw, CheckCircle2 } from "lucide-react";
import type { Instance } from "./Features";

type Runtime = {path:string;version:string;major:number};
export default function JavaPanel({instanceId}:{instanceId:string}) {
  const [runtimes,setRuntimes]=useState<Runtime[]>([]);
  const [busy,setBusy]=useState(false);
  const [message,setMessage]=useState("");
  const [error,setError]=useState("");
  async function refresh(){
    setBusy(true);setError("");
    try {setRuntimes(await invoke<Runtime[]>("list_java_runtimes"));}
    catch(e){setError(String(e));}
    finally{setBusy(false);}
  }
  useEffect(()=>{void refresh();},[]);
  async function choose(runtime:Runtime){
    if(!instanceId){setError("Спочатку обери збірку у FRE INSTANCES.");return;}
    setBusy(true);setError("");setMessage("");
    try {
      const list=await invoke<Instance[]>("list_instances");
      const instance=list.find(item=>item.id===instanceId);
      if(!instance){setError("Обрана збірка не знайдена.");return;}
      await invoke("update_instance", {
        id: instance.id,
        name: instance.name,
        favorite: instance.favorite,
        memoryMb: instance.memoryMb,
        javaPath: runtime.path
      });
      setMessage(`Для збірки «${instance.name}» обрано Java ${runtime.major}.`);
    } catch(e){setError(String(e));}
    finally{setBusy(false);}
  }
  return <div className="java-panel">
    <div className="panel-heading"><h3>FRE JAVA MANAGER</h3><Coffee size={18}/></div>
    <p>Автоматично знаходить Java у PATH, JAVA_HOME та поширених директоріях Windows, включно з Minecraft Launcher.</p>
    <button className="secondary" disabled={busy} onClick={()=>void refresh()}><RefreshCw size={15}/> Оновити список Java</button>
    {error&&<div className="error">{error}</div>}
    {message&&<div className="notice"><CheckCircle2 size={15}/>{message}</div>}
    <div className="java-list">
      {runtimes.map(runtime=><div className="java-row" key={runtime.path}>
        <span className="java-major">{runtime.major}</span>
        <div><b>{runtime.version}</b><small>{runtime.path}</small></div>
        <button className="secondary" disabled={busy || !instanceId} onClick={()=>void choose(runtime)}>Обрати</button>
      </div>)}
    </div>
    {!busy && runtimes.length===0 && <p className="muted">Java не знайдено. Встанови сумісну Java вручну.</p>}
    {!instanceId&&<p className="muted">Щоб прив'язати Java до збірки, спочатку обери її у «Збірках».</p>}
  </div>;
}
