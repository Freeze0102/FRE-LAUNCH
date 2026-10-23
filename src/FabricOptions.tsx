import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Layers3, LoaderCircle } from "lucide-react";
import type { Instance } from "./Features";

type FabricLoader = {version: string; stable: boolean};

export default function FabricOptions({instance, onSaved}: {
  instance: Instance;
  onSaved: () => Promise<void>;
}) {
  const [kind, setKind] = useState(instance.loader);
  const [loaders, setLoaders] = useState<FabricLoader[]>([]);
  const [version, setVersion] = useState(instance.loaderVersion ?? "");
  const [busy, setBusy] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (kind !== "fabric" && kind !== "quilt") return;
    let alive = true;
    setFetching(true);
    void invoke<FabricLoader[]>(kind==="quilt"?"list_quilt_loaders":"list_fabric_loaders", {gameVersion: instance.version})
      .then((available) => {
        if (!alive) return;
        setLoaders(available);
        setVersion((current) => {
          if (current && available.some((entry) => entry.version === current)) return current;
          return available.find((entry) => entry.stable)?.version ?? available[0]?.version ?? "";
        });
      })
      .catch((err) => {if (alive) setError(String(err));})
      .finally(() => {if (alive) setFetching(false);});
    return () => {alive = false;};
  }, [kind, instance.version]);

  async function save() {
    if ((kind === "fabric" || kind === "quilt") && !version) return;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await invoke<Instance>("set_instance_loader", {
        id: instance.id,
        loader: kind,
        loaderVersion: kind === "fabric" || kind === "quilt" ? version : null,
      });
      setMessage(kind === "fabric" || kind === "quilt"
        ? `${kind==="fabric"?"Fabric":"Quilt"} Loader ${version} обраний. Бібліотеки завантажаться під час запуску.`
        : "Vanilla режим обрано.");
      await onSaved();
    } catch (err) {setError(String(err));}
    finally {setBusy(false);}
  }
  return <div className="fabric-options">
    <div className="panel-heading"><h3>Модлоадер</h3><Layers3 size={18}/></div>
    <label className="field-label" htmlFor={"loader-"+instance.id}>Тип збірки</label>
    <select id={"loader-"+instance.id} value={kind} onChange={(e)=>{setKind(e.target.value);setError("");}}>
      <option value="vanilla">Vanilla — без модлоадера</option>
      <option value="fabric">Fabric — підтримується</option>
      <option value="quilt">Quilt — підтримується</option>
      <option value="forge" disabled>Forge — готуємо підтримку</option>
      <option value="optifine" disabled>OptiFine — окреме встановлення</option>
    </select>
    {(kind === "fabric" || kind === "quilt") && <>
      <label className="field-label" htmlFor={"fabric-version-"+instance.id}>Версія модлоадера</label>
      {fetching
        ? <span className="muted"><LoaderCircle size={14}/> Завантаження сумісних версій з офіційного API...</span>
        : <select id={"fabric-version-"+instance.id} value={version} onChange={(e)=>setVersion(e.target.value)}>
            {loaders.length===0 && <option value="">Немає сумісних версій</option>}
            {loaders.map((item)=><option key={item.version} value={item.version}>{item.version}{item.stable?" · stable":""}</option>)}
          </select>}
      <p className="muted">Потрібна сумісна Java. Залежності завантажуються з офіційних репозиторіїв Fabric або Quilt.</p>
    </>}
    {error && <p className="error">{error}</p>}
    {message && <p className="notice">{message}</p>}
    <button className="primary" disabled={busy || fetching || ((kind === "fabric" || kind === "quilt") && !version)} onClick={()=>void save()}>
      Зберегти модлоадер
    </button>
    <div className="fre-loader-guidance"><strong>Forge та OptiFine</strong><p>Автоматичне встановлення Forge і OptiFine поки не реалізоване. Ми не запускаємо сторонні інсталятори у фоні.</p><div><a href="https://files.minecraftforge.net/" target="_blank" rel="noopener noreferrer">Офіційний Forge ↗</a><a href="https://optifine.net/downloads" target="_blank" rel="noopener noreferrer">Офіційний OptiFine ↗</a></div></div>
  </div>;
}
