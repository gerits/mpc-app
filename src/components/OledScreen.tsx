import React from 'react';
import { Music, Disc, Activity } from 'lucide-react';
import type { SystemStatus, LooperSnapshot } from '../types/ipc';

interface OledScreenProps {
  status: SystemStatus | null;
  looperSnap: LooperSnapshot | null;
  className?: string;
}

export const OledScreen: React.FC<OledScreenProps> = ({
  status,
  looperSnap,
  className = '',
}) => {
  const isMidiConnected = Boolean(status?.midi_device);
  const latencyMs = status?.buffer_size && status?.sample_rate
    ? ((status.buffer_size / status.sample_rate) * 1000).toFixed(1)
    : '5.8';

  const melodicPreset = status?.active_melodic_preset ?? 'GRAND PIANO';
  const drumPreset = status?.active_drum_preset ?? 'TR-808 KIT';

  return (
    <div
      className={`relative bg-[#070d0a] border-2 border-[#16291e] rounded-md px-3.5 py-2 font-mono shadow-[inset_0_2px_8px_rgba(0,0,0,0.8)] overflow-hidden ${className}`}
    >
      {/* Subtle CRT Scanline Overlay */}
      <div className="pointer-events-none absolute inset-0 bg-scanlines opacity-20" />

      {/* Screen Header Line */}
      <div className="flex items-center justify-between text-[11px] border-b border-[#16291e] pb-1 mb-1.5">
        <div className="flex items-center gap-1.5">
          <span
            className={`w-2 h-2 rounded-full ${
              isMidiConnected
                ? 'bg-[#2cf583] shadow-[0_0_6px_#2cf583]'
                : 'bg-[#f5a623] shadow-[0_0_6px_#f5a623] animate-pulse'
            }`}
          />
          <span className="text-slate-400 font-bold">MIDI:</span>
          <span className={isMidiConnected ? 'text-[#2cf583] font-bold tracking-wide' : 'text-[#f5a623]'}>
            {isMidiConnected ? status?.midi_device?.toUpperCase() : 'NO CONTROLLER (STANDALONE)'}
          </span>
        </div>

        <div className="flex items-center gap-2 text-slate-400 text-[10px]">
          <span className="flex items-center gap-1">
            <Activity className="w-3 h-3 text-[#2cf583]" />
            <span className="text-[#2cf583] font-semibold">{latencyMs}ms</span>
          </span>
          <span>@</span>
          <span className="text-slate-300">{(status?.sample_rate ?? 44100) / 1000}kHz</span>
        </div>
      </div>

      {/* Main Preset Readout */}
      <div className="grid grid-cols-2 gap-3 text-xs">
        {/* Melodic Program */}
        <div className="flex items-center gap-2">
          <Music className="w-3.5 h-3.5 text-[#2cf583] shrink-0" />
          <div className="truncate">
            <div className="text-[9px] uppercase tracking-wider text-slate-500 font-semibold">KEYS (CH 1)</div>
            <div className="text-[#2cf583] font-bold truncate tracking-wide">
              {melodicPreset.toUpperCase()}
            </div>
          </div>
        </div>

        {/* Drum Kit Program */}
        <div className="flex items-center gap-2 border-l border-[#16291e] pl-3">
          <Disc className="w-3.5 h-3.5 text-[#f5a623] shrink-0" />
          <div className="truncate">
            <div className="text-[9px] uppercase tracking-wider text-slate-500 font-semibold">PADS (CH 10)</div>
            <div className="text-[#f5a623] font-bold truncate tracking-wide">
              {drumPreset.toUpperCase()}
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Transport Progress Line */}
      {looperSnap && (
        <div className="mt-1.5 pt-1 border-t border-[#16291e] flex items-center justify-between text-[10px] text-slate-400">
          <span className="text-[#2cf583]">
            {looperSnap.bpm.toFixed(0)} BPM &bull; {looperSnap.bar_count} BARS
          </span>
          <span>
            BAR <strong className="text-slate-200">{looperSnap.current_bar}</strong> BEAT{' '}
            <strong className="text-slate-200">{looperSnap.current_beat}</strong>
          </span>
        </div>
      )}
    </div>
  );
};
