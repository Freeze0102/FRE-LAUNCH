import { memo, useMemo } from "react";

/**
 * Custom, vector-only FRE LAUNCH landscape.
 * The stepped world is constructed from individual 3D voxels,
 * not a Minecraft screenshot or an external stock illustration.
 */
function WorldArt() {
  const columns = useMemo(() => {
    const heights = [2,2,3,3,4,6,6,7,9,10,9,8,7,7,8,10,12,13,13,11,10,9,8,7,5,4,3,2,2];
    return heights.map((height, i) => {
      const x = 28 + i * 22;
      const top = 390 - height * 15;
      const depth = 16;
      const surface = i % 6 === 0 ? "#b8e27d" : i % 3 === 0 ? "#b1cc81" : "#99bf6f";
      return {height, x, top, depth, surface, index:i};
    });
  }, []);
  return <svg className="world-art" viewBox="0 0 720 510" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Авторський піксельний ландшафт FRE LAUNCH із горами та сяючим порталом">
    <defs>
      <pattern id="fre-dots" width="22" height="22" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r="1" fill="#f4eee2" opacity=".14"/></pattern>
      <pattern id="fre-cross" width="32" height="32" patternUnits="userSpaceOnUse"><path d="M16 12V20M12 16H20" stroke="#d3d6c6" opacity=".11"/></pattern>
      <clipPath id="fre-sky"><rect x="0" y="0" width="720" height="510" rx="2"/></clipPath>
    </defs>
    <g clipPath="url(#fre-sky)">
      <rect width="720" height="510" fill="#243630"/>
      <rect width="720" height="510" fill="url(#fre-dots)"/>
      <circle cx="548" cy="134" r="94" fill="#efac8d"/>
      <circle cx="548" cy="134" r="126" stroke="#d1c3a5" opacity=".31" strokeWidth="1" strokeDasharray="4 8"/>
      <circle cx="548" cy="134" r="146" stroke="#d1c3a5" opacity=".13" strokeWidth="1"/>
      <path d="M548 6V262M420 134H676" stroke="#2f4940" strokeOpacity=".22"/>
      <path d="M0 302h44v-22h44v-27h66v-22h66v-37h65v37h44v26h55v23h65v35h74v-32h80v28h83v20h34v199H0V302Z" fill="#4c6353"/>
      <path d="M0 352h65v-25h44v-23h45v-21h44v-22h65v-37h44v28h44v44h55v22h66v-41h65v-24h66v23h43v26h74v208H0V352Z" fill="#718768"/>
      <path d="M0 383h86v-33h43v-27h44v-44h44v-26h44v-29h44v28h44v60h43v40h66v-28h87v-40h43v-21h66v38h64v209H0V383Z" fill="#829a70"/>
      <path d="M0 448h720v62H0z" fill="#192821"/>
      {columns.map(({x,top,depth,surface,height,index})=><g key={index}>
        <path d={`M${x} ${top+depth} L${x+20} ${top+depth} L${x+20} 460 L${x} 460Z`} fill={index%4===0?"#355344":"#48634d"}/>
        <path d={`M${x+20} ${top+depth} l9 -9 V451 l-9 9Z`} fill="#253d35"/>
        <path d={`M${x} ${top+depth} l9 -9 h20 l-9 9Z`} fill={surface}/>
        {height>6 && <path d={`M${x+2} ${top+depth+37} h14 M${x+3} ${top+depth+75} h12`} stroke="#72936e" strokeWidth="1" opacity=".5"/>}
      </g>)}
      <path d="M244 326h86v-75h16v-16h44v16h16v75h86v135H244V326Z" fill="#111f1d"/>
      <path d="M289 322v-39h22v-21h22v-19h30v19h23v21h20v39" stroke="#c8f476" strokeWidth="7" strokeLinejoin="miter"/>
      <path d="M314 330V292h20v-16h38v16h19v38" fill="#c8f476" opacity=".15"/>
      <path d="M334 341h54v103h-54z" fill="#a4d775"/>
      <path d="M346 360h29v84h-29z" fill="#192821"/>
      <rect x="358" y="371" width="6" height="13" fill="#f3a686"/>
      <rect x="307" y="335" width="10" height="10" fill="#c9f87e"/>
      <rect x="405" y="347" width="9" height="9" fill="#c9f87e"/>
      <path d="M24 464h672" stroke="#deebcb" opacity=".35" strokeWidth="1" strokeDasharray="2 10"/>
      <rect x="18" y="18" width="111" height="35" fill="#192821" stroke="#7c9282" strokeWidth="1"/>
      <text x="31" y="40" fill="#c8f476" fontFamily="monospace" fontSize="13" letterSpacing="2">F / 001</text>
      <text x="520" y="463" fill="#c5d0b7" fontFamily="monospace" fontSize="11" letterSpacing="2">OVERWORLD</text>
      <text x="25" y="487" fill="#a7b5a3" fontFamily="monospace" fontSize="10" letterSpacing="2">X: 128  /  Z: 256</text>
      <path d="M680 24h-34m17-17v34" stroke="#dfddca" opacity=".5"/>
      <path d="M74 88h12m-6-6v12M221 79h16M229 71v16" stroke="#d5e1d3" opacity=".38"/>
      <rect width="720" height="510" fill="url(#fre-cross)"/>
    </g>
  </svg>;
}

// Installation progress must not re-render the complex voxel art on every event.
export default memo(WorldArt);
