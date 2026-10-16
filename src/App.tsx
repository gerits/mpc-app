import { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { api, onMidiEvent, onLooperSnapshot, type SystemStatus, type LooperSnapshot, type RoutedEvent } from './types/ipc';

export function App() {
  const [status, setStatus] = useState<SystemStatus | null>(null);
  const [looperSnap, setLooperSnap] = useState<LooperSnapshot | null>(null);
  const [masterVolume, setMasterVolume] = useState<number>(0.9);
  const [lastMidiEvent, setLastMidiEvent] = useState<string>('Ready');
  const [midiActivity, setMidiActivity] = useState<boolean>(false);

  // Poll system status on mount & periodically
  useEffect(() => {
    const fetchStatus = () => {
      api.getSystemStatus()
        .then((s) => setStatus(s))
        .catch((err) => console.error('Failed to get system status:', err));
    };

    fetchStatus();
    const interval = setInterval(fetchStatus, 2000);
    return () => clearInterval(interval);
  }, []);

  // Listen to realtime hardware MIDI events
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onMidiEvent((event: RoutedEvent) => {
      // Flash MIDI activity LED
      setMidiActivity(true);
      setTimeout(() => setMidiActivity(false), 80);

      switch (event.type) {
        case 'KeyboardNoteOn':
          setLastMidiEvent(`Keys: Note ${event.data.note} (Vel ${event.data.velocity})`);
          break;
        case 'KeyboardNoteOff':
          setLastMidiEvent(`Keys: Note ${event.data.note} Off`);
          break;
        case 'DrumPadOn':
          setLastMidiEvent(`Pad ${event.data.pad_index + 1}: Note ${event.data.note} (Vel ${event.data.velocity})`);
          break;
        case 'DrumPadOff':
          setLastMidiEvent(`Pad ${event.data.pad_index + 1} Released`);
          break;
        case 'MacroKnob':
          setLastMidiEvent(`Knob K${event.data.knob_index + 1}: ${(event.data.value * 100).toFixed(0)}%`);
          break;
        case 'PitchBend':
          setLastMidiEvent(`Pitch Bend: ${(event.data.value * 100).toFixed(0)}%`);
          break;
        case 'Modulation':
          setLastMidiEvent(`Modulation: ${(event.data.value * 100).toFixed(0)}%`);
          break;
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Listen to looper transport updates
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onLooperSnapshot((snap) => {
      setLooperSnap(snap);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleMasterVolumeChange = (vol: number) => {
    setMasterVolume(vol);
    api.setVolume('master', vol).catch(console.error);
  };

  const handlePanic = () => {
    api.panicAudio().catch(console.error);
    setLastMidiEvent('PANIC: All voices silenced');
  };

  const handleOpenSettings = () => {
    // Will trigger Settings modal (wired in TASK-011)
    setLastMidiEvent('Opening Settings...');
  };

  return (
    <div className="flex flex-col h-screen w-full bg-chassis-900 text-slate-200 select-none overflow-hidden">
      {/* Top Header / Akai Vintage OLED / Master Volume / VU Meters */}
      <Header
        status={status}
        looperSnap={looperSnap}
        masterVolume={masterVolume}
        onMasterVolumeChange={handleMasterVolumeChange}
        onPanic={handlePanic}
        onOpenSettings={handleOpenSettings}
      />

      {/* Main Workstation Workspace Shell */}
      <main className="flex-1 bg-carbon p-4 flex flex-col gap-4 overflow-hidden">
        {/* Placeholder for Middle Performance Grid (TASK-009 & TASK-010) */}
        <div className="flex-1 grid grid-cols-1 lg:grid-cols-12 gap-4 min-h-0">
          {/* Left Column: Keyboard & Melodic Area */}
          <div className="lg:col-span-7 bg-chassis-850 border border-chassis-750 rounded-xl p-4 flex flex-col justify-between shadow-lg relative overflow-hidden">
            <div className="flex items-center justify-between pb-3 border-b border-chassis-750">
              <div className="flex items-center gap-2">
                <span className="w-2.5 h-2.5 rounded-full bg-akai-red" />
                <h2 className="text-sm font-bold tracking-wider uppercase text-slate-100">
                  Melodic Synth & Keyboard
                </h2>
              </div>
              <span className="text-xs font-mono text-slate-400 bg-chassis-800 px-2.5 py-1 rounded border border-chassis-700">
                25 Velocity-Sensitive Keys
              </span>
            </div>

            <div className="flex-1 flex items-center justify-center my-4 text-center text-slate-500 font-mono text-sm">
              <div className="flex flex-col items-center gap-2">
                <div className="text-akai-red font-bold text-xs uppercase tracking-widest">
                  Ready for TASK-009
                </div>
                <div>KeyboardPreview & SoundFont Instrument Selector</div>
              </div>
            </div>

            <div className="flex items-center justify-between pt-2 border-t border-chassis-750 text-xs text-slate-400 font-mono">
              <span>Octave: C2 - C4</span>
              <span>Velocity Curve: Linear</span>
            </div>
          </div>

          {/* Right Column: Macro Knobs, Rubber Pads, Looper Transport */}
          <div className="lg:col-span-5 flex flex-col gap-4 min-h-0">
            {/* Top Right: 8 Macro Rotary Knobs */}
            <div className="bg-chassis-850 border border-chassis-750 rounded-xl p-4 shadow-lg">
              <div className="flex items-center justify-between pb-2 border-b border-chassis-750 mb-3">
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-200">
                  Macro Knobs (K1 - K8)
                </h3>
                <span className="text-[10px] font-mono text-slate-400">CC 70 - 77</span>
              </div>
              <div className="h-20 flex items-center justify-center text-slate-500 font-mono text-xs">
                DSP Macro Controls (Cutoff, Res, Chorus, Reverb, Overdrive, Stems)
              </div>
            </div>

            {/* Bottom Right: 2x4 MPC Rubber Pads */}
            <div className="flex-1 bg-chassis-850 border border-chassis-750 rounded-xl p-4 shadow-lg flex flex-col justify-between">
              <div className="flex items-center justify-between pb-2 border-b border-chassis-750 mb-3">
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-200">
                  MPC Drum Pads (Bank A / B)
                </h3>
                <span className="text-[10px] font-mono text-slate-400">Channel 10 Percussion</span>
              </div>
              <div className="flex-1 flex items-center justify-center text-slate-500 font-mono text-xs">
                8 Backlit Thick Rubber Pads with Note Repeat & Full Level
              </div>
            </div>
          </div>
        </div>
      </main>

      {/* Bottom Status Ticker / Hardware Link Bar */}
      <footer className="h-8 bg-chassis-900 border-t border-chassis-750 px-4 flex items-center justify-between text-[11px] font-mono text-slate-400 select-none">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5">
            <span
              className={`w-2 h-2 rounded-full transition-colors duration-75 ${
                midiActivity ? 'bg-[#2cf583] shadow-[0_0_8px_#2cf583]' : 'bg-slate-700'
              }`}
            />
            <span className="text-slate-500 font-semibold">MIDI IN:</span>
            <span className="text-slate-300 font-medium">{lastMidiEvent}</span>
          </div>
        </div>

        <div className="flex items-center gap-4 text-slate-500">
          <span>BUFFERS: 256 SPLS</span>
          <span className="text-slate-700">|</span>
          <span>LATENCY: ~5.8ms</span>
          <span className="text-slate-700">|</span>
          <span className="text-emerald-500/80 font-bold">PURE RUST SOUNDFONT ENGINE</span>
        </div>
      </footer>
    </div>
  );
}

export default App;
