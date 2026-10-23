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
    if (kind !== "fabric") return;
    let alive = true;
    setFetching(true);
    void invoke<FabricLoader[]>("list_fabric_loaders", {gameVersion: instance.version})
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
    if (kind === "fabric" && !version) return;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await invoke<Instance>("set_instance_loader", {
        id: instance.id,
        loader: kind,
        loaderVersion: kind === "fabric" ? version : null,
      });
      setMessage(kind === "fabric"
        ? `Fabric Loader ${version} обраний. Його бібліотеки завантажаться під час запуску.`
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
    </select>
    {kind === "fabric" && <>
      <label className="field-label" htmlFor={"fabric-version-"+instance.id}>Версія Fabric Loader</label>
      {fetching
        ? <span className="muted"><LoaderCircle size={14}/> Завантаження сумісних версій з Fabric API...</span>
        : <select id={"fabric-version-"+instance.id} value={version} onChange={(e)=>setVersion(e.target.value)}>
            {loaders.length===0 && <option value="">Немає сумісних версій</option>}
            {loaders.map((item)=><option key={item.version} value={item.version}>{item.version}{item.stable?" · stable":""}</option>)}
          </select>}
      <p className="muted">Для Fabric потрібна сумісна Java. Залежності завантажуються з офіційного Fabric Maven.</p>
    </>}
    {error && <p className="error">{error}</p>}
    {message && <p className="notice">{message}</p>}
    <button className="primary" disabled={busy || fetching || (kind === "fabric" && !version)} onClick={()=>void save()}>
      Зберегти модлоадер
    </button>
    <p className="muted">Forge, NeoForge та Quilt будуть додані окремо після перевірки механізмів встановлення.</p>
  </div>;
}
