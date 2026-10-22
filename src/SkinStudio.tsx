import { useEffect, useRef, useState } from "react";
import { SkinViewer, WalkingAnimation, RunningAnimation } from "skinview3d";
import { ImagePlus, RotateCcw, Footprints, Pause, Play, Upload } from "lucide-react";
import SkinPixelEditor from "./SkinPixelEditor";

type Animate = "idle" | "walk" | "run";
const STORAGE_SKIN = "fre-launch:skin-preview";

export default function SkinStudio() {
  const canvas = useRef<HTMLCanvasElement>(null);
  const viewer = useRef<SkinViewer | null>(null);
  const [skin, setSkin] = useState<string>(() => localStorage.getItem(STORAGE_SKIN) ?? "");
  const [cape, setCape] = useState<string>("");
  const [animation, setAnimation] = useState<Animate>("walk");
  const [spin, setSpin] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!canvas.current) return;
    const current = new SkinViewer({
      canvas: canvas.current,
      width: 340,
      height: 440,
      ...(skin ? {skin} : {})
    });
    current.autoRotate = true;
    current.animation = new WalkingAnimation();
    viewer.current = current;
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
          try {localStorage.setItem(STORAGE_SKIN, encoded);} catch {setError("Недостатньо місця для збереження скіна.");}
          setSkin(encoded);
        } else setCape(encoded);
      };
      reader.onerror = () => setError("Не вдалося прочитати PNG.");
      reader.readAsDataURL(file);
    };
    image.onerror = () => {URL.revokeObjectURL(local);setError("Пошкоджений PNG-файл.");};
    image.src = local;
  }

  function applySkin(dataUrl:string){
    try {localStorage.setItem(STORAGE_SKIN,dataUrl);setSkin(dataUrl);}
    catch {setError("Не вдалося зберегти відредагований скін.");}
  }
  return <><div className="studio-grid">
    <section className="panel skin-stage">
      <div className="panel-heading"><h3>FRE SKIN STUDIO</h3><ImagePlus size={19}/></div>
      <div className="skin-canvas"><canvas ref={canvas} aria-label="3D-попередній перегляд скіна Minecraft"/></div>
      <p className="muted">Крути персонажа мишкою, змінюй масштаб коліщатком.</p>
    </section>
    <section className="panel skin-tools">
      <h3>Мій персонаж</h3>
      <p>Завантаж власний PNG-скін або плащ. Скін зберігається локально для наступного перегляду.</p>
      <label className="file-action"><Upload size={18}/> Завантажити скін<input type="file" accept="image/png" onChange={(e) => importImage(e.target.files?.[0],"skin")}/></label>
      <label className="file-action alt"><Upload size={18}/> Завантажити плащ<input type="file" accept="image/png" onChange={(e) => importImage(e.target.files?.[0],"cape")}/></label>
      <div className="field-label">Анімація персонажа</div>
      <div className="segmented">
        <button className={animation === "idle" ? "selected" : ""} onClick={() => setAnimation("idle")}><Pause size={15}/> Стоїть</button>
        <button className={animation === "walk" ? "selected" : ""} onClick={() => setAnimation("walk")}><Footprints size={15}/> Ходить</button>
        <button className={animation === "run" ? "selected" : ""} onClick={() => setAnimation("run")}><Play size={15}/> Біжить</button>
      </div>
      <button className="secondary" onClick={() => setSpin(!spin)}><RotateCcw size={16}/> {spin ? "Зупинити обертання" : "Увімкнути обертання"}</button>
      {error && <p className="error">{error}</p>}
      <p className="muted">Це локальний 3D-перегляд. Застосування скінів до Microsoft-акаунта поки не підтримується.</p>
    </section>
  </div>{skin && <SkinPixelEditor source={skin} onEdit={applySkin}/>}</>;
}
