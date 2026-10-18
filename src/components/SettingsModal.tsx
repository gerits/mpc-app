import React, { useState, useEffect } from 'react';
import { X, Volume2, Cpu, Music, Sliders, RefreshCw } from 'lucide-react';
import { SoundFontManager } from './SoundFontManager';
import { api, type MidiDeviceInfo, type SystemStatus, type MetronomeMode } from '../types/ipc';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  systemStatus: SystemStatus | null;
  onStatusUpdated?: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  systemStatus,
  onStatusUpdated,
}) => {
  const [activeTab, setActiveTab] = useState<'audio-midi' | 'soundfonts' | 'mapping'>('audio-midi');
  const [audioDevices, setAudioDevices] = useState<string[]>([]);
  const [midiDevices, setMidiDevices] = useState<MidiDeviceInfo[]>([]);
  const [selectedBuffer, setSelectedBuffer] = useState<number>(systemStatus?.buffer_size ?? 256);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  // Metronome settings state
  const [metroEnabled, setMetroEnabled] = useState<boolean>(false);
  const [metroVolume, setMetroVolume] = useState<number>(0.7);
  const [metroMode, setMetroMode] = useState<MetronomeMode>('while_running');

  const loadDevices = () => {
    api.getAudioDevices()
      .then(setAudioDevices)
      .catch((err) => console.error('Audio devices error:', err));

    api.getMidiDevices()
      .then(setMidiDevices)
      .catch((err) => console.error('MIDI devices error:', err));
  };

  useEffect(() => {
    if (isOpen) {
      loadDevices();
      if (systemStatus?.buffer_size) {
        setSelectedBuffer(systemStatus.buffer_size);
      }
      api.getLooperSnapshot().then((snap) => {
        if (snap.metronome_enabled !== undefined) setMetroEnabled(snap.metronome_enabled);
        if (snap.metronome_volume !== undefined) setMetroVolume(snap.metronome_volume);
        if (snap.metronome_mode !== undefined) setMetroMode(snap.metronome_mode);
      }).catch(console.error);
    }
  }, [isOpen, systemStatus]);

  if (!isOpen) return null;

  const handleBufferSizeChange = (newSize: number) => {
    setSelectedBuffer(newSize);
    api.setBufferSize(newSize)
      .then(() => {
        setStatusMessage(`Buffer size updated to ${newSize} samples`);
        if (onStatusUpdated) onStatusUpdated();
        setTimeout(() => setStatusMessage(null), 3000);
      })
      .catch((err) => setStatusMessage(`Failed to set buffer: ${err}`));
  };

  const handleConnectMidi = (name: string) => {
    api.connectMidiDevice(name)
      .then(() => {
        setStatusMessage(`Connected to MIDI device: ${name}`);
        loadDevices();
        if (onStatusUpdated) onStatusUpdated();
        setTimeout(() => setStatusMessage(null), 4000);
      })
      .catch((err) => setStatusMessage(`Connection error: ${err}`));
  };

  const handleDisconnectMidi = () => {
    api.disconnectMidiDevice()
      .then(() => {
        setStatusMessage('Disconnected MIDI input');
        loadDevices();
        if (onStatusUpdated) onStatusUpdated();
        setTimeout(() => setStatusMessage(null), 3000);
      })
      .catch((err) => setStatusMessage(`Disconnect error: ${err}`));
  };

  const sampleRate = systemStatus?.sample_rate ?? 44100;
  const latencyMs = ((selectedBuffer / sampleRate) * 1000).toFixed(1);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm select-none">
      <div className="w-full max-w-2xl bg-chassis-850 border-2 border-chassis-700 rounded-2xl shadow-2xl flex flex-col overflow-hidden max-h-[85vh]">
        {/* Modal Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-chassis-750 bg-chassis-900">
          <div className="flex items-center gap-2.5">
            <Sliders className="w-5 h-5 text-akai-red" />
            <h3 className="font-extrabold text-white text-base tracking-wide uppercase">
              Workstation Settings
            </h3>
          </div>

          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-chassis-750 transition-all active:scale-95"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Tab Navigation */}
        <div className="flex items-center px-6 border-b border-chassis-750 bg-chassis-850 gap-4 font-mono text-xs">
          <button
            onClick={() => setActiveTab('audio-midi')}
            className={`py-3 font-bold border-b-2 transition-all flex items-center gap-2 ${
              activeTab === 'audio-midi'
                ? 'border-akai-red text-white'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Volume2 className="w-4 h-4 text-akai-red" />
            Audio & MIDI
          </button>

          <button
            onClick={() => setActiveTab('soundfonts')}
            className={`py-3 font-bold border-b-2 transition-all flex items-center gap-2 ${
              activeTab === 'soundfonts'
                ? 'border-akai-red text-white'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Music className="w-4 h-4 text-akai-red" />
            SoundFont Manager
          </button>

          <button
            onClick={() => setActiveTab('mapping')}
            className={`py-3 font-bold border-b-2 transition-all flex items-center gap-2 ${
              activeTab === 'mapping'
                ? 'border-akai-red text-white'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Cpu className="w-4 h-4 text-akai-red" />
            MIDI Layout
          </button>
        </div>

        {/* Modal Body */}
        <div className="p-6 overflow-y-auto flex-1 font-mono text-xs">
          {statusMessage && (
            <div className="mb-4 p-3 rounded-lg bg-chassis-900 border border-emerald-500/50 text-emerald-400 flex items-center justify-between">
              <span>{statusMessage}</span>
              <button onClick={() => setStatusMessage(null)} className="text-slate-400 hover:text-white">
                &times;
              </button>
            </div>
          )}

          {/* TAB 1: Audio & MIDI Settings */}
          {activeTab === 'audio-midi' && (
            <div className="flex flex-col gap-6">
              {/* Audio Output Section */}
              <div className="flex flex-col gap-3">
                <h4 className="font-bold text-slate-200 uppercase text-xs flex items-center gap-2">
                  <Volume2 className="w-4 h-4 text-akai-red" />
                  Audio Output Engine (CPAL Low-Latency)
                </h4>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 bg-chassis-900 border border-chassis-750 p-4 rounded-xl">
                  {/* Device Select */}
                  <div className="flex flex-col gap-1.5">
                    <span className="text-[10px] text-slate-400 uppercase font-bold">Output Device</span>
                    <select
                      value={systemStatus?.audio_device ?? ''}
                      disabled
                      className="bg-chassis-800 border border-chassis-700 rounded-lg p-2 text-slate-200 text-xs focus:outline-none"
                    >
                      {audioDevices.length > 0 ? (
                        audioDevices.map((d, i) => (
                          <option key={i} value={d}>
                            {d}
                          </option>
                        ))
                      ) : (
                        <option>{systemStatus?.audio_device ?? 'Default System Output'}</option>
                      )}
                    </select>
                  </div>

                  {/* Buffer Size & Latency */}
                  <div className="flex flex-col gap-1.5">
                    <div className="flex items-center justify-between text-[10px]">
                      <span className="text-slate-400 uppercase font-bold">Buffer Size</span>
                      <span className="text-emerald-400 font-bold">~{latencyMs}ms Latency</span>
                    </div>
                    <select
                      value={selectedBuffer}
                      onChange={(e) => handleBufferSizeChange(parseInt(e.target.value))}
                      className="bg-chassis-800 border border-chassis-700 rounded-lg p-2 text-slate-200 text-xs focus:outline-none focus:border-akai-red cursor-pointer"
                    >
                      <option value={64}>64 samples (1.5 ms) - Ultra Low</option>
                      <option value={128}>128 samples (2.9 ms) - Low Latency</option>
                      <option value={256}>256 samples (5.8 ms) - Balanced Default</option>
                      <option value={512}>512 samples (11.6 ms) - Conservative</option>
                      <option value={1024}>1024 samples (23.2 ms) - High Stability</option>
                    </select>
                  </div>
                </div>
              </div>

              {/* Metronome & Click Monitor Section */}
              <div className="flex flex-col gap-3">
                <div className="flex items-center justify-between">
                  <h4 className="font-bold text-slate-200 uppercase text-xs flex items-center gap-2">
                    <span className="text-akai-red font-bold text-sm">⏱</span>
                    Metronome & Click Track (Loop Isolated)
                  </h4>
                  <span className="text-[10px] text-emerald-400 font-bold bg-emerald-950/60 border border-emerald-800/80 px-2 py-0.5 rounded">
                    Audition Only • Never in Loop/WAV
                  </span>
                </div>

                <div className="bg-chassis-900 border border-chassis-750 p-4 rounded-xl flex flex-col gap-3">
                  <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    {/* Toggle Metronome */}
                    <div className="flex flex-col gap-1.5">
                      <span className="text-[10px] text-slate-400 uppercase font-bold">Metronome State</span>
                      <button
                        onClick={() => {
                          const next = !metroEnabled;
                          setMetroEnabled(next);
                          api.looperSetMetronome(next).catch(console.error);
                        }}
                        className={`p-2 rounded-lg text-xs font-bold transition-all border ${
                          metroEnabled
                            ? 'bg-akai-red text-white border-white shadow'
                            : 'bg-chassis-800 border-chassis-700 text-slate-300 hover:text-white'
                        }`}
                      >
                        {metroEnabled ? 'CLICK ENABLED (ON)' : 'CLICK MUTED (OFF)'}
                      </button>
                    </div>

                    {/* Volume Slider */}
                    <div className="flex flex-col gap-1.5">
                      <div className="flex items-center justify-between text-[10px]">
                        <span className="text-slate-400 uppercase font-bold">Click Volume</span>
                        <span className="text-white font-bold">{Math.round(metroVolume * 100)}%</span>
                      </div>
                      <input
                        type="range"
                        min="0"
                        max="1.5"
                        step="0.05"
                        value={metroVolume}
                        onChange={(e) => {
                          const v = parseFloat(e.target.value);
                          setMetroVolume(v);
                          api.looperSetMetronomeVolume(v).catch(console.error);
                        }}
                        className="w-full accent-akai-red cursor-pointer h-2 bg-chassis-750 rounded-lg appearance-none mt-2"
                      />
                    </div>

                    {/* Mode Selector */}
                    <div className="flex flex-col gap-1.5">
                      <span className="text-[10px] text-slate-400 uppercase font-bold">Trigger Mode</span>
                      <div className="grid grid-cols-2 gap-1 bg-chassis-800 p-1 rounded-lg border border-chassis-700">
                        <button
                          onClick={() => {
                            setMetroMode('while_running');
                            api.looperSetMetronomeMode('while_running').catch(console.error);
                          }}
                          className={`py-1 px-1 rounded text-[10px] font-bold text-center transition-all ${
                            metroMode === 'while_running'
                              ? 'bg-akai-red text-white'
                              : 'text-slate-400 hover:text-white'
                          }`}
                        >
                          Play / Rec
                        </button>
                        <button
                          onClick={() => {
                            setMetroMode('always');
                            api.looperSetMetronomeMode('always').catch(console.error);
                          }}
                          className={`py-1 px-1 rounded text-[10px] font-bold text-center transition-all ${
                            metroMode === 'always'
                              ? 'bg-akai-red text-white'
                              : 'text-slate-400 hover:text-white'
                          }`}
                        >
                          Always On
                        </button>
                      </div>
                    </div>
                  </div>

                  <p className="text-[10px] text-slate-400">
                    The metronome click uses a sample-accurate synthesized high/low woodblock sound. It is mixed directly into the headphone/speaker monitor output so it is audible while recording, but is mathematically isolated from looper recording buffers and WAV exports.
                  </p>
                </div>
              </div>

              {/* MIDI Controller Section */}
              <div className="flex flex-col gap-3">
                <div className="flex items-center justify-between">
                  <h4 className="font-bold text-slate-200 uppercase text-xs flex items-center gap-2">
                    <Cpu className="w-4 h-4 text-akai-red" />
                    MIDI Input Controller
                  </h4>
                  <button
                    onClick={loadDevices}
                    className="flex items-center gap-1.5 text-[10px] text-slate-400 hover:text-white px-2 py-1 rounded bg-chassis-900 border border-chassis-750"
                  >
                    <RefreshCw className="w-3 h-3" />
                    <span>Scan Devices</span>
                  </button>
                </div>

                <div className="flex flex-col gap-2">
                  {midiDevices.length > 0 ? (
                    midiDevices.map((dev) => {
                      const isConnected = systemStatus?.midi_device?.toLowerCase() === dev.name.toLowerCase();
                      return (
                        <div
                          key={dev.index}
                          className="bg-chassis-900 border border-chassis-750 p-3 rounded-xl flex items-center justify-between gap-3"
                        >
                          <div className="flex items-center gap-3">
                            <span
                              className={`w-2.5 h-2.5 rounded-full ${
                                isConnected ? 'bg-emerald-500 shadow-[0_0_8px_#2cf583]' : 'bg-slate-600'
                              }`}
                            />
                            <div>
                              <div className="font-bold text-slate-200 flex items-center gap-2">
                                <span>{dev.name}</span>
                                {dev.is_akai && (
                                  <span className="px-1.5 py-0.2 rounded text-[9px] bg-akai-red/20 text-akai-red border border-akai-red/50">
                                    Akai Hardware
                                  </span>
                                )}
                              </div>
                              <span className="text-[10px] text-slate-500">Port Index: {dev.index}</span>
                            </div>
                          </div>

                          <div>
                            {isConnected ? (
                              <button
                                onClick={handleDisconnectMidi}
                                className="px-3 py-1.5 rounded-lg bg-chassis-800 hover:bg-rose-950/60 text-slate-300 hover:text-rose-400 border border-chassis-700 text-xs active:scale-95"
                              >
                                Disconnect
                              </button>
                            ) : (
                              <button
                                onClick={() => handleConnectMidi(dev.name)}
                                className="px-3 py-1.5 rounded-lg bg-akai-red hover:bg-akai-redBright text-white font-bold text-xs active:scale-95 shadow"
                              >
                                Connect
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })
                  ) : (
                    <div className="bg-chassis-900 border border-chassis-750 p-4 rounded-xl text-center text-slate-500">
                      No hardware MIDI devices currently detected. Plug in your MPK mini 3 via USB and click Scan.
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* TAB 2: SoundFont Manager */}
          {activeTab === 'soundfonts' && <SoundFontManager />}

          {/* TAB 3: MIDI Mapping Reference */}
          {activeTab === 'mapping' && (
            <div className="flex flex-col gap-4">
              <h4 className="font-bold text-slate-200 uppercase text-xs">
                Akai MPK mini 3 Standard Hardware Mapping
              </h4>

              <div className="grid grid-cols-2 gap-4">
                {/* Bank A Mapping */}
                <div className="bg-chassis-900 border border-chassis-750 p-3 rounded-xl flex flex-col gap-2">
                  <span className="font-bold text-akai-red text-xs">BANK A (Channel 10)</span>
                  <div className="space-y-1 text-[11px] text-slate-300">
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>Pad 1 (Kick)</span>
                      <span className="text-slate-500">MIDI #36 (C1)</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>Pad 2 (Snare)</span>
                      <span className="text-slate-500">MIDI #38 (D1)</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>Pad 3 (Closed Hat)</span>
                      <span className="text-slate-500">MIDI #42 (F#1)</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>Pad 4 (Open Hat)</span>
                      <span className="text-slate-500">MIDI #46 (A#1)</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>Pad 5 (Low Tom)</span>
                      <span className="text-slate-500">MIDI #45 (A1)</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>Pad 6 (Mid Tom)</span>
                      <span className="text-slate-500">MIDI #48 (C2)</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>Pad 7 (Crash)</span>
                      <span className="text-slate-500">MIDI #49 (C#2)</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Pad 8 (Ride)</span>
                      <span className="text-slate-500">MIDI #51 (D#2)</span>
                    </div>
                  </div>
                </div>

                {/* Rotary Knobs Mapping */}
                <div className="bg-chassis-900 border border-chassis-750 p-3 rounded-xl flex flex-col gap-2">
                  <span className="font-bold text-amber-400 text-xs">ROTARY MACROS (CC 70 - 77)</span>
                  <div className="space-y-1 text-[11px] text-slate-300">
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>K1: Filter Cutoff</span>
                      <span className="text-slate-500">CC 70</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>K2: Resonance</span>
                      <span className="text-slate-500">CC 71</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>K3: Stereo Chorus</span>
                      <span className="text-slate-500">CC 72</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>K4: Reverb</span>
                      <span className="text-slate-500">CC 73</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>K5: Soft Overdrive</span>
                      <span className="text-slate-500">CC 74</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>K6: Keys Gain</span>
                      <span className="text-slate-500">CC 75</span>
                    </div>
                    <div className="flex justify-between border-b border-chassis-800 pb-0.5">
                      <span>K7: Drums Gain</span>
                      <span className="text-slate-500">CC 76</span>
                    </div>
                    <div className="flex justify-between">
                      <span>K8: Master Gain</span>
                      <span className="text-slate-500">CC 77</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Modal Footer */}
        <div className="px-6 py-3 border-t border-chassis-750 bg-chassis-900 flex items-center justify-between text-slate-500 font-mono text-[11px]">
          <span>MiniStation v0.1.0 &bull; Akai Performance Workstation</span>
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-lg bg-chassis-750 hover:bg-chassis-700 text-slate-200 font-bold text-xs border border-chassis-600 active:scale-95"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
