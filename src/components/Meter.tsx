import React, { useEffect, useState } from 'react';
import { onAudioMeter, type AudioMeterPayload } from '../types/ipc';

interface MeterProps {
  className?: string;
}

const LED_COUNT = 16;

export const Meter: React.FC<MeterProps> = ({ className = '' }) => {
  const [levels, setLevels] = useState<AudioMeterPayload>({
    peak_left: 0,
    peak_right: 0,
    rms_left: 0,
    rms_right: 0,
  });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onAudioMeter((payload) => {
      setLevels(payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Convert linear amplitude (0.0 to 1.0) into number of active LEDs
  const getActiveLeds = (amplitude: number): number => {
    if (amplitude <= 0.001) return 0;
    // Map -48dB .. 0dB logarithmically
    const db = 20 * Math.log10(Math.min(1.0, amplitude));
    const normalized = Math.max(0, (db + 48) / 48); // 0.0 to 1.0
    return Math.round(normalized * LED_COUNT);
  };

  const leftActive = getActiveLeds(levels.peak_left);
  const rightActive = getActiveLeds(levels.peak_right);

  const renderChannel = (activeCount: number, channelLabel: string) => {
    return (
      <div className="flex items-center gap-1.5">
        <span className="text-[9px] font-mono font-bold text-slate-500 w-2.5">{channelLabel}</span>
        <div className="flex items-center gap-[2px] bg-chassis-900 px-1 py-0.5 rounded border border-chassis-750 shadow-inner">
          {Array.from({ length: LED_COUNT }).map((_, idx) => {
            const isActive = idx < activeCount;
            // First 10 green, next 4 yellow/amber, top 2 red
            let activeColor = 'bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.8)]';
            let inactiveColor = 'bg-emerald-950/40';

            if (idx >= 14) {
              activeColor = 'bg-akai-red shadow-[0_0_6px_rgba(229,37,33,0.9)]';
              inactiveColor = 'bg-rose-950/40';
            } else if (idx >= 10) {
              activeColor = 'bg-amber-400 shadow-[0_0_6px_rgba(251,191,36,0.8)]';
              inactiveColor = 'bg-amber-950/40';
            }

            return (
              <div
                key={idx}
                className={`w-1.5 h-3 rounded-[1px] transition-colors duration-75 ${
                  isActive ? activeColor : inactiveColor
                }`}
              />
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <div className={`flex flex-col gap-1 select-none ${className}`}>
      {renderChannel(leftActive, 'L')}
      {renderChannel(rightActive, 'R')}
    </div>
  );
};
