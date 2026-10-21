import { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { InstrumentSelector } from './components/InstrumentSelector';
import { KeyboardPreview } from './components/KeyboardPreview';
import { PadMatrix } from './components/PadMatrix';
import { KnobRack } from './components/KnobRack';
import { LooperBar } from './components/LooperBar';
import { LoopEditor } from './components/LoopEditor';
import { SettingsModal } from './components/SettingsModal';
import { api, onMidiEvent, onLooperSnapshot, type SystemStatus, type LooperSnapshot, type RoutedEvent, type CuratedPreset } from './types/ipc';

export function App() {
  const [status, setStatus] = useState<SystemStatus | null>(null);
  const [looperSnap, setLooperSnap] = useState<LooperSnapshot | null>(null);
  const [selectedTrack, setSelectedTrack] = useState<number>(0);
  const [showLoopEditor, setShowLoopEditor] = useState<boolean>(true);
  const [masterVolume, setMasterVolume] = useState<number>(0.9);
  const [lastMidiEvent, setLastMidiEvent] = useState<string>('Ready');
  const [midiActivity, setMidiActivity] = useState<boolean>(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);

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
    setIsSettingsOpen(true);
  };

  const handlePresetChanged = (preset: CuratedPreset) => {
    setLastMidiEvent(`Loaded: ${preset.name}`);
    api.getSystemStatus().then(setStatus).catch(console.error);
  };

  return (
    <div className="flex flex-col h-screen w-full bg-chassis-900 text-slate-200 select-none overflow-hidden">
      {/* Top Header: Master Volume / VU Meters / Controls */}
      <Header
        masterVolume={masterVolume}
        onMasterVolumeChange={handleMasterVolumeChange}
        onPanic={handlePanic}
        onOpenSettings={handleOpenSettings}
      />

      {/* Settings Modal */}
      <SettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        systemStatus={status}
        onStatusUpdated={() => api.getSystemStatus().then(setStatus)}
      />

      {/* Main Workstation Workspace Shell */}
      <main className="flex-1 bg-carbon p-4 flex flex-col gap-3 overflow-y-auto min-h-0">
        {/* Top: 2-Track Quick Looper & Transport Bar */}
        <LooperBar
          snapshot={looperSnap}
          selectedTrack={selectedTrack}
          onSelectTrack={setSelectedTrack}
          showEditor={showLoopEditor}
          onToggleEditor={() => setShowLoopEditor((prev) => !prev)}
        />

        {/* Interactive Loop Visualizer & Note Alignment Editor */}
        {showLoopEditor && (
          <LoopEditor
            snapshot={looperSnap}
            selectedTrack={selectedTrack}
            onSelectTrack={setSelectedTrack}
          />
        )}

        <div className="flex-1 grid grid-cols-1 lg:grid-cols-12 gap-4 min-h-0">
          {/* Left Column: Keyboard & Melodic Area */}
          <div className="lg:col-span-7 bg-chassis-850 border border-chassis-750 rounded-xl p-4 flex flex-col justify-between shadow-lg relative overflow-hidden gap-4">
            <InstrumentSelector onPresetChanged={handlePresetChanged} />
            <div className="mt-auto">
              <KeyboardPreview />
            </div>
          </div>

          {/* Right Column: Macro Knobs & Rubber Pads */}
          <div className="lg:col-span-5 flex flex-col gap-3 min-h-0">
            {/* Top Right: 8 Macro Rotary Knobs */}
            <div className="bg-chassis-850 border border-chassis-750 rounded-xl p-3 shadow-lg">
              <KnobRack />
            </div>

            {/* Bottom Right: 2x4 MPC Rubber Pads */}
            <div className="flex-1 bg-chassis-850 border border-chassis-750 rounded-xl p-3 shadow-lg flex flex-col justify-between overflow-y-auto">
              <PadMatrix />
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
      </footer>
    </div>
  );
}

export default App;
