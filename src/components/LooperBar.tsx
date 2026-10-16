import React, { useState, useRef } from 'react';
import {
  Play,
  Square,
  Circle,
  Download,
  Trash2,
  VolumeX,
  Volume2,
  Radio,
} from 'lucide-react';
import { api, type LooperSnapshot } from '../types/ipc';

interface LooperBarProps {
  snapshot: LooperSnapshot | null;
}

export const LooperBar: React.FC<LooperBarProps> = ({ snapshot }) => {
  const [selectedTrack, setSelectedTrack] = useState<number>(0); // 0 = Track 1, 1 = Track 2
  const [bpm, setBpm] = useState<number>(snapshot?.bpm ?? 120);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState<boolean>(false);

  // Tap tempo state
  const lastTapTime = useRef<number>(0);
  const tapDiffs = useRef<number[]>([]);

  const t1State = snapshot?.track1_state ?? 'empty';
  const t2State = snapshot?.track2_state ?? 'empty';
  const isRunning = snapshot?.is_running ?? false;
  const currentBar = snapshot?.current_bar ?? 1;
  const currentBeat = snapshot?.current_beat ?? 1;
  const progress = snapshot?.progress ?? 0;
  const barCount = snapshot?.bar_count ?? 4;

  const currentTrackState = selectedTrack === 0 ? t1State : t2State;

  const handleRecord = () => {
    if (currentTrackState === 'recording') {
      api.looperPlay().catch(console.error);
    } else {
      api.looperRecord(selectedTrack).catch(console.error);
    }
  };

  const handleOverdub = () => {
    api.looperOverdub(selectedTrack).catch(console.error);
  };

  const handlePlay = () => {
    api.looperPlay().catch(console.error);
  };

  const handleStop = () => {
    api.looperStop().catch(console.error);
  };

  const handleClearCurrent = () => {
    api.looperClear(selectedTrack).catch(console.error);
  };

  const handleClearAll = () => {
    api.looperClearAll().catch(console.error);
  };

  const handleToggleMute = (trackIdx: number) => {
    api.looperToggleMute(trackIdx).catch(console.error);
  };

  const handleBarsChange = (bars: number) => {
    api.looperSetBars(bars).catch(console.error);
  };

  const handleBpmChange = (newBpm: number) => {
    const clamped = Math.max(40, Math.min(260, newBpm));
    setBpm(clamped);
    api.looperSetBpm(clamped).catch(console.error);
  };

  const handleTapTempo = () => {
    const now = performance.now();
    if (lastTapTime.current > 0) {
      const diff = now - lastTapTime.current;
      if (diff > 200 && diff < 2000) {
        tapDiffs.current.push(diff);
        if (tapDiffs.current.length > 4) tapDiffs.current.shift();
        const avg = tapDiffs.current.reduce((a, b) => a + b, 0) / tapDiffs.current.length;
        const calculatedBpm = Math.round(60000 / avg);
        handleBpmChange(calculatedBpm);
      } else {
        tapDiffs.current = [];
      }
    }
    lastTapTime.current = now;
  };

  const handleExportWav = async () => {
    setIsExporting(true);
    setExportMessage(null);
    try {
      const path = await api.looperExport();
      setExportMessage(`Saved 24-bit WAV to: ${path}`);
      setTimeout(() => setExportMessage(null), 6000);
    } catch (err) {
      setExportMessage(`Export error: ${err}`);
      setTimeout(() => setExportMessage(null), 5000);
    } finally {
      setIsExporting(false);
    }
  };

  const renderStateBadge = (state: string) => {
    switch (state) {
      case 'recording':
        return (
          <span className="px-1.5 py-0.5 rounded text-[9px] font-bold bg-akai-red text-white animate-pulse">
            REC
          </span>
        );
      case 'overdubbing':
        return (
          <span className="px-1.5 py-0.5 rounded text-[9px] font-bold bg-amber-500 text-chassis-950 animate-pulse">
            DUB
          </span>
        );
      case 'playing':
        return (
          <span className="px-1.5 py-0.5 rounded text-[9px] font-bold bg-emerald-500 text-chassis-950">
            PLAY
          </span>
        );
      case 'muted':
        return (
          <span className="px-1.5 py-0.5 rounded text-[9px] font-bold bg-slate-700 text-slate-300">
            MUTED
          </span>
        );
      default:
        return (
          <span className="px-1.5 py-0.5 rounded text-[9px] font-bold bg-chassis-750 text-slate-500">
            EMPTY
          </span>
        );
    }
  };

  return (
    <div className="bg-chassis-850 border border-chassis-750 rounded-xl p-3 flex flex-col gap-2.5 shadow-lg select-none">
      {/* Top Track Switcher Tabs and Export Notification */}
      <div className="flex items-center justify-between pb-2 border-b border-chassis-750 gap-2 font-mono">
        <div className="flex items-center gap-2">
          {/* Track 1 Tab */}
          <button
            onClick={() => setSelectedTrack(0)}
            className={`flex items-center gap-2 px-3 py-1 rounded-md text-xs font-bold transition-all border ${
              selectedTrack === 0
                ? 'bg-chassis-750 border-akai-red text-white shadow'
                : 'bg-chassis-900 border-chassis-700 text-slate-400 hover:text-white'
            }`}
          >
            <span>TRACK 1 (DRUMS)</span>
            {renderStateBadge(t1State)}
          </button>

          {/* Track 2 Tab */}
          <button
            onClick={() => setSelectedTrack(1)}
            className={`flex items-center gap-2 px-3 py-1 rounded-md text-xs font-bold transition-all border ${
              selectedTrack === 1
                ? 'bg-chassis-750 border-akai-red text-white shadow'
                : 'bg-chassis-900 border-chassis-700 text-slate-400 hover:text-white'
            }`}
          >
            <span>TRACK 2 (KEYS)</span>
            {renderStateBadge(t2State)}
          </button>

          {/* Quick Mute Toggles */}
          <button
            onClick={() => handleToggleMute(0)}
            title="Toggle Mute Track 1"
            className={`p-1.5 rounded border transition-all ${
              t1State === 'muted'
                ? 'bg-rose-950/80 border-rose-700 text-rose-300'
                : 'bg-chassis-900 border-chassis-700 text-slate-400 hover:text-white'
            }`}
          >
            {t1State === 'muted' ? <VolumeX className="w-3.5 h-3.5" /> : <Volume2 className="w-3.5 h-3.5" />}
          </button>
          <button
            onClick={() => handleToggleMute(1)}
            title="Toggle Mute Track 2"
            className={`p-1.5 rounded border transition-all ${
              t2State === 'muted'
                ? 'bg-rose-950/80 border-rose-700 text-rose-300'
                : 'bg-chassis-900 border-chassis-700 text-slate-400 hover:text-white'
            }`}
          >
            {t2State === 'muted' ? <VolumeX className="w-3.5 h-3.5" /> : <Volume2 className="w-3.5 h-3.5" />}
          </button>
        </div>

        {/* Bar Length Selector (1, 2, 4, 8) */}
        <div className="flex items-center gap-1.5 text-xs">
          <span className="text-slate-500 text-[10px]">LENGTH:</span>
          <div className="flex items-center bg-chassis-900 rounded p-0.5 border border-chassis-700">
            {[1, 2, 4, 8].map((b) => (
              <button
                key={b}
                onClick={() => handleBarsChange(b)}
                className={`px-2 py-0.5 rounded text-[10px] font-bold transition-all ${
                  barCount === b
                    ? 'bg-akai-red text-white shadow'
                    : 'text-slate-400 hover:text-white'
                }`}
              >
                {b}B
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Main Transport Controls & BPM Row */}
      <div className="flex items-center justify-between gap-4">
        {/* Big Transport Buttons */}
        <div className="flex items-center gap-2">
          {/* Record Button */}
          <button
            onClick={handleRecord}
            title={currentTrackState === 'recording' ? 'Switch to Playback' : 'Record into active track'}
            className={`flex items-center gap-1.5 px-3 py-2 rounded-lg font-mono text-xs font-bold transition-all border shadow ${
              currentTrackState === 'recording'
                ? 'bg-akai-red text-white border-white animate-pulse shadow-[0_0_16px_rgba(229,37,33,0.9)]'
                : 'bg-chassis-900 hover:bg-rose-950/40 text-rose-400 border-rose-900/60 hover:border-akai-red'
            }`}
          >
            <Circle className={`w-3.5 h-3.5 ${currentTrackState === 'recording' ? 'fill-white' : 'fill-rose-500'}`} />
            <span>REC</span>
          </button>

          {/* Overdub Button */}
          <button
            onClick={handleOverdub}
            title="Overdub on top of existing loop"
            className={`flex items-center gap-1.5 px-3 py-2 rounded-lg font-mono text-xs font-bold transition-all border shadow ${
              currentTrackState === 'overdubbing'
                ? 'bg-amber-500 text-chassis-950 border-white animate-pulse shadow-[0_0_16px_rgba(245,166,35,0.9)]'
                : 'bg-chassis-900 hover:bg-amber-950/40 text-amber-400 border-amber-900/60 hover:border-amber-500'
            }`}
          >
            <Radio className="w-3.5 h-3.5" />
            <span>DUB</span>
          </button>

          {/* Play Button */}
          <button
            onClick={handlePlay}
            title="Play active loops"
            className={`flex items-center gap-1.5 px-3.5 py-2 rounded-lg font-mono text-xs font-bold transition-all border shadow ${
              isRunning && currentTrackState !== 'recording'
                ? 'bg-emerald-500 text-chassis-950 border-white shadow-[0_0_16px_rgba(16,185,129,0.9)]'
                : 'bg-chassis-900 hover:bg-emerald-950/40 text-emerald-400 border-emerald-900/60 hover:border-emerald-500'
            }`}
          >
            <Play className={`w-3.5 h-3.5 ${isRunning ? 'fill-chassis-950' : 'fill-emerald-400'}`} />
            <span>PLAY</span>
          </button>

          {/* Stop Button */}
          <button
            onClick={handleStop}
            title="Stop playback and rewind"
            className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-chassis-900 hover:bg-chassis-800 text-slate-300 hover:text-white border border-chassis-700 font-mono text-xs font-bold transition-all active:scale-95 shadow"
          >
            <Square className="w-3.5 h-3.5 fill-slate-300" />
            <span>STOP</span>
          </button>

          {/* Clear Track */}
          <button
            onClick={handleClearCurrent}
            title="Clear active track audio"
            className="p-2 rounded-lg bg-chassis-900 hover:bg-rose-950/50 text-slate-400 hover:text-rose-300 border border-chassis-750 transition-all active:scale-95"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>

          {/* Clear All */}
          <button
            onClick={handleClearAll}
            title="Clear all tracks and reset transport"
            className="px-2 py-1.5 rounded-lg bg-chassis-900 hover:bg-rose-950/50 text-slate-400 hover:text-rose-300 border border-chassis-750 font-mono text-[10px] font-bold transition-all"
          >
            CLEAR ALL
          </button>
        </div>

        {/* Center Loop Progress Visualizer */}
        <div className="flex-1 max-w-xs flex flex-col gap-1 font-mono">
          <div className="flex items-center justify-between text-[10px] text-slate-400">
            <span>
              BAR <strong className="text-white text-xs">{currentBar}</strong> / {barCount}
            </span>
            <span>
              BEAT <strong className="text-white text-xs">{currentBeat}</strong> / 4
            </span>
          </div>

          {/* Smooth Continuous Progress Bar */}
          <div className="h-2 w-full bg-chassis-950 rounded-full overflow-hidden border border-chassis-700">
            <div
              className={`h-full transition-all duration-75 rounded-full ${
                currentTrackState === 'recording'
                  ? 'bg-akai-red shadow-[0_0_8px_#e52521]'
                  : currentTrackState === 'overdubbing'
                  ? 'bg-amber-400 shadow-[0_0_8px_#f5a623]'
                  : 'bg-emerald-500 shadow-[0_0_8px_#2cf583]'
              }`}
              style={{ width: `${Math.round(progress * 100)}%` }}
            />
          </div>
        </div>

        {/* Right Tempo BPM & WAV Export */}
        <div className="flex items-center gap-3">
          {/* BPM Controls */}
          <div className="flex items-center gap-1.5 bg-chassis-900 px-2.5 py-1 rounded-lg border border-chassis-700 font-mono">
            <span className="text-[10px] text-slate-500 font-bold">BPM:</span>
            <input
              type="number"
              min={40}
              max={260}
              value={Math.round(bpm)}
              onChange={(e) => handleBpmChange(parseInt(e.target.value) || 120)}
              className="w-11 bg-transparent text-white font-bold text-xs text-center focus:outline-none"
            />
            <button
              onClick={handleTapTempo}
              title="Click in rhythm to tap tempo"
              className="px-1.5 py-0.5 rounded bg-chassis-800 hover:bg-chassis-700 text-[9px] text-slate-300 font-bold border border-chassis-650 active:scale-95"
            >
              TAP
            </button>
          </div>

          {/* 24-bit WAV Export Button */}
          <button
            onClick={handleExportWav}
            disabled={isExporting}
            className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-akai-red hover:bg-akai-redBright text-white font-mono text-xs font-bold transition-all active:scale-95 shadow disabled:opacity-50"
          >
            <Download className="w-3.5 h-3.5" />
            <span>{isExporting ? 'EXPORTING...' : 'EXPORT WAV'}</span>
          </button>
        </div>
      </div>

      {/* Export Status Toast Banner */}
      {exportMessage && (
        <div className="mt-1 p-2 rounded bg-chassis-900 border border-emerald-600/50 text-emerald-400 font-mono text-[11px] flex items-center justify-between">
          <span className="truncate">{exportMessage}</span>
          <button
            onClick={() => setExportMessage(null)}
            className="text-slate-400 hover:text-white text-xs ml-2"
          >
            &times;
          </button>
        </div>
      )}
    </div>
  );
};
