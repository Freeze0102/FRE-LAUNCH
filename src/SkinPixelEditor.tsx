import { useEffect, useRef, useState } from "react";
import type { PointerEvent } from "react";
import { Brush, Eraser, Pipette, Undo2, Download } from "lucide-react";

type Tool = "brush" | "erase" | "pick";

export default function SkinPixelEditor({source,onEdit}:{source:string;onEdit:(dataUrl:string)=>void}){
  const canvas=useRef<HTMLCanvasElement>(null);
  const [color,setColor]=useState("#29c6f0");
  const [tool,setTool]=useState<Tool>("brush");
  const [undo,setUndo]=useState<string[]>([]);
  const [error,setError]=useState("");
  useEffect(()=>{
    if(!source||!canvas.current)return;
    const img=new Image();
    img.onload=()=>{
      if(!canvas.current)return;
      canvas.current.width=img.naturalWidth;
      canvas.current.height=img.naturalHeight;
      const ctx=canvas.current.getContext("2d");
      if(ctx){ctx.imageSmoothingEnabled=false;ctx.clearRect(0,0,canvas.current.width,canvas.current.height);ctx.drawImage(img,0,0);}
    };
    img.onerror=()=>setError("Не вдалося відкрити текстуру для редагування.");
    img.src=source;
  },[source]);

  function paint(e:PointerEvent<HTMLCanvasElement>){
    const c=canvas.current;if(!c)return;
    const ctx=c.getContext("2d",{willReadFrequently:true});if(!ctx)return;
    const rect=c.getBoundingClientRect();
    if(rect.width===0||rect.height===0)return;
    const x=Math.max(0,Math.min(c.width-1,Math.floor((e.clientX-rect.left)/rect.width*c.width)));
    const y=Math.max(0,Math.min(c.height-1,Math.floor((e.clientY-rect.top)/rect.height*c.height)));
    if(tool==="pick"){
      const p=ctx.getImageData(x,y,1,1).data;
      setColor("#"+Array.from(p.slice(0,3)).map(a=>a.toString(16).padStart(2,"0")).join(""));
      return;
    }
    ctx.imageSmoothingEnabled=false;
    if(tool==="erase"){ctx.clearRect(x,y,1,1);}
    else{ctx.fillStyle=color;ctx.fillRect(x,y,1,1);}
  }
  function down(e:PointerEvent<HTMLCanvasElement>){
    const c=canvas.current;if(!c||!source)return;
    if(tool!=="pick"){
      setUndo(h=>[...h.slice(-14),c.toDataURL("image/png")]);
      c.setPointerCapture(e.pointerId);
    }
    paint(e);
  }
  function move(e:PointerEvent<HTMLCanvasElement>){
    if(e.buttons===1&&tool!=="pick")paint(e);
  }
  function up(e:PointerEvent<HTMLCanvasElement>){
    const c=canvas.current;if(!c||tool==="pick")return;
    if(c.hasPointerCapture(e.pointerId))c.releasePointerCapture(e.pointerId);
    try{onEdit(c.toDataURL("image/png"));}catch(err){setError(String(err));}
  }
  function undoEdit(){
    if(!undo.length)return;
    const previous=undo[undo.length-1];
    setUndo(a=>a.slice(0,-1));
    onEdit(previous);
  }
  return <section className="panel skin-editor">
    <div className="panel-heading"><h3>FRE PIXEL EDITOR</h3><Brush size={19}/></div>
    <p>Редагуй пікселі текстури. Натисни і веди мишкою або стилусом. Зміни одразу відображаються в 3D.</p>
    <div className="skin-edit-toolbar">
      <button className={tool==="brush"?"active":""} onClick={()=>setTool("brush")} title="Малювати"><Brush size={16}/> Олівець</button>
      <button className={tool==="erase"?"active":""} onClick={()=>setTool("erase")} title="Стерти піксель"><Eraser size={16}/> Гумка</button>
      <button className={tool==="pick"?"active":""} onClick={()=>setTool("pick")} title="Взяти колір"><Pipette size={16}/> Піпетка</button>
      <input aria-label="Колір пензля" title="Колір" type="color" value={color} onChange={e=>setColor(e.target.value)}/>
      <button disabled={!undo.length} onClick={undoEdit} title="Скасувати"><Undo2 size={16}/></button>
    </div>
    {source?<canvas className="pixel-canvas" ref={canvas} onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={up}/>:<div className="small-empty">Імпортуй PNG-скін для редагування.</div>}
    {source&&<a className="secondary skin-save" href={source} download="FRE-LAUNCH-skin.png"><Download size={15}/> Зберегти PNG</a>}
    {error&&<div className="error">{error}</div>}
    <p className="muted">Збереження в бібліотеці локальне; зміни не публікуються на акаунті Minecraft.</p>
  </section>;
}
