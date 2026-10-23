/**
 * Offline Steve-style fallback for the skin preview.
 * Generates a real 64x64 Minecraft skin atlas with HTML Canvas and no network.
 * Only used when no user-supplied PNG has been saved.
 */
export function builtInSkin(): string {
  const canvas=document.createElement("canvas");
  canvas.width=64;canvas.height=64;
  const context=canvas.getContext("2d",{willReadFrequently:false});
  if(!context)return "";
  const fill=(x:number,y:number,w:number,h:number,color:string)=>{
    context.fillStyle=color;context.fillRect(x,y,w,h);
  };
  const hair="#493224",hairLight="#624331",skin="#bd8560",skinBright="#d5a078";
  const shirt="#24a3af",shirtDark="#16818d",pants="#344f8f",pantsDark="#253c70";
  const boots="#3d3330";
  // Head UV faces.
  fill(8,0,16,8,hair);fill(0,8,32,8,skinBright);
  fill(8,8,8,8,skinBright);fill(8,8,8,2,hair);
  fill(8,10,1,3,hairLight);fill(15,10,1,3,hairLight);
  fill(10,11,2,1,"#ede9e1");fill(13,11,2,1,"#ede9e1");
  fill(11,11,1,1,"#414f67");fill(13,11,1,1,"#414f67");
  fill(11,14,3,1,"#83543f");
  fill(24,8,8,8,hair);
  // Chest/torso UV.
  fill(20,16,16,4,shirt);
  fill(16,20,24,12,shirt);
  fill(20,20,8,12,shirt);
  fill(20,20,1,12,shirtDark);fill(27,20,1,12,shirtDark);
  fill(20,30,8,2,pantsDark);fill(32,20,8,12,shirtDark);
  // Right arm UV: skin below rolled-up cyan sleeves.
  fill(44,16,8,4,shirt);
  fill(40,20,16,12,skin);fill(44,20,4,12,skinBright);
  fill(44,20,4,4,shirt);fill(40,20,4,4,shirtDark);
  // Left arm UV (new 64x64 format).
  fill(36,48,8,4,shirt);fill(32,52,16,12,skin);
  fill(36,52,4,12,skinBright);fill(36,52,4,4,shirt);
  // Pants and boots (both legs).
  fill(4,16,8,4,pants);fill(0,20,16,12,pants);fill(4,20,4,12,pants);
  fill(4,29,4,3,boots);fill(0,29,4,3,boots);
  fill(20,48,8,4,pants);fill(16,52,16,12,pants);
  fill(20,52,4,12,pants);fill(20,61,4,3,boots);fill(16,61,4,3,boots);
  return canvas.toDataURL("image/png");
}
