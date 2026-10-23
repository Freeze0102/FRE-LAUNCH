import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Image, RefreshCw, Trash2, X, Camera, Download } from "lucide-react";

type Screenshot = {filename:string;sizeBytes:number;modifiedAt:number};

export default function ScreenshotsPanel({instanceId}:{instanceId:string}) {
  const [items,setItems]=useState<Screenshot[]>([]);
  const [previews,setPreviews]=useState<Record<string,string>>({});
  const [visible,setVisible]=useState(12);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState("");
  const [lightbox,setLightbox]=useState<string|null>(null);

  async function refresh(){
    if(!instanceId){setItems([]);setPreviews({});return;}
    setBusy(true);setError("");
    try{
      const data=await invoke<Screenshot[]>("list_screenshots",{instanceId});
      setItems(data);setVisible(12);setPreviews({});
    }catch(err){setError(String(err));}
    finally{setBusy(false);}
  }
  useEffect(()=>{void refresh();setLightbox(null);},[instanceId]);
  useEffect(()=>{
    if(!instanceId)return;
    let alive=true;
    const files=items.slice(0,visible).filter(file=>!previews[file.filename]);
    // Throttle expensive base64 conversions to limit WebView memory pressure.
    const load=async()=>{
      for(const file of files) {
        if(!alive)break;
        try {
          const data=await invoke<string>("read_screenshot",{instanceId,filename:file.filename});
          if(alive)setPreviews(prev=>({...prev,[file.filename]:data}));
        } catch{/* Keep filename visible even when a PNG cannot be previewed. */}
      }
    };
    void load();
    return()=>{alive=false;};
  },[items,visible,instanceId]);

  useEffect(()=>{
    if(!lightbox)return;
    const onKey=(event:KeyboardEvent)=>{if(event.key==="Escape")setLightbox(null);};
    window.addEventListener("keydown",onKey);
    return ()=>window.removeEventListener("keydown",onKey);
  },[lightbox]);

  async function remove(file:Screenshot){
    if(!instanceId || !window.confirm(`Видалити скріншот «${file.filename}» назавжди?`))return;
    setBusy(true);setError("");
    try{await invoke("delete_screenshot",{instanceId,filename:file.filename});await refresh();}
    catch(err){setError(String(err));}
    finally{setBusy(false);}
  }

  return <section className="panel">
    <div className="panel-heading"><h3>FRE SCREENSHOT GALLERY</h3><Camera size={20}/></div>
    <p>Скріншоти з вибраної Minecraft-збірки. Попередній перегляд працює локально, файли нікуди не завантажуються.</p>
    <button className="secondary screenshot-refresh" disabled={busy||!instanceId} onClick={()=>void refresh()}><RefreshCw size={16}/> Оновити галерею</button>
    {error&&<div className="error">{error}</div>}
    {!instanceId&&<div className="small-empty"><Camera size={36}/><h3>Спочатку вибери збірку</h3><p>Зайди у «Збірки» та обери Minecraft-інстанс.</p></div>}
    {!!instanceId&&items.length===0&&<div className="small-empty"><Image size={35}/><h3>Скріншотів поки немає</h3><p>У Minecraft натисни F2, щоб зберегти зображення.</p></div>}
    <div className="screenshot-grid">
      {items.slice(0,visible).map(file=><article className="screenshot-card" key={file.filename}>
        <button className="screenshot-preview" disabled={!previews[file.filename]} onClick={()=>setLightbox(file.filename)} title="Переглянути">
          {previews[file.filename]?<img src={previews[file.filename]} alt={"Minecraft screenshot "+file.filename}/>:<Image size={30}/>}
        </button>
        <div className="screenshot-meta">
          <b title={file.filename}>{file.filename}</b>
          <small>{new Date(file.modifiedAt*1000).toLocaleString("uk-UA")} · {(file.sizeBytes/1024/1024).toFixed(1)} МБ</small>
        </div>
        <button className="icon-button danger" onClick={()=>void remove(file)} disabled={busy} title="Видалити назавжди"><Trash2 size={16}/></button>
      </article>)}
    </div>
    {items.length>visible&&<button className="secondary screenshot-more" onClick={()=>setVisible(v=>v+12)}>Показати ще ({items.length-visible})</button>}
    {lightbox&&previews[lightbox]&&<div className="screenshot-overlay" role="dialog" aria-modal="true" aria-label="Попередній перегляд скріншота">
      <button className="screenshot-overlay-close" title="Закрити" onClick={()=>setLightbox(null)}><X size={20}/></button>
      <a className="screenshot-overlay-download" href={previews[lightbox]} download={lightbox} title="Зберегти PNG"><Download size={17}/> Зберегти PNG</a>
      <img src={previews[lightbox]} alt={lightbox}/>
      <span>{lightbox}</span>
    </div>}
  </section>;
}
