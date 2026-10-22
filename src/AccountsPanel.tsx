import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UserRound, Plus, Trash2, ShieldCheck, UserCheck } from "lucide-react";

type Account = {id:string;nickname:string;offlineUuid:string};
export default function AccountsPanel({nickname,onSelect}:{nickname:string;onSelect:(name:string)=>void}){
  const [accounts,setAccounts]=useState<Account[]>([]);
  const [name,setName]=useState("");
  const [error,setError]=useState("");
  const [busy,setBusy]=useState(false);
  async function refresh(){
    try {setAccounts(await invoke<Account[]>("list_offline_accounts"));setError("");}
    catch(e){setError(String(e));}
  }
  useEffect(()=>{void refresh();},[]);
  async function add(){
    setBusy(true);setError("");
    try{
      const newAccount=await invoke<Account>("create_offline_account",{nickname:name});
      onSelect(newAccount.nickname);
      setName("");
      await refresh();
    }catch(e){setError(String(e));}
    finally{setBusy(false);}
  }
  async function remove(account:Account){
    if(!window.confirm(`Видалити локальний акаунт «${account.nickname}»? Ця дія не видаляє світи Minecraft.`))return;
    setBusy(true);setError("");
    try{await invoke("delete_offline_account",{id:account.id});
      if(nickname===account.nickname)onSelect("Player");
      await refresh();
    }catch(e){setError(String(e));}
    finally{setBusy(false);}
  }
  return <section className="panel">
    <div className="panel-heading"><h3>FRE ACCOUNTS</h3><UserRound size={20}/></div>
    <p>Додавай локальні офлайн-профілі. Вони зберігаються в SQLite, мають стабільний UUID і швидко перемикаються.</p>
    <div className="toolbar create-form">
      <input className="input" value={name} maxLength={16} onChange={e=>setName(e.target.value.replace(/[^a-zA-Z0-9_]/g,""))} placeholder="Новий Minecraft-нікнейм"/>
      <button className="primary" onClick={()=>void add()} disabled={!name || busy}><Plus size={17}/> Додати акаунт</button>
    </div>
    {error && <div className="error">{error}</div>}
    <div className="account-grid">
      {accounts.map(a=><article className={"account-card"+(nickname===a.nickname?" chosen":"")} key={a.id}>
        <div className="account-avatar">{a.nickname[0]?.toUpperCase()}</div>
        <div className="account-details"><b>{a.nickname}</b><span>UUID: {a.offlineUuid}</span></div>
        <button className="secondary" disabled={nickname===a.nickname} onClick={()=>onSelect(a.nickname)}><UserCheck size={15}/>{nickname===a.nickname?"Активний":"Обрати"}</button>
        <button className="icon-button danger" disabled={busy} onClick={()=>void remove(a)} title="Видалити локальний профіль"><Trash2 size={17}/></button>
      </article>)}
    </div>
    {accounts.length===0&&<div className="small-empty"><UserRound size={32}/><h3>Додай свій перший акаунт</h3><p>Локальні профілі працюють з офлайн-сумісними середовищами.</p></div>}
    <p className="muted"><ShieldCheck size={14}/> Microsoft OAuth ще не реалізовано. Офлайн-профіль не підтверджує придбання Minecraft і не обходить серверну авторизацію.</p>
  </section>;
}
