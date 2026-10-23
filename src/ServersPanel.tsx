import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Globe2, Plus, RefreshCw, Trash2, Users, Wifi, WifiOff, Copy } from "lucide-react";

type Server = { id:string; name:string; host:string; port:number };
type Status = { online:boolean; pingMs:number;version:string;playersOnline:number;playersMax:number;description:string };

export default function ServersPanel() {
  const [servers,setServers]=useState<Server[]>([]);
  const [status,setStatus]=useState<Record<string,Status>>({});
  const [errors,setErrors]=useState<Record<string,string>>({});
  const [busy,setBusy]=useState(false);
  const [checking,setChecking]=useState<string[]>([]);
  const [name,setName]=useState("");
  const [host,setHost]=useState("");
  const [port,setPort]=useState("25565");
  const [error,setError]=useState("");

  async function refresh() {
    try {setServers(await invoke<Server[]>("list_servers"));setError("");}
    catch(err) {setError(String(err));}
  }
  useEffect(()=>{void refresh();},[]);

  async function add() {
    setBusy(true);setError("");
    try {
      const entry=await invoke<Server>("add_server",{
        name:name.trim(),host:host.trim(),port:Number(port)
      });
      setName("");setHost("");setPort("25565");
      await refresh();
      void ping(entry);
    } catch(err){setError(String(err));}
    finally {setBusy(false);}
  }
  async function remove(server:Server) {
    if(!window.confirm(`Видалити «${server.name}» зі списку серверів?`))return;
    setBusy(true);
    try {await invoke("delete_server",{id:server.id});await refresh();}
    catch(err){setError(String(err));}
    finally {setBusy(false);}
  }
  async function ping(server:Server) {
    setChecking(a=>a.includes(server.id)?a:[...a,server.id]);
    setErrors(a=>({...a,[server.id]:""}));
    try {
      const response=await invoke<Status>("ping_server",{host:server.host,port:server.port});
      setStatus(a=>({...a,[server.id]:response}));
    } catch(err) {
      setStatus(a=>{const copy={...a};delete copy[server.id];return copy;});
      setErrors(a=>({...a,[server.id]:String(err)}));
    } finally {setChecking(a=>a.filter(id=>id!==server.id));}
  }
  return <section className="panel">
    <div className="panel-heading"><h3>FRE SERVERS</h3><Globe2 size={20}/></div>
    <p>Збережені сервери Minecraft Java Edition. Перевіряй доступність, версію, кількість гравців і затримку через справжній статус-протокол Minecraft.</p>
    <div className="toolbar create-form">
      <input className="input" maxLength={50} value={name} onChange={e=>setName(e.target.value)} placeholder="Назва сервера"/>
      <input className="input" maxLength={253} value={host} onChange={e=>setHost(e.target.value)} placeholder="Адреса: play.example.net"/>
      <input className="input server-port" inputMode="numeric" value={port} onChange={e=>setPort(e.target.value.replace(/[^0-9]/g,""))} placeholder="Порт"/>
      <button className="primary" disabled={busy || !name.trim() || !host.trim() || !Number.isInteger(Number(port)) || Number(port)<1 || Number(port)>65535} onClick={()=>void add()}><Plus size={16}/> Додати</button>
    </div>
    {error&&<div className="error">{error}</div>}
    <div className="server-grid">
      {servers.map(server=>{
        const state=status[server.id];
        const inProgress=checking.includes(server.id);
        return <article className="server-card" key={server.id}>
          <div className="server-badge"><Globe2 size={24}/></div>
          <div className="server-info">
            <strong>{server.name}</strong>
            <span>{server.host}:{server.port}</span>
            {inProgress?<span>Перевірка сервера...</span>:state?.online?<div className="server-stat"><span><Wifi size={14}/> Онлайн · {state.pingMs} мс</span><span><Users size={14}/> {state.playersOnline}/{state.playersMax}</span><span>{state.version}</span></div>:<span className="server-offline"><WifiOff size={13}/> {errors[server.id]||"Статус ще не перевірений"}</span>}
            {state?.description&&<small>{state.description}</small>}
          </div>
          <button className="icon-button" title="Копіювати адресу" onClick={()=>void navigator.clipboard.writeText(`${server.host}:${server.port}`)}><Copy size={16}/></button>
          <button className="icon-button" disabled={inProgress} title="Оновити статус" onClick={()=>void ping(server)}><RefreshCw size={16}/></button>
          <button className="icon-button danger" disabled={busy} title="Видалити" onClick={()=>void remove(server)}><Trash2 size={16}/></button>
        </article>;
      })}
    </div>
    {servers.length===0&&<div className="small-empty"><Globe2 size={32}/><h3>Додай свій перший сервер</h3><p>Сервери зберігаються на цьому пристрої. Лаунчер не обходить перевірку ліцензії.</p></div>}
    <p className="muted">FRE LAUNCH не відправляє твій Microsoft-пароль серверам. Для входу на захищені сервери потрібна офіційна авторизація Minecraft.</p>
  </section>;
}
