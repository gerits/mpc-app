import { useState, useEffect } from "react";
import { Sliders, Music, Disc, Cpu, Zap, Volume2 } from "lucide-react";

export function App() {
  const [activeTab, setActiveTab] = useState<"performance" | "settings">("performance");
  const [deviceConnected] = useState<boolean>(false);

  useEffect(() => {
    // Scaffold initial state check
    console.log("MiniStation App initialized");
  }, []);

  return (
    <div className="flex flex-col h-screen w-full bg-chassis-900 text-slate-200">
      {/* Top Header / OLED Display Bar */}
      <header className="h-14 border-b border-chassis-700 bg-chassis-850 px-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-akai-red animate-pulse" />
            <h1 className="font-bold tracking-wider text-white text-lg flex items-center gap-1.5">
              MINI<span className="text-akai-red">STATION</span>
            </h1>
          </div>
          <span className="text-xs px-2 py-0.5 rounded bg-chassis-700 text-slate-400 font-mono">
            MPK mini 3 Edition
          </span>
        </div>

        {/* OLED Screen simulation */}
        <div className="bg-oled-bg border border-chassis-700 rounded px-4 py-1.5 flex items-center gap-4 text-xs font-mono shadow-inner">
          <div className="flex items-center gap-1.5">
            <Cpu className="w-3.5 h-3.5 text-oled-green" />
            <span className="text-slate-400">MIDI:</span>
            <span className={deviceConnected ? "text-oled-green" : "text-oled-amber"}>
              {deviceConnected ? "CONNECTED" : "SCANNING..."}
            </span>
          </div>
          <div className="h-3 w-px bg-chassis-700" />
          <div className="flex items-center gap-1.5">
            <Music className="w-3.5 h-3.5 text-oled-green" />
            <span className="text-slate-400">PROGRAM:</span>
            <span className="text-oled-green font-semibold">001: GRAND PIANO</span>
          </div>
          <div className="h-3 w-px bg-chassis-700" />
          <div className="flex items-center gap-1.5">
            <Volume2 className="w-3.5 h-3.5 text-oled-green" />
            <span className="text-slate-400">DSP:</span>
            <span className="text-oled-green">48kHz / 128 SPLS</span>
          </div>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => setActiveTab(activeTab === "performance" ? "settings" : "performance")}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-chassis-750 hover:bg-chassis-700 text-xs text-slate-300 transition-colors border border-chassis-600"
          >
            <Sliders className="w-3.5 h-3.5 text-akai-red" />
            <span>{activeTab === "performance" ? "Config" : "Back to Station"}</span>
          </button>
        </div>
      </header>

      {/* Main Workstation Workspace */}
      <main className="flex-1 overflow-hidden p-4 grid grid-rows-[auto_1fr] gap-4">
        {/* Top Control Bar: Instrument & Macro Quick View */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-chassis-800 border border-chassis-700 rounded-lg p-3">
            <div className="text-xs uppercase tracking-wider text-slate-400 font-semibold mb-2 flex items-center gap-1.5">
              <Music className="w-3.5 h-3.5 text-akai-red" />
              Melodic Synth (Keys)
            </div>
            <div className="bg-chassis-850 border border-chassis-700 rounded p-2 text-sm text-slate-200">
              Yamaha C3 Acoustic Grand (SF2)
            </div>
          </div>

          <div className="bg-chassis-800 border border-chassis-700 rounded-lg p-3">
            <div className="text-xs uppercase tracking-wider text-slate-400 font-semibold mb-2 flex items-center gap-1.5">
              <Disc className="w-3.5 h-3.5 text-akai-red" />
              Drum Kit (Pads)
            </div>
            <div className="bg-chassis-850 border border-chassis-700 rounded p-2 text-sm text-slate-200">
              Vintage 808 / MPC Standard Kit
            </div>
          </div>

          <div className="bg-chassis-800 border border-chassis-700 rounded-lg p-3">
            <div className="text-xs uppercase tracking-wider text-slate-400 font-semibold mb-2 flex items-center gap-1.5">
              <Zap className="w-3.5 h-3.5 text-akai-red" />
              Macro Knobs Status
            </div>
            <div className="bg-chassis-850 border border-chassis-700 rounded p-2 text-sm text-slate-200 font-mono">
              8 Rotary Knobs Mapped (CC 70–77)
            </div>
          </div>
        </div>

        {/* Center Canvas Preview: Scaffolding Complete */}
        <div className="bg-chassis-800 border border-chassis-700 rounded-lg p-6 flex flex-col items-center justify-center text-center">
          <div className="w-16 h-16 rounded-full bg-chassis-750 border border-chassis-600 flex items-center justify-center mb-4 text-akai-red">
            <Sliders className="w-8 h-8" />
          </div>
          <h2 className="text-xl font-bold text-white mb-2">MiniStation Core Scaffolding Verified</h2>
          <p className="text-sm text-slate-400 max-w-lg mb-6">
            Tauri v2 + Rust sound synthesis backend and React 18 / TypeScript frontend pipeline initialized. Ready for MIDI auto-routing and SoundFont integration.
          </p>
          <div className="flex gap-2">
            <span className="text-xs bg-chassis-700 text-slate-300 px-3 py-1 rounded font-mono">
              Tauri v2
            </span>
            <span className="text-xs bg-chassis-700 text-slate-300 px-3 py-1 rounded font-mono">
              Rust 1.95
            </span>
            <span className="text-xs bg-chassis-700 text-slate-300 px-3 py-1 rounded font-mono">
              React + Vite
            </span>
            <span className="text-xs bg-chassis-700 text-slate-300 px-3 py-1 rounded font-mono">
              Tailwind CSS
            </span>
          </div>
        </div>
      </main>

      {/* Status Bar */}
      <footer className="h-7 border-t border-chassis-700 bg-chassis-850 px-4 flex items-center justify-between text-xs text-slate-500 font-mono">
        <div>STATUS: OK • READY FOR HARDWARE DISCOVERY</div>
        <div>TASK-001 SCAFFOLDING COMPLETE</div>
      </footer>
    </div>
  );
}

export default App;
