import { useEffect, useRef, useState } from "react";
import { SkinViewer, WalkingAnimation, RunningAnimation } from "skinview3d";
import { ImagePlus, RotateCcw, Footprints, Pause, Play, Upload, Plus, Trash2, Check } from "lucide-react";
import SkinPixelEditor from "./SkinPixelEditor";
import {builtInSkin} from "./defaultSkin";

type Animate = "idle" | "walk" | "run";
const STORAGE_SKIN = "fre-launch:skin-preview";
const STORAGE_GALLERY = "fre-launch:skin-gallery";
type SavedSkin={id:string;name:string;image:string};
function storedSkins():SavedSkin[]{
  try {
    const raw=localStorage.getItem(STORAGE_GALLERY);
    if(raw){
      const value:unknown=JSON.parse(raw);
      if(Array.isArray(value))return value.filter((item):item is SavedSkin=>
        typeof item?.id==="string" && typeof item?.name==="string" && typeof item?.image==="string" && item.image.startsWith("data:image/png;base64,"))
        .slice(0,10);
    }
    const legacy=localStorage.getItem(STORAGE_SKIN);
    return legacy?.startsWith("data:image/png;base64,")?[{id:"imported-legacy",name:"Мій скін",image:legacy}]:[];
  }catch{return [];}
}


export default function SkinStudio() {
  const canvas = useRef<HTMLCanvasElement>(null);
  const viewer = useRef<SkinViewer | null>(null);
  const [skin, setSkin] = useState<string>(() => localStorage.getItem(STORAGE_SKIN) || builtInSkin());
  const [custom, setCustom] = useState<boolean>(() => !!localStorage.getItem(STORAGE_SKIN));
  const [cape, setCape] = useState<string>("");
  const [savedSkins,setSavedSkins]=useState<SavedSkin[]>(storedSkins);
  const [animation, setAnimation] = useState<Animate>("walk");
  const [spin, setSpin] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!canvas.current) return;
    let current: SkinViewer;
    try {
      current = new SkinViewer({
        canvas: canvas.current,
        width: 340,
        height: 440,
        skin: skin || builtInSkin()
      });
      current.autoRotate = true;
      current.animation = new WalkingAnimation();
      viewer.current = current;
    } catch {
      setError("3D-перегляд недоступний у поточному WebView2. Можна відкрити або завантажити PNG-скін.");
      return;
    }
    return () => {
      current.dispose();
      viewer.current = null;
    };
  }, []);

  useEffect(() => {
    if (viewer.current && skin) {
      void viewer.current.loadSkin(skin).catch((err: unknown) => setError(String(err)));
    }
  }, [skin]);
  useEffect(() => {
    if (!viewer.current) return;
    viewer.current.animation = animation === "walk" ? new WalkingAnimation() :
      animation === "run" ? new RunningAnimation() : null;
  }, [animation]);
  useEffect(() => {if (viewer.current) viewer.current.autoRotate = spin;}, [spin]);
  useEffect(() => {
    if (cape && viewer.current) void viewer.current.loadCape(cape).catch((err: unknown) => setError(String(err)));
  }, [cape]);

  function importImage(file: File | undefined, kind: "skin" | "cape") {
    if (!file) return;
    setError("");
    if (file.type !== "image/png" || file.size > 2 * 1024 * 1024) {
      setError("Обери PNG-файл не більший за 2 МБ.");
      return;
    }
    const image = new Image();
    const local = URL.createObjectURL(file);
    image.onload = () => {
      const width = image.naturalWidth, height = image.naturalHeight;
      URL.revokeObjectURL(local);
      const valid = kind === "skin"
        ? width >= 64 && width % 64 === 0 && (height === width || height === width / 2)
        : width > 0 && height > 0;
      if (!valid) {
        setError("Непідтримуваний розмір скіна. Потрібен формат 64×64 або 64×32 (можливі HD-множники).");
        return;
      }
      const reader = new FileReader();
      reader.onload = () => {
        const encoded = String(reader.result);
        if (kind === "skin") {
          applySkin(encoded, file.name.replace(/\.png$/i, "").slice(0,32) || "Мій скін");
        } else setCape(encoded);
      };
      reader.onerror = () => setError("Не вдалося прочитати PNG.");
      reader.readAsDataURL(file);
    };
    image.onerror = () => {URL.revokeObjectURL(local);setError("Пошкоджений PNG-файл.");};
    image.src = local;
  }

  function applySkin(dataUrl:string,name="Відредагований скін"){
    try {
      if(!dataUrl.startsWith("data:image/png;base64,"))throw Error("Некоректний PNG");
      localStorage.setItem(STORAGE_SKIN,dataUrl);
      const next=[{id:Date.now().toString(36)+"-"+Math.random().toString(36).slice(2),name,image:dataUrl},...savedSkins].slice(0,8);
      localStorage.setItem(STORAGE_GALLERY,JSON.stringify(next));
      setSavedSkins(next);
      setSkin(dataUrl);setCustom(true);setError("");
    }catch{setError("Не вдалося зберегти PNG у локальній галереї. Перевір вільне місце.");}
  }
  function selectSaved(item:SavedSkin){
    try{localStorage.setItem(STORAGE_SKIN,item.image);setSkin(item.image);setCustom(true);setError("");}
    catch{setError("Не вдалося застосувати локальний скін.");}
  }
  function deleteSaved(item:SavedSkin){
    if(!window.confirm(`Прибрати скін «${item.name}» із локальної галереї?`))return;
    try{
      const remaining=savedSkins.filter(entry=>entry.id!==item.id);
      localStorage.setItem(STORAGE_GALLERY,JSON.stringify(remaining));
      setSavedSkins(remaining);
      if(skin===item.image)restoreDefault();
    }catch{setError("Не вдалося оновити галерею скінів.");}
  }
  function restoreDefault(){
    localStorage.removeItem(STORAGE_SKIN);
    setSkin(builtInSkin());
    setCustom(false);
    setError("");
  }
  return <><div className="studio-grid">
    <section className="panel skin-stage">
      <div className="panel-heading"><h3>Перегляд скіна</h3><ImagePlus size={19}/></div>
      <div className="skin-canvas"><canvas ref={canvas} aria-label="3D-попередній перегляд скіна Minecraft"/>{error.includes("3D-перегляд недоступний")&&<img className="fre-skin-fallback" src={skin} alt="PNG-скін персонажа"/>}</div>
      <p className="muted fre-skin-preview-tip">Перетягни мишкою, щоб повернути персонажа. Коліщатко змінює масштаб.</p>
    </section>
    <section className="panel skin-tools fre-skin-gallery">
      <h3>Збережені скіни</h3>
      <div className="fre-skin-saved">
        <label className="fre-skin-gallery-card fre-skin-gallery-add"><Plus size={25}/><span>Додати скін</span><input type="file" accept="image/png" aria-label="Додати скін до галереї" onChange={e=>importImage(e.target.files?.[0],"skin")}/></label>
        {savedSkins.map(item=><div className={"fre-skin-gallery-card"+(custom&&skin===item.image?" active":"")} key={item.id}>
          <button onClick={()=>selectSaved(item)} title={"Обрати "+item.name}><img src={item.image} alt={item.name}/><span>{item.name}</span>{custom&&skin===item.image&&<Check size={16}/>}</button>
          <button className="fre-skin-gallery-delete" onClick={()=>deleteSaved(item)} title={"Видалити "+item.name} aria-label={"Видалити "+item.name}><Trash2 size={14}/></button>
        </div>)}
      </div>
      <h3>Стандартні скіни</h3>
      <div className="fre-skin-defaults">
        <button className={"fre-skin-gallery-card"+(!custom?" active":"")} onClick={restoreDefault}>
          <img src={builtInSkin()} alt="Стів — стандартний скін Minecraft"/><span>Стів</span>{!custom&&<Check size={16}/>}
        </button>
      </div>
      <h3>Налаштування скіна</h3>
      <div className="fre-skin-status">{custom?"Власний скін збережено":"Зараз стандартний скін"}</div>
      <div className="fre-skin-guide">
        <strong>1. Завантаж PNG</strong>
        <p>Вибери скін Minecraft 64×64 або 64×32. Побачиш його одразу зліва. Файл зберігається для наступного перегляду.</p>
      </div>
      <div className="fre-skin-file-actions">
        <label className="file-action"><Upload size={18}/> Вибрати скін<input type="file" accept="image/png" aria-label="Вибрати PNG-файл скіна" onChange={(e) => importImage(e.target.files?.[0],"skin")}/></label>
        <label className="file-action alt"><Upload size={18}/> Вибрати плащ<input type="file" accept="image/png" aria-label="Вибрати PNG-файл плаща" onChange={(e) => importImage(e.target.files?.[0],"cape")}/></label>
      </div>
      <div className="fre-skin-controls">
        <div className="field-label">2. Анімація персонажа</div>
        <div className="segmented">
          <button className={animation === "idle" ? "selected" : ""} onClick={() => setAnimation("idle")}><Pause size={15}/> Стоїть</button>
          <button className={animation === "walk" ? "selected" : ""} onClick={() => setAnimation("walk")}><Footprints size={15}/> Ходить</button>
          <button className={animation === "run" ? "selected" : ""} onClick={() => setAnimation("run")}><Play size={15}/> Біжить</button>
        </div>
        <button className="secondary" onClick={() => setSpin(!spin)}><RotateCcw size={16}/> {spin ? "Зупинити обертання" : "Увімкнути обертання"}</button>
        <button className="secondary" disabled={!custom} onClick={restoreDefault}><RotateCcw size={16}/> Повернути стандартний скін</button>
      </div>
      {error && <p className="error">{error}</p>}
      <div className="fre-skin-guide">
        <strong>Де працюватиме цей скін?</strong>
        <p>Наразі це лише локальний перегляд і редактор. На Microsoft-акаунт та ігрові сервери скін звідси не завантажується.</p>
      </div>
    </section>
  </div>{custom && skin && <SkinPixelEditor source={skin} onEdit={applySkin}/>}</>;
}
