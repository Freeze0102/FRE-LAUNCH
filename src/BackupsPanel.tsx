import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Archive, Download, RotateCcw, ShieldCheck } from "lucide-react";

type Backup = {filename: string; bytes: number};
export default function BackupsPanel({instanceId, instanceName}: {instanceId: string; instanceName: string}) {
  const [items, setItems] = useState<Backup[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  async function refresh() {
    if (!instanceId) {setItems([]);return;}
    setBusy(true);
    try {setItems(await invoke<Backup[]>("list_world_backups", {instanceId}));}
    catch (err) {setError(String(err));}
    finally {setBusy(false);}
  }
  useEffect(() => {void refresh();}, [instanceId]);
  async function make() {
    if (!instanceId) return;
    setBusy(true);setMessage("");setError("");
    try {
      const result = await invoke<Backup>("create_world_backup", {instanceId});
      setMessage(`Резервну копію створено: ${result.filename}`);
      await refresh();
    } catch (err) {setError(String(err));}
    finally {setBusy(false);}
  }
  async function restore(item: Backup) {
    if (!window.confirm(`Відновити світи з ${item.filename}? Вони будуть скопійовані в нові папки, наявні світи не перезаписуються.`)) return;
    setBusy(true);setMessage("");setError("");
    try {
      setMessage(await invoke<string>("restore_world_backup", {instanceId, filename: item.filename}));
    } catch (err) {setError(String(err));}
    finally {setBusy(false);}
  }
  return <section className="panel">
    <div className="panel-heading"><h3>FRE WORLD BACKUPS</h3><Archive size={21}/></div>
    <p>Безпечне резервне копіювання світів у ZIP. Відновлення завжди створює нові папки та не замінює наявні світи.</p>
    {!instanceId && <div className="small-empty"><Archive size={32}/><h3>Спочатку обери збірку</h3><p>Резервні копії прив'язуються до окремої збірки у FRE INSTANCES.</p></div>}
    {!!instanceId && <>
      <div className="toolbar"><span className="version-chip">{instanceName || "Вибрана збірка"}</span>
        <button className="primary" onClick={() => void make()} disabled={busy}><Download size={17}/> Створити копію</button>
      </div>
      {message && <div className="notice"><ShieldCheck size={17}/>{message}</div>}
      {error && <div className="error">{error}</div>}
      <div className="backup-list">
        {items.length === 0 && <p className="muted">Резервних копій поки немає.</p>}
        {items.map(item => <div className="backup-row" key={item.filename}>
          <span className="backup-icon"><Archive size={21}/></span>
          <div><b>{item.filename}</b><small>{(item.bytes / 1024 / 1024).toFixed(1)} MB</small></div>
          <button className="secondary" disabled={busy} onClick={() => void restore(item)}><RotateCcw size={16}/> Відновити</button>
        </div>)}
      </div>
    </>}
  </section>;
}
