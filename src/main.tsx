import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import ErrorBoundary, { LAST_UI_CRASH } from "./ErrorBoundary";
import "./style.css";

// Collect JavaScript failures locally. No analytics or network transport.
window.addEventListener("error",event=>{
  try{localStorage.setItem(LAST_UI_CRASH,[new Date().toISOString(),event.message,event.filename,event.lineno].join(" | ").slice(0,24000));}catch{/* storage disabled */}
});
window.addEventListener("unhandledrejection",event=>{
  try{localStorage.setItem(LAST_UI_CRASH,[new Date().toISOString(),"Unhandled promise",String(event.reason)].join(" | ").slice(0,24000));}catch{/* storage disabled */}
});
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary><App/></ErrorBoundary>
  </React.StrictMode>
);
