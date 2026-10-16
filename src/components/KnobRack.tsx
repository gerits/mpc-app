import React, { useState, useEffect } from 'react';
import { RotaryKnob } from './RotaryKnob';
import { api, onMidiEvent, type RoutedEvent } from '../types/ipc';

interface KnobConfig {
  index: number;
  label: string;
  cc: number;
  defaultValue: number;
  format: (val: number) => string;
}

const KNOB_CONFIGS: KnobConfig[] = [
  {
    index: 0,
    label: 'Cutoff',
    cc: 70,
    defaultValue: 0.8,
    format: (v) => {
      const hz = 20 * Math.pow(1000, v);
      return hz >= 1000 ? `${(hz / 1000).toFixed(1)} kHz` : `${Math.round(hz)} Hz`;
    },
  },
  {
    index: 1,
    label: 'Resonance',
    cc: 71,
    defaultValue: 0.1,
    format: (v) => `Q ${(0.7 + v * 7.3).toFixed(1)}`,
  },
  {
    index: 2,
    label: 'Chorus',
    cc: 72,
    defaultValue: 0.0,
    format: (v) => `${Math.round(v * 100)}%`,
  },
  {
    index: 3,
    label: 'Reverb',
    cc: 73,
    defaultValue: 0.25,
    format: (v) => `${Math.round(v * 100)}%`,
  },
  {
    index: 4,
    label: 'Drive',
    cc: 74,
    defaultValue: 0.0,
    format: (v) => `${Math.round(v * 100)}%`,
  },
  {
    index: 5,
    label: 'Keys Vol',
    cc: 75,
    defaultValue: 0.5,
    format: (v) => `${Math.round(v * 200)}%`,
  },
  {
    index: 6,
    label: 'Drums Vol',
    cc: 76,
    defaultValue: 0.5,
    format: (v) => `${Math.round(v * 200)}%`,
  },
  {
    index: 7,
    label: 'Master Out',
    cc: 77,
    defaultValue: 0.45,
    format: (v) => `${Math.round(v * 200)}%`,
  },
];

export const KnobRack: React.FC = () => {
  const [knobs, setKnobs] = useState<number[]>([0.8, 0.1, 0.0, 0.25, 0.0, 0.5, 0.5, 0.45]);

  useEffect(() => {
    // Initial fetch from backend
    api.getMacroParameters()
      .then((vals) => {
        if (vals && vals.length === 8) {
          setKnobs(Array.from(vals));
        }
      })
      .catch(console.error);

    // Realtime hardware MIDI CC 70..77 sync
    let unlisten: (() => void) | undefined;
    onMidiEvent((event: RoutedEvent) => {
      if (event.type === 'MacroKnob') {
        const { knob_index, value } = event.data;
        setKnobs((prev) => {
          const next = [...prev];
          if (knob_index < next.length) {
            next[knob_index] = value;
          }
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

  const handleKnobChange = (index: number, val: number) => {
    setKnobs((prev) => {
      const next = [...prev];
      next[index] = val;
      return next;
    });
    api.setMacroParameter(index, val).catch(console.error);
  };

  return (
    <div className="flex flex-col gap-2">
      {/* Top Header */}
      <div className="flex items-center justify-between pb-2 border-b border-chassis-750 font-mono">
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-bold uppercase tracking-wider text-slate-300">
            ROTARY MACROS (K1 - K8)
          </span>
          <span className="text-[10px] text-slate-500">CC 70 - 77</span>
        </div>
        <span className="text-[10px] text-slate-500 font-semibold">270° SWEEP</span>
      </div>

      {/* 2x4 Knobs Grid */}
      <div className="grid grid-cols-4 gap-y-2 gap-x-1 pt-1">
        {KNOB_CONFIGS.map((config) => (
          <RotaryKnob
            key={config.index}
            knobIndex={config.index}
            label={config.label}
            value={knobs[config.index] ?? config.defaultValue}
            formatValue={config.format}
            defaultValue={config.defaultValue}
            onChange={handleKnobChange}
          />
        ))}
      </div>
    </div>
  );
};
