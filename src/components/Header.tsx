import React from 'react';
import { Volume2, AlertOctagon, Settings } from 'lucide-react';
import { OledScreen } from './OledScreen';
import { Meter } from './Meter';
import type { SystemStatus, LooperSnapshot } from '../types/ipc';

interface HeaderProps {
  status: SystemStatus | null;
  looperSnap: LooperSnapshot | null;
  masterVolume: number;
  onMasterVolumeChange: (vol: number) => void;
  onPanic: () => void;
  onOpenSettings: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  status,
  looperSnap,
  masterVolume,
  onMasterVolumeChange,
  onPanic,
  onOpenSettings,
}) => {
  return (
    <header className="relative bg-chassis-850 border-b-2 border-akai-red px-5 py-2.5 flex items-center justify-between gap-4 select-none shadow-md">
      {/* Chrome Corner Screw Accents */}
      <div className="absolute top-2 left-2 w-2 h-2 rounded-full bg-chassis-600 border border-chassis-500 shadow-inner" />
      <div className="absolute top-2 right-2 w-2 h-2 rounded-full bg-chassis-600 border border-chassis-500 shadow-inner" />

      {/* Brand & Badge */}
      <div className="flex items-center gap-3 shrink-0">
        <div className="flex items-center">
          <div className="bg-white text-chassis-900 font-extrabold text-base tracking-tighter px-1.5 py-0.5 rounded-l">
            MINI
          </div>
          <div className="bg-akai-red text-white font-extrabold text-base tracking-wider px-1.5 py-0.5 rounded-r shadow-md">
            STATION
          </div>
        </div>
        <div className="hidden lg:flex flex-col">
          <span className="text-[10px] font-mono tracking-widest text-slate-400 font-bold uppercase">
            Performance Workstation
          </span>
          <span className="text-[9px] font-mono text-akai-red font-semibold">
            AKAI MPK MINI 3 EDITION
          </span>
        </div>
      </div>

      {/* Center Vintage Phosphor OLED Display */}
      <div className="flex-1 max-w-2xl mx-auto">
        <OledScreen status={status} looperSnap={looperSnap} />
      </div>

      {/* Right Controls: Master Volume, Stereo Meter, Panic, and Settings */}
      <div className="flex items-center gap-4 shrink-0">
        {/* Master Output Level & Fader */}
        <div className="flex items-center gap-2 bg-chassis-900 border border-chassis-750 px-3 py-1.5 rounded-lg shadow-inner">
          <Volume2 className="w-4 h-4 text-slate-400 shrink-0" />
          <div className="flex flex-col gap-0.5">
            <div className="flex items-center justify-between text-[10px] font-mono text-slate-400">
              <span>MASTER</span>
              <span className="text-white font-bold">{Math.round(masterVolume * 100)}%</span>
            </div>
            <input
              type="range"
              min="0"
              max="1.5"
              step="0.01"
              value={masterVolume}
              onChange={(e) => onMasterVolumeChange(parseFloat(e.target.value))}
              className="w-20 h-1.5 bg-chassis-700 rounded-lg appearance-none cursor-pointer accent-akai-red"
            />
          </div>
        </div>

        {/* Dual Stereo LED Meter */}
        <Meter />

        {/* Panic Button */}
        <button
          onClick={onPanic}
          title="Panic: Silence all voices immediately"
          className="flex items-center gap-1.5 px-3 py-2 rounded bg-chassis-750 hover:bg-rose-950/60 text-slate-300 hover:text-akai-red border border-chassis-650 hover:border-akai-red transition-all text-xs font-mono font-bold active:scale-95 shadow"
        >
          <AlertOctagon className="w-3.5 h-3.5 text-akai-red" />
          <span className="hidden sm:inline">PANIC</span>
        </button>

        {/* Settings Dialog Trigger */}
        <button
          onClick={onOpenSettings}
          title="Open Audio, MIDI & SoundFont Settings"
          className="p-2 rounded bg-chassis-750 hover:bg-chassis-700 text-slate-300 hover:text-white border border-chassis-650 transition-all active:scale-95 shadow"
        >
          <Settings className="w-4 h-4 text-slate-300" />
        </button>
      </div>
    </header>
  );
};
