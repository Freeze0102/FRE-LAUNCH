import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Images, Image, RefreshCw, Download, Trash2, FileImage, AlertTriangle } from "lucide-react";

type Screenshot = { filename:string; sizeBytes:number; modifiedAt:number };
export default function ScreenshotGallery({instanceId}:{instanceId:string}) {
  const [items,setItems]=useState<Screenshot[]>([]);
  const [filename,setFilename]=useState("");
  const [preview,setPreview]=useState("");
  const [busy,setBusy]=useState(false);
  const [previewBusy,setPreviewBusy]=useState(false);
  const [error,setError]=useState("");
  const [message,setMessage]=useState("");

  async function refresh() {
    if(!instanceId){setItems([]);setFilename("");setPreview("");return;}
    setBusy(true);setError("");
    try {
      const list=await invoke<Screenshot[]>("list_screenshots",{instanceId});
      setItems(list);
      if(!list.some(x=>x.filename===filename)){
        setFilename(list[0]?.filename??"");
        setPreview("");
      }
    } catch(err){setError(String(err));}
    finally {setBusy(false);}
  }
  useEffect(()=>{void refresh();},[instanceId]);
  useEffect(()=>{
    if(!instanceId||!filename){setPreview("");return;}
    let cancelled=false;
    setPreviewBusy(true);setError("");
    void invoke<string>("read_screenshot",{instanceId,filename})
      .then(data=>{if(!cancelled)setPreview(data);})
      .catch(err=>{if(!cancelled){setError(String(err));setPreview("");}})
      .finally(()=>{if(!cancelled)setPreviewBusy(false);});
    return ()=>{cancelled=true;};
  },[filename,instanceId]);

  async function remove(){
    if(!instanceId||!filename||!window.confirm("Безповоротно видалити скриншот «"+filename+"»?"))return;
    setBusy(true);setError("");setMessage("");
    try {
      await invoke("delete_screenshot",{instanceId,filename});
      setMessage("Скриншот видалено.");
      setFilename("");setPreview("");
      await refresh();
    } catch(err){setError(String(err));}
    finally {setBusy(false);}
  }

  return <section className="panel screenshot-page">
    <div className="panel-heading"><h3>FRE SCREENSHOT GALLERY</h3><Images size={22}/></div>
    <p>Перегляд скриншотів Minecraft із папки вибраної збірки. Можна зберігати PNG на пристрій або видаляти непотрібні знімки.</p>
    {!instanceId&&<div className="small-empty"><FileImage size={35}/><h3>Спочатку обери збірку</h3><p>Відкрий «Збірки» та вибери ту, де збережено скриншоти.</p></div>}
    {!!instanceId&&<>
      <div className="gallery-toolbar">
        <span>{items.length} скриншотів</span>
        <button className="secondary" disabled={busy} onClick={()=>void refresh()}><RefreshCw size={16}/> Оновити</button>
        {filename&&preview&&<a className="secondary" href={preview} download={filename}><Download size={16}/> Зберегти PNG</a>}
        {filename&&<button className="secondary gallery-delete" disabled={busy} onClick={()=>void remove()}><Trash2 size={16}/> Видалити</button>}
      </div>
      {error&&<div className="error"><AlertTriangle size={17}/>{error}</div>}
      {message&&<div className="notice">{message}</div>}
      {!items.length&&<div className="small-empty"><Image size={34}/><h3>Скриншотів ще немає</h3><p>У грі натисни F2, щоб створити знімок. Після цього онови галерею.</p></div>}
      {!!items.length&&<div className="gallery-layout">
        <div className="gallery-preview">
          {previewBusy?<div className="preview-empty">Відкриваю зображення...</div>:preview?<img src={preview} alt={"Скриншот "+filename}/>:<div className="preview-empty">Обери скриншот зі списку</div>}
          {filename&&<div className="gallery-caption"><b>{filename}</b><small>{items.find(x=>x.filename===filename)?.sizeBytes ? (items.find(x=>x.filename===filename)!.sizeBytes/1048576).toFixed(2)+" МБ" : ""}</small></div>}
        </div>
        <div className="gallery-items" role="list" aria-label="Скриншоти Minecraft">
          {items.map(item=><button role="listitem" className={"gallery-item"+(filename===item.filename?" active":"")} key={item.filename} onClick={()=>setFilename(item.filename)}>
            <div className="gallery-file-icon"><FileImage size={21}/></div>
            <div><b>{item.filename}</b><small>{new Date(item.modifiedAt*1000).toLocaleString("uk-UA")} · {(item.sizeBytes/1048576).toFixed(2)} МБ</small></div>
          </button>)}
        </div>
      </div>}
      <p className="muted">Лаунчер читає лише PNG-файли в папці скриншотів вибраної збірки. Інші папки комп'ютера не скануються.</p>
    </>}
  </section>;
}
