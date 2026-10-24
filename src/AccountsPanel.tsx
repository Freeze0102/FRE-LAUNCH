import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UserRound, Plus, Trash2, ShieldCheck, UserCheck, ExternalLink, LogIn, LogOut } from "lucide-react";

type Account = {id:string;nickname:string;offlineUuid:string};
export type MicrosoftAccount = {name:string;id:string};
type DeviceChallenge = {userCode:string;verificationUri:string;expiresIn:number;interval:number};
type PollStatus = {status:"pending"|"connected";account:MicrosoftAccount|null;interval:number;persisted?:boolean};
type BrowserLoginResult={account:MicrosoftAccount;persisted:boolean};
export default function AccountsPanel({nickname,onSelect,microsoft,onMicrosoftSelect}:{
  nickname:string;onSelect:(name:string)=>void;microsoft:MicrosoftAccount|null;onMicrosoftSelect:(account:MicrosoftAccount|null)=>void;
}){
  const [clientId,setClientId]=useState(()=>{
    try{return localStorage.getItem("fre-ms-client-id")||"";}catch{return "";}
  });
  const [challenge,setChallenge]=useState<DeviceChallenge|null>(null);
  const [hasBuiltInId,setHasBuiltInId]=useState(false);
  const [showDeveloperOptions,setShowDeveloperOptions]=useState(false);
  useEffect(()=>{void invoke<boolean>("microsoft_login_configuration").then(setHasBuiltInId).catch(()=>{});},[]);
  const [authBusy,setAuthBusy]=useState(false);
  const [authError,setAuthError]=useState("");
  const [authNotice,setAuthNotice]=useState("");
  const checking=useRef(false);
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
      const newAccount=await invoke<Account>("ensure_offline_account",{nickname:name});
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
  const canLogin=hasBuiltInId||(showDeveloperOptions&&/^[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}$/.test(clientId.trim()));
  async function beginBrowserLogin(){
    setAuthBusy(true);setAuthError("");setAuthNotice("");setChallenge(null);
    try{
      const result=await invoke<BrowserLoginResult>("microsoft_browser_login",{clientId:showDeveloperOptions?clientId.trim()||null:null});
      if(showDeveloperOptions)try{localStorage.setItem("fre-ms-client-id",clientId.trim());}catch{}
      onMicrosoftSelect(result.account);
      setAuthNotice(result.persisted?
        "Minecraft Java підключено через Microsoft. Вхід збережено в Windows Credential Manager.":
        "Minecraft Java підключено, але Windows не підтвердила збереження входу. Можливо, знадобиться повторна авторизація.");
    }catch(e){setAuthError(String(e));}
    finally{setAuthBusy(false);}
  }
  async function beginMicrosoft(){
    setAuthBusy(true);setAuthError("");setAuthNotice("");setChallenge(null);
    try{
      const code=await invoke<DeviceChallenge>("microsoft_begin_login",{clientId:showDeveloperOptions?clientId.trim()||null:null});
      if(showDeveloperOptions)try{localStorage.setItem("fre-ms-client-id",clientId.trim());}catch{/* public app identifier only */}
      setChallenge(code);
    }catch(e){setAuthError(String(e));}
    finally{setAuthBusy(false);}
  }
  async function logoutMicrosoft(){
    setAuthBusy(true);setAuthError("");
    try{await invoke("microsoft_logout");onMicrosoftSelect(null);setChallenge(null);}
    catch(e){setAuthError(String(e));}
    finally{setAuthBusy(false);}
  }
  useEffect(()=>{
    if(!challenge)return;
    let active=true;
    const ms=Math.max(5,Math.min(60,challenge.interval))*1000;
    const timer=window.setInterval(async()=>{
      if(checking.current)return;
      checking.current=true;
      try{
        const result=await invoke<PollStatus>("microsoft_poll_login");
        if(!active)return;
        if(result.status==="connected"&&result.account){
          setChallenge(null);
          onMicrosoftSelect(result.account);
          setAuthError("");
          setAuthNotice(result.persisted?
            "Minecraft-акаунт підключено. Windows збереже вхід для наступного запуску.":
            "Minecraft-акаунт підключено, але збереження сесії у Windows не підтверджене. Можливо, після перезапуску доведеться ввійти ще раз.");
        }
      }catch(err){
        if(active){setAuthError(String(err));setChallenge(null);}
      }finally{checking.current=false;}
    },ms);
    return()=>{active=false;window.clearInterval(timer);};
  },[challenge,onMicrosoftSelect]);
  return <section className="panel">
    <div className="panel-heading"><h3>Акаунти Minecraft</h3><UserRound size={20}/></div>
    <div className="fre-ms-account">
      <div className="fre-ms-head">
        <span className="fre-ms-logo">M</span>
        <div><h3>Microsoft · Minecraft Java Edition</h3><p>Справжній вхід через Microsoft → Xbox → Minecraft. Пароль вводиться тільки на сайті Microsoft.</p></div>
      </div>
      {microsoft?<><div className="fre-ms-signed"><UserCheck size={23}/><div><strong>{microsoft.name}</strong><small>Microsoft Minecraft · {microsoft.id}</small></div><button className="secondary" disabled={authBusy} onClick={()=>void logoutMicrosoft()}><LogOut size={15}/> Вийти</button></div>{authNotice&&<p className="notice">{authNotice}</p>}</>:
        <div className="fre-ms-login">
          <div className="fre-ms-form">
            <button className="primary" disabled={authBusy||!canLogin} onClick={()=>void beginBrowserLogin()}>
              <LogIn size={16}/> {authBusy?"Очікуємо підтвердження…":"Увійти через Microsoft"}
            </button>
            <button className="secondary" disabled={authBusy||!canLogin} onClick={()=>void beginMicrosoft()}>Увійти за кодом</button>
            <button className="secondary" disabled={authBusy} onClick={()=>setShowDeveloperOptions(v=>!v)}>Для розробника</button>
          </div>
          {authBusy&&<p className="notice" role="status">Відкривається системний браузер Microsoft. Увійди в потрібний акаунт і повернися в FRE LAUNCH. Підтвердження може тривати до 5 хвилин.</p>}
          {hasBuiltInId?
            <p className="muted">Вхід відкриває офіційний сайт Microsoft у браузері й автоматично завершується у FRE LAUNCH. Якщо локальний браузер заблокований, обери «Увійти за кодом».</p>:
            <p className="muted">FRE LAUNCH ще не має власного схваленого Minecraft Client ID. Поки розробник не отримає дозвіл Minecraft Services та не додасть ID до збірки, ліцензійний Microsoft-вхід не запрацює. Локальний нікнейм можна використовувати окремо.</p>}
          {showDeveloperOptions&&<div className="fre-ms-dev-options">
            <label className="field-label">Власний Minecraft-схвалений Application Client ID</label>
            <input className="input" value={clientId} onChange={e=>setClientId(e.target.value)} placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" autoComplete="off"/>
            <p className="muted">Тільки для власного схваленого Minecraft Client ID. Для входу через браузер додай в Microsoft Entra платформу «Mobile and desktop applications» з redirect URI http://localhost. Вхід за кодом не потребує redirect URI. Звичайний Client ID без дозволу Minecraft Services може отримати HTTP 403.</p>
          </div>}
          {challenge&&<div className="fre-ms-challenge">
            <span>Підтвердь вхід у Microsoft</span><strong>{challenge.userCode}</strong>
            <p>Відкрий офіційний сайт Microsoft, введи цей код і підтвердь вхід. Перевірка виконується автоматично.</p>
            <a href="https://www.microsoft.com/link" target="_blank" rel="noopener noreferrer">Відкрити Microsoft <ExternalLink size={15}/></a>
            <button className="secondary" onClick={()=>setChallenge(null)}>Приховати код</button>
          </div>}
          {authError&&<div className="error">{authError}</div>}
          <div className="fre-ms-links">
            <a href="https://signup.live.com/" target="_blank" rel="noopener noreferrer">Створити Microsoft-акаунт <ExternalLink size={12}/></a>
            <a href="https://entra.microsoft.com/" target="_blank" rel="noopener noreferrer">Реєстрація застосунку (для розробника) <ExternalLink size={12}/></a>
            <a href="https://help.minecraft.net/hc/en-us/articles/16254801392141" target="_blank" rel="noopener noreferrer">Дозвіл Minecraft на Client ID <ExternalLink size={12}/></a>
          </div>
        </div>}
      <p className="fre-ms-caution">Пароль не потрапляє в FRE LAUNCH. Токен Minecraft не передається інтерфейсу. Під час успішного входу лаунчер намагається зберегти ключ оновлення в Windows Credential Manager; при наступному запуску FRE LAUNCH спробує автоматично відновити Minecraft-сесію. Для входу потрібен схвалений Minecraft Client ID.</p>
    </div>
    <h3 className="fre-offline-title">Офлайн-профілі</h3>
    <p>Додавай локальні імена Minecraft. Вони не є Microsoft-акаунтами та не дають ліцензійного доступу.</p>
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
    <p className="muted"><ShieldCheck size={14}/> Це лише локальні офлайн-профілі — не вхід у Microsoft/Minecraft. Нікнейм сам по собі не дає доступу до твого ліцензійного акаунта. На серверах з окремою реєстрацією (/register або /login) пароль і прив'язка профілю визначаються самим сервером. Якщо нік уже зайнятий, не намагайся входити в чужий обліковий запис.</p>
  </section>;
}
