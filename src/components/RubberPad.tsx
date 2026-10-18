import React, { useState } from 'react';
import { api } from '../types/ipc';

interface RubberPadProps {
  padIndex: number; // 0..=7
  padNumber: number; // 1..=8
  name: string;
  note: number;
  isActive: boolean;
  fullLevel: boolean;
  onTrigger?: (padIndex: number) => void;
}

export const RubberPad: React.FC<RubberPadProps> = ({
  padIndex,
  padNumber,
  name,
  note,
  isActive,
  fullLevel,
  onTrigger,
}) => {
  const [isPressed, setIsPressed] = useState(false);
  const active = isActive || isPressed;

  const handlePointerDown = (e: React.PointerEvent) => {
    e.preventDefault();
    setIsPressed(true);
    const vel = fullLevel ? 127 : 110;
    api.sendPadOn(padIndex, note, vel).catch(console.error);
    if (onTrigger) onTrigger(padIndex);
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    e.preventDefault();
    setIsPressed(false);
    api.sendPadOff(padIndex, note).catch(console.error);
  };

  const handlePointerLeave = () => {
    if (isPressed) {
      setIsPressed(false);
      api.sendPadOff(padIndex, note).catch(console.error);
    }
  };

  return (
    <div
      onPointerDown={handlePointerDown}
      onPointerUp={handlePointerUp}
      onPointerLeave={handlePointerLeave}
      className={`relative select-none cursor-pointer rounded-lg p-2.5 flex flex-col justify-between transition-all duration-75 border-2 ${
        active
          ? 'bg-akai-red border-white shadow-[0_0_24px_rgba(229,37,33,0.9)] text-white scale-[0.98]'
          : 'bg-[#21242b] hover:bg-[#282c35] border-[#313742] text-slate-300 akai-pad-shadow hover:border-slate-500'
      }`}
      style={{
        aspectRatio: '1 / 1',
        minHeight: '84px',
      }}
    >
      {/* Top Header: Pad Number */}
      <div className="flex items-center justify-start text-[10px] font-mono font-bold">
        <span className={active ? 'text-white' : 'text-slate-400'}>PAD {padNumber}</span>
      </div>

      {/* Center Pad Name / Drum Instrument */}
      <div className="flex flex-col items-center justify-center text-center my-auto">
        <span
          className={`text-xs font-bold tracking-wider uppercase leading-tight ${
            active ? 'text-white drop-shadow' : 'text-slate-200'
          }`}
        >
          {name}
        </span>
      </div>

      {/* Bottom Backlit LED Line Indicator */}
      <div
        className={`h-1 w-full rounded-full transition-all ${
          active
            ? 'bg-white shadow-[0_0_8px_#ffffff]'
            : 'bg-akai-red/40'
        }`}
      />
    </div>
  );
};
