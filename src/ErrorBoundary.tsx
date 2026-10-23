import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertTriangle, Copy, RotateCcw } from "lucide-react";

type Props = {children:ReactNode};
type State = {error:string|null;details:string};

export const LAST_UI_CRASH = "fre-launch-last-ui-crash";

export default class ErrorBoundary extends Component<Props,State> {
  state:State = {error:null,details:""};
  static getDerivedStateFromError(error:Error):Partial<State>{
    return {error:error.message||"Unknown rendering error"};
  }
  componentDidCatch(error:Error,info:ErrorInfo){
    const data=[new Date().toISOString(),error.stack??error.message,info.componentStack??""].join("\n");
    try{localStorage.setItem(LAST_UI_CRASH,data.slice(0,24000));}catch{/* local storage may be unavailable */}
    this.setState({details:data});
    console.error("FRE LAUNCH UI failure",error,info);
  }
  render(){
    if(this.state.error){
      return <main className="fre-crash-screen">
        <div className="fre-crash-inner">
          <div className="fre-crash-symbol"><AlertTriangle size={34}/></div>
          <div className="fre-crash-index">FRE / SAFE MODE — UI FAILURE</div>
          <h1>МИ НЕ ЗАГУБИЛИ<br/><em>ТВІЙ СВІТ.</em></h1>
          <p>Інтерфейс зіткнувся з помилкою. Світи Minecraft не видаляються. Скопіюй діагностику й перезапусти вікно.</p>
          <pre>{this.state.details||this.state.error}</pre>
          <div className="fre-crash-actions">
            <button onClick={()=>window.location.reload()}><RotateCcw size={19}/> ПЕРЕЗАПУСТИТИ ІНТЕРФЕЙС</button>
            <button onClick={()=>{void navigator.clipboard.writeText(this.state.details||this.state.error||"");}}><Copy size={19}/> КОПІЮВАТИ ПОМИЛКУ</button>
          </div>
          <div className="fre-crash-footer">FRE LAUNCH / LOCAL DIAGNOSTICS / NO TELEMETRY</div>
        </div>
      </main>;
    }
    return this.props.children;
  }
}
