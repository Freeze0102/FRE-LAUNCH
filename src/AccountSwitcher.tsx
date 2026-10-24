import {useEffect,useRef,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {Check,ChevronRight,LogIn,Plus,ShieldCheck,UserRound,X} from "lucide-react";
import "./account-switcher.css";

type OfflineAccount={id:string;nickname:string;offlineUuid:string};
type MicrosoftAccount={name:string;id:string};
type Props={
  nickname:string;isMicrosoft:boolean;microsoft:MicrosoftAccount|null;
  onSelectOffline:(nickname:string)=>void;
  onSelectMicrosoft:()=>void;
  onAccounts:()=>void;
};
export default function AccountSwitcher({nickname,isMicrosoft,microsoft,onSelectOffline,onSelectMicrosoft,onAccounts}:Props){
  const [open,setOpen]=useState(false);
  const [accounts,setAccounts]=useState<OfflineAccount[]>([]);
  const [name,setName]=useState("");
  const [error,setError]=useState("");
  const [busy,setBusy]=useState(false);
  const ref=useRef<HTMLDivElement>(null);
  useEffect(()=>{
    if(!open)return;
    let active=true;
    void invoke<OfflineAccount[]>("list_offline_accounts").then(items=>{if(active)setAccounts(items);})
      .catch(e=>{if(active)setError(String(e));});
    const keydown=(event:KeyboardEvent)=>{if(event.key==="Escape")setOpen(false);};
    const close=(event:PointerEvent)=>{if(ref.current&&!ref.current.contains(event.target as Node))setOpen(false);};
    window.addEventListener("keydown",keydown);
    window.addEventListener("pointerdown",close);
    return()=>{active=false;window.removeEventListener("keydown",keydown);window.removeEventListener("pointerdown",close);};
  },[open]);
  async function addByNickname(){
    const normalized=name.trim();
    if(!/^[A-Za-z0-9_]{1,16}$/.test(normalized)){setError("Введи нік: 1–16 латинських літер, цифр або _");return;}
    setBusy(true);setError("");
    try{
      const account=await invoke<OfflineAccount>("ensure_offline_account",{nickname:normalized});
      setAccounts(old=>[...old.filter(x=>x.id!==account.id),account]);
      onSelectOffline(account.nickname);setName("");setOpen(false);
    }catch(e){setError(String(e));}
    finally{setBusy(false);}
  }
  return <div className="fre-switcher" ref={ref}>
    <button className="fre-command-account" onClick={()=>{setOpen(o=>!o);setError("");}}
      title="Вибрати або додати Minecraft-профіль" aria-haspopup="dialog" aria-expanded={open}>
      <span className="fre-avatar">{nickname[0]?.toUpperCase()||"P"}</span>
      <span className="fre-switcher-title"><strong>{nickname}</strong><small>{isMicrosoft&&microsoft?"Microsoft":"Локальний профіль"}</small></span>
      <ChevronRight size={15}/>
    </button>
    {open&&<div className="fre-switcher-menu" role="dialog" aria-label="Обрати Minecraft-акаунт">
      <div className="fre-switcher-header"><strong>Акаунти Minecraft</strong><button aria-label="Закрити" onClick={()=>setOpen(false)}><X size={16}/></button></div>
      {microsoft&&<button className={"fre-switcher-account"+(isMicrosoft?" active":"")}
        onClick={()=>{onSelectMicrosoft();setOpen(false);}}>
        <span className="fre-switcher-avatar microsoft"><ShieldCheck size={18}/></span>
        <span><strong>{microsoft.name}</strong><small>Minecraft Java · Microsoft</small></span>
        {isMicrosoft&&<Check size={17}/>}
      </button>}
      <div className="fre-switcher-category">Офлайн-профілі</div>
      {accounts.map(account=><button key={account.id}
        className={"fre-switcher-account"+(!isMicrosoft&&nickname===account.nickname?" active":"")}
        onClick={()=>{onSelectOffline(account.nickname);setOpen(false);}}>
        <span className="fre-switcher-avatar">{account.nickname[0]?.toUpperCase()}</span>
        <span><strong>{account.nickname}</strong><small>Локальний нікнейм</small></span>
        {!isMicrosoft&&nickname===account.nickname&&<Check size={17}/>}
      </button>)}
      <div className="fre-switcher-new">
        <label htmlFor="fre-quick-nick">Додати акаунт за ніком</label>
        <form onSubmit={event=>{event.preventDefault();void addByNickname();}}>
          <input id="fre-quick-nick" value={name} maxLength={16}
            onChange={e=>setName(e.target.value.replace(/[^A-Za-z0-9_]/g,""))}
            placeholder="Minecraft-нік" autoComplete="off"/>
          <button type="submit" disabled={busy||!name.trim()} aria-label="Додати локальний акаунт"><Plus size={18}/></button>
        </form>
        <p>Профіль створиться одразу. Це не Microsoft-ліцензія й не реєстрація на ігровому сервері.</p>
      </div>
      {error&&<p className="fre-switcher-error" role="alert">{error}</p>}
      <button className="fre-switcher-manage" onClick={()=>{setOpen(false);onAccounts();}}>
        <UserRound size={16}/> Керування акаунтами <ChevronRight size={15}/>
      </button>
      {!microsoft&&<button className="fre-switcher-microsoft" onClick={()=>{setOpen(false);onAccounts();}}>
        <LogIn size={16}/> Увійти через Microsoft
      </button>}
    </div>}
  </div>;
}
