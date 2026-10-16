import React, { useState, useEffect } from 'react';
import { RubberPad } from './RubberPad';
import { onMidiEvent, type RoutedEvent } from '../types/ipc';

interface PadDefinition {
  padNumber: number;
  name: string;
  note: number;
}

const BANK_A_PADS: PadDefinition[] = [
  { padNumber: 1, name: 'Kick', note: 36 },
  { padNumber: 2, name: 'Snare', note: 38 },
  { padNumber: 3, name: 'Closed Hat', note: 42 },
  { padNumber: 4, name: 'Open Hat', note: 46 },
  { padNumber: 5, name: 'Low Tom', note: 45 },
  { padNumber: 6, name: 'Mid Tom', note: 48 },
  { padNumber: 7, name: 'Crash', note: 49 },
  { padNumber: 8, name: 'Ride', note: 51 },
];

const BANK_B_PADS: PadDefinition[] = [
  { padNumber: 1, name: 'Rimshot', note: 37 },
  { padNumber: 2, name: 'Clap', note: 39 },
  { padNumber: 3, name: 'Pedal Hat', note: 44 },
  { padNumber: 4, name: 'Shaker', note: 70 },
  { padNumber: 5, name: 'High Tom', note: 50 },
  { padNumber: 6, name: 'Splash', note: 55 },
  { padNumber: 7, name: 'Cowbell', note: 56 },
  { padNumber: 8, name: 'Tambourine', note: 54 },
];

export const PadMatrix: React.FC = () => {
  const [bank, setBank] = useState<'A' | 'B'>('A');
  const [fullLevel, setFullLevel] = useState<boolean>(true);
  const [activePads, setActivePads] = useState<Set<number>>(new Set());

  const currentPads = bank === 'A' ? BANK_A_PADS : BANK_B_PADS;

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onMidiEvent((event: RoutedEvent) => {
      if (event.type === 'DrumPadOn') {
        const idx = event.data.pad_index;
        setActivePads((prev) => new Set(prev).add(idx));
      } else if (event.type === 'DrumPadOff') {
        const idx = event.data.pad_index;
        setActivePads((prev) => {
          const next = new Set(prev);
          next.delete(idx);
          return next;
        });
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Top row: Pads 5, 6, 7, 8 (Indices 4, 5, 6, 7)
  const topRow = [currentPads[4], currentPads[5], currentPads[6], currentPads[7]];
  // Bottom row: Pads 1, 2, 3, 4 (Indices 0, 1, 2, 3)
  const bottomRow = [currentPads[0], currentPads[1], currentPads[2], currentPads[3]];

  return (
    <div className="flex flex-col gap-3">
      {/* Top Controls: Bank Selector & Full Level Mode */}
      <div className="flex items-center justify-between pb-2 border-b border-chassis-750">
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-bold uppercase tracking-wider text-slate-300 font-mono">
            MPC PADS
          </span>
          <span className="text-[10px] text-slate-500 font-mono">(CH 10)</span>
        </div>

        <div className="flex items-center gap-2 font-mono text-xs">
          {/* Bank A / B Switch */}
          <div className="flex items-center bg-chassis-900 rounded p-0.5 border border-chassis-700">
            <button
              onClick={() => setBank('A')}
              className={`px-2.5 py-0.5 rounded text-[11px] font-bold transition-all ${
                bank === 'A'
                  ? 'bg-akai-red text-white shadow'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              BANK A
            </button>
            <button
              onClick={() => setBank('B')}
              className={`px-2.5 py-0.5 rounded text-[11px] font-bold transition-all ${
                bank === 'B'
                  ? 'bg-amber-500 text-chassis-950 shadow'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              BANK B
            </button>
          </div>

          {/* Full Level Button */}
          <button
            onClick={() => setFullLevel(!fullLevel)}
            className={`px-2.5 py-1 rounded text-[10px] font-bold transition-all border ${
              fullLevel
                ? 'bg-akai-red/20 border-akai-red text-akai-red'
                : 'bg-chassis-900 border-chassis-700 text-slate-400 hover:text-slate-300'
            }`}
          >
            FULL LEVEL: {fullLevel ? 'ON' : 'OFF'}
          </button>
        </div>
      </div>

      {/* 2x4 Rubber Pad Matrix Grid */}
      <div className="grid grid-cols-4 gap-2.5">
        {/* Top Row: Pads 5, 6, 7, 8 */}
        {topRow.map((pad, offset) => {
          const padIdx = offset + 4;
          return (
            <RubberPad
              key={`pad-${pad.padNumber}`}
              padIndex={padIdx}
              padNumber={pad.padNumber}
              name={pad.name}
              note={pad.note}
              isActive={activePads.has(padIdx)}
              fullLevel={fullLevel}
            />
          );
        })}

        {/* Bottom Row: Pads 1, 2, 3, 4 */}
        {bottomRow.map((pad, padIdx) => (
          <RubberPad
            key={`pad-${pad.padNumber}`}
            padIndex={padIdx}
            padNumber={pad.padNumber}
            name={pad.name}
            note={pad.note}
            isActive={activePads.has(padIdx)}
            fullLevel={fullLevel}
          />
        ))}
      </div>
    </div>
  );
};
