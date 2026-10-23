import {useEffect,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {Archive, Box, ChevronRight, FileCog, Folder, Image, Layers, Package, RefreshCw, ScrollText, ShieldCheck, Wrench} from "lucide-react";

type FolderInfo={key:string;name:string;path:string;exists:boolean};
const icons:Record<string,typeof Folder>={
  root:Folder,mods:Package,saves:Archive,resourcepacks:Layers,
  shaderpacks:Image,screenshots:Image,config:Wrench,logs:ScrollText,datapacks:FileCog
};

export default function GameFolders({instanceId,version}: {instanceId:string;version:string}) {
  const [items,setItems]=useState<FolderInfo[]>([]);
  const [loading,setLoading]=useState(false);
  const [opening,setOpening]=useState("");
  const [error,setError]=useState("");
  const [notice,setNotice]=useState("");
  async function refresh(){
    if(!version){setItems([]);return;}
    setLoading(true);setError("");
    try{setItems(await invoke<FolderInfo[]>("list_game_folders",{instanceId:instanceId||null,version}));}
    catch(e){setError(String(e));}finally{setLoading(false);}
  }
  useEffect(()=>{void refresh();},[instanceId,version]);
  async function open(kind:string){
    setOpening(kind);setError("");setNotice("");
    try{
      const dir=await invoke<string>("open_game_folder",{instanceId:instanceId||null,version,kind});
      setNotice("Відкрито у Провіднику Windows: "+dir);
    }catch(e){setError(String(e));}finally{setOpening("");}
  }
  return <section className="fre-files-panel" aria-label="Папки Minecraft">
    <div className="fre-files-header">
      <div><span className="fre-files-eyebrow"><Box size={15}/> FILE MANAGER</span><h3>Файли Minecraft</h3><p>Кожна збірка має свої світи, моди й налаштування. Жодні файли не переміщуються.</p></div>
      <button className="fre-files-refresh" onClick={()=>void refresh()} disabled={loading||!version}><RefreshCw size={16}/> Оновити</button>
    </div>
    {!instanceId&&<div className="fre-files-hint"><ShieldCheck size={15}/> Папки поточної версії. Для окремих модів і світів радимо створити збірку в бібліотеці.</div>}
    <div className="fre-files-grid">
      {items.map(folder=>{
        const Icon=icons[folder.key]||Folder;
        return <button key={folder.key} className={"fre-file-tile"+(folder.key==="root"?" fre-file-tile-root":"")} onClick={()=>void open(folder.key)} disabled={!!opening}>
          <span className="fre-file-icon"><Icon size={20}/></span>
          <span className="fre-file-details"><strong>{folder.name}</strong><small title={folder.path}>{folder.path}</small></span>
          <ChevronRight size={18}/>
        </button>;
      })}
    </div>
    {loading&&<p className="muted">Перевіряємо папки гри…</p>}
    {error&&<p className="error">{error}</p>}
    {notice&&<p className="notice">{notice}</p>}
  </section>;
}