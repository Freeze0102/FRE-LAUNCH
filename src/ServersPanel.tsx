import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Server, Plus, RefreshCw, Star, Pencil, Trash2, Wifi, WifiOff, Copy, Check, X } from "lucide-react";

type SavedServer = { id:string; name:string; address:string; port:number; favorite:boolean };
type ServerStatus = { online:boolean; version:string|null; playersOnline:number|null; maxPlayers:number|null; description:string|null; latencyMs:number|null; error:string|null };

export default function ServersPanel(){
  const [servers,setServers]=useState<SavedServer[]>([]);
  const [statuses,setStatuses]=useState<Record<string,ServerStatus>>({});
  const [checking,setChecking]=useState<Record<string,boolean>>({});
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState("");
  const [notice,setNotice]=useState("");
  const [editing,setEditing]=useState<string|null>(null);
  const [name,setName]=useState("");
  const [address,setAddress]=useState("");
  const [port,setPort]=useState("25565");
  async function refresh(){
    setBusy(true);
    try{setServers(await invoke<SavedServer[]>("list_servers"));setError("");}
    catch(e){setError(String(e));}
    finally{setBusy(false);}
  }
  useEffect(()=>{void refresh();},[]);
  function resetForm(){setEditing(null);setName("");setAddress("");setPort("25565");}
  async function save(){
    const num=Number(port);
    if(!name.trim()||!address.trim()||!Number.isInteger(num)||num<1||num>65535){
      setError("Вкажи назву, правильну адресу та порт 1–65535.");return;
    }
    const old=editing?servers.find(s=>s.id===editing):undefined;
    setBusy(true);setError("");setNotice("");
    try{
      const item=await invoke<SavedServer>("save_server",{id:editing,name:name.trim(),address:address.trim(),port:num,favorite:old?.favorite??false});
      resetForm();await refresh();setNotice("Сервер збережено.");void ping(item);
    }catch(e){setError(String(e));}
    finally{setBusy(false);}
  }
  async function ping(s:SavedServer){
    setChecking(prev=>({...prev,[s.id]:true}));
    try{
      const result=await invoke<ServerStatus>("check_server_status",{address:s.address,port:s.port});
      setStatuses(prev=>({...prev,[s.id]:result}));
    }catch(e){setStatuses(prev=>({...prev,[s.id]:{online:false,version:null,playersOnline:null,maxPlayers:null,description:null,latencyMs:null,error:String(e)}}));}
    finally{setChecking(prev=>({...prev,[s.id]:false}));}
  }
  async function favorite(s:SavedServer){
    try{
      await invoke("save_server",{id:s.id,name:s.name,address:s.address,port:s.port,favorite:!s.favorite});
      await refresh();
    }catch(e){setError(String(e));}
  }
  async function remove(s:SavedServer){
    if(!window.confirm("Видалити сервер «"+s.name+"» зі списку?"))return;
    try{
      await invoke("delete_server",{id:s.id});
      setStatuses(prev=>{const next={...prev};delete next[s.id];return next;});
      await refresh();
    }catch(e){setError(String(e));}
  }
  function edit(s:SavedServer){setEditing(s.id);setName(s.name);setAddress(s.address);setPort(String(s.port));}
  async function copy(s:SavedServer){
    const host=s.address.includes(":")?"["+s.address+"]":s.address;
    try{await navigator.clipboard.writeText(host+":"+s.port);setNotice("Адресу скопійовано.");}
    catch{setError("Не вдалося скопіювати адресу.");}
  }
  return <section className="panel">
    <div className="panel-heading"><h3>Ігрові сервери Minecraft</h3><Server size={21}/></div>
    <p>Сервери, на яких ти граєш: BedWars, SkyWars, Survival, MiniGames тощо. Додавай адресу, перевіряй онлайн, пінг та кількість гравців. Це не хостинг і не оренда серверів.</p>
    <div className="server-form">
      <div className="form-field"><label>Назва</label><input className="input" maxLength={64} placeholder="Мій сервер" value={name} onChange={e=>setName(e.target.value)}/></div>
      <div className="form-field"><label>Адреса</label><input className="input" maxLength={253} placeholder="play.example.net" value={address} onChange={e=>setAddress(e.target.value)}/></div>
      <div className="form-field short"><label>Порт</label><input className="input" inputMode="numeric" maxLength={5} value={port} onChange={e=>setPort(e.target.value.replace(/[^0-9]/g,""))}/></div>
      <button className="primary" disabled={busy||!name.trim()||!address.trim()} onClick={()=>void save()}>{editing?<Check size={17}/>:<Plus size={17}/>} {editing?"Зберегти":"Додати"}</button>
      {editing&&<button className="icon-button" title="Скасувати" onClick={resetForm}><X size={18}/></button>}
    </div>
    {error&&<div className="error">{error}</div>}
    {notice&&<div className="notice">{notice}</div>}
    {!servers.length&&<div className="small-empty"><Server size={34}/><h3>Поки немає серверів</h3><p>Додай адресу вище, щоб бачити стан сервера.</p></div>}
    <div className="server-grid">{servers.map(s=>{
      const state=statuses[s.id],pending=checking[s.id];
      return <article className="server-card" key={s.id}>
        <div className="server-top"><div className="server-avatar"><Server size={24}/></div><div className="server-name"><b>{s.name}</b><small>{s.address}:{s.port}</small></div><button className={"icon-button "+(s.favorite?"starred":"")} onClick={()=>void favorite(s)} title="Обране"><Star size={18} fill={s.favorite?"currentColor":"none"}/></button></div>
        <div className="server-state">{pending?<><RefreshCw size={17}/> Перевірка...</>:state?.online?<><Wifi size={17}/><strong>Онлайн</strong><span>{state.latencyMs??"—"} мс</span></>:state?<><WifiOff size={17}/><strong>Офлайн / недоступний</strong></>:<span>Статус ще не перевірений</span>}</div>
        {state?.online&&<div className="server-details"><span>Версія: {state.version??"невідома"}</span><span>Гравці: {state.playersOnline??"?"}/{state.maxPlayers??"?"}</span>{state.description&&<p>{state.description}</p>}</div>}
        {state?.error&&<p className="server-error">{state.error}</p>}
        <div className="server-actions"><button className="secondary" disabled={pending} onClick={()=>void ping(s)}><RefreshCw size={15}/> Перевірити</button><button className="icon-button" title="Копіювати адресу" onClick={()=>void copy(s)}><Copy size={16}/></button><button className="icon-button" title="Редагувати" onClick={()=>edit(s)}><Pencil size={16}/></button><button className="icon-button danger" title="Видалити" onClick={()=>void remove(s)}><Trash2 size={16}/></button></div>
      </article>;
    })}</div>
    <p className="muted">Щоб підключитися до сервера, натисни кнопку копіювання адреси, запусти Minecraft і вибери «Мережева гра → Пряме підключення». Стан сервера визначається стандартним Java status ping; SRV-записи й захист іноді впливають на результат. Для входу може знадобитися твій Minecraft-акаунт або реєстрація на самому сервері.</p>
  </section>;
}
