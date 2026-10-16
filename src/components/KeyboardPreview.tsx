import React, { useState, useEffect } from 'react';
import { api, onMidiEvent, type RoutedEvent } from '../types/ipc';

interface KeyDef {
  midiNote: number;
  noteName: string;
  isBlack: boolean;
}

const NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

export const KeyboardPreview: React.FC = () => {
  // Base octave offset: 48 is C3 (in 0-based MIDI standard octave 3/4)
  const [octaveOffset, setOctaveOffset] = useState<number>(48);
  const [activeNotes, setActiveNotes] = useState<Set<number>>(new Set());
  const [pitchBend, setPitchBend] = useState<number>(0);
  const [modulation, setModulation] = useState<number>(0);

  // Generate 25 keys (2 octaves C to C)
  const keys: KeyDef[] = [];
  for (let i = 0; i <= 24; i++) {
    const midi = octaveOffset + i;
    const noteInOctave = midi % 12;
    const isBlack = [1, 3, 6, 8, 10].includes(noteInOctave);
    const noteName = `${NOTE_NAMES[noteInOctave]}${Math.floor(midi / 12) - 1}`;
    keys.push({ midiNote: midi, noteName, isBlack });
  }

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onMidiEvent((event: RoutedEvent) => {
      if (event.type === 'KeyboardNoteOn') {
        const note = event.data.note;
        setActiveNotes((prev) => new Set(prev).add(note));
      } else if (event.type === 'KeyboardNoteOff') {
        const note = event.data.note;
        setActiveNotes((prev) => {
          const next = new Set(prev);
          next.delete(note);
          return next;
        });
      } else if (event.type === 'PitchBend') {
        setPitchBend(event.data.value);
      } else if (event.type === 'Modulation') {
        setModulation(event.data.value);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handlePointerDown = (note: number) => {
    setActiveNotes((prev) => new Set(prev).add(note));
    api.sendNoteOn(0, note, 100).catch(console.error);
  };

  const handlePointerUp = (note: number) => {
    setActiveNotes((prev) => {
      const next = new Set(prev);
      next.delete(note);
      return next;
    });
    api.sendNoteOff(0, note).catch(console.error);
  };

  const handleOctaveDown = () => {
    if (octaveOffset > 24) setOctaveOffset((o) => o - 12);
  };

  const handleOctaveUp = () => {
    if (octaveOffset < 84) setOctaveOffset((o) => o + 12);
  };

  const octaveLabel = `OCT ${Math.floor(octaveOffset / 12) - 1}`;

  return (
    <div className="flex flex-col gap-2.5 select-none">
      {/* Top Header: Octave Shift & Joystick Readout */}
      <div className="flex items-center justify-between pb-2 border-b border-chassis-750 font-mono text-xs">
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-bold uppercase tracking-wider text-slate-300">
            25-KEY MINI KEYBOARD
          </span>
          <span className="text-[10px] text-slate-500">(CH 1)</span>
        </div>

        <div className="flex items-center gap-3">
          {/* Pitch & Mod Indicators */}
          <div className="hidden sm:flex items-center gap-2 text-[10px] text-slate-400 bg-chassis-900 px-2 py-0.5 rounded border border-chassis-750">
            <span>PITCH: <strong className="text-white">{pitchBend > 0 ? `+${(pitchBend * 100).toFixed(0)}%` : `${(pitchBend * 100).toFixed(0)}%`}</strong></span>
            <span className="text-slate-600">|</span>
            <span>MOD: <strong className="text-white">{(modulation * 100).toFixed(0)}%</strong></span>
          </div>

          {/* Octave Controls */}
          <div className="flex items-center bg-chassis-900 rounded p-0.5 border border-chassis-700">
            <button
              onClick={handleOctaveDown}
              className="px-2 py-0.5 rounded text-[10px] font-bold text-slate-400 hover:text-white hover:bg-chassis-750 transition-all active:scale-95"
            >
              OCT -
            </button>
            <span className="px-2 text-[10px] font-bold text-[#2cf583]">{octaveLabel}</span>
            <button
              onClick={handleOctaveUp}
              className="px-2 py-0.5 rounded text-[10px] font-bold text-slate-400 hover:text-white hover:bg-chassis-750 transition-all active:scale-95"
            >
              OCT +
            </button>
          </div>
        </div>
      </div>

      {/* 25 Piano Keys Layout */}
      <div className="relative flex h-28 bg-[#101216] p-1 rounded-lg border-2 border-chassis-700 shadow-inner overflow-hidden">
        {/* Render White Keys in Natural Sequence */}
        {keys
          .filter((k) => !k.isBlack)
          .map((k) => {
            const isNoteActive = activeNotes.has(k.midiNote);
            return (
              <button
                key={`white-${k.midiNote}`}
                onPointerDown={() => handlePointerDown(k.midiNote)}
                onPointerUp={() => handlePointerUp(k.midiNote)}
                onPointerLeave={() => {
                  if (activeNotes.has(k.midiNote)) handlePointerUp(k.midiNote);
                }}
                className={`flex-1 h-full rounded-b transition-all flex flex-col justify-end items-center pb-1 text-[9px] font-mono font-bold border-r border-[#cbd5e1]/40 last:border-r-0 ${
                  isNoteActive
                    ? 'bg-akai-red text-white shadow-[0_0_16px_rgba(229,37,33,0.9)] scale-[0.99]'
                    : 'bg-[#f1f5f9] hover:bg-white text-slate-700 shadow-[inset_0_-4px_6px_rgba(0,0,0,0.15)] active:bg-slate-200'
                }`}
              >
                {k.noteName.startsWith('C') ? k.noteName : ''}
              </button>
            );
          })}

        {/* Render Black Keys Absolutely Positioned */}
        <div className="absolute inset-0 pointer-events-none flex px-1">
          {/* Black key positioning based on white key slot offsets */}
          {(() => {
            // White keys count is 15 (2 octaves: 7 + 7 + 1 = 15)
            // Black keys positions relative to 15 slots
            const blackKeySlots = [
              { noteIndex: 1, leftPct: 1 / 15 },      // C#
              { noteIndex: 3, leftPct: 2 / 15 },      // D#
              { noteIndex: 6, leftPct: 4 / 15 },      // F#
              { noteIndex: 8, leftPct: 5 / 15 },      // G#
              { noteIndex: 10, leftPct: 6 / 15 },     // A#
              { noteIndex: 13, leftPct: 8 / 15 },     // C# (2nd oct)
              { noteIndex: 15, leftPct: 9 / 15 },     // D# (2nd oct)
              { noteIndex: 18, leftPct: 11 / 15 },    // F# (2nd oct)
              { noteIndex: 20, leftPct: 12 / 15 },    // G# (2nd oct)
              { noteIndex: 22, leftPct: 13 / 15 },    // A# (2nd oct)
            ];

            return blackKeySlots.map(({ noteIndex, leftPct }) => {
              const k = keys[noteIndex];
              const isNoteActive = activeNotes.has(k.midiNote);

              return (
                <button
                  key={`black-${k.midiNote}`}
                  onPointerDown={(e) => {
                    e.stopPropagation();
                    handlePointerDown(k.midiNote);
                  }}
                  onPointerUp={(e) => {
                    e.stopPropagation();
                    handlePointerUp(k.midiNote);
                  }}
                  onPointerLeave={(e) => {
                    e.stopPropagation();
                    if (activeNotes.has(k.midiNote)) handlePointerUp(k.midiNote);
                  }}
                  className={`pointer-events-auto absolute top-1 w-[4.4%] h-[60%] rounded-b transition-all flex flex-col justify-end items-center pb-1 text-[8px] font-mono font-bold z-10 ${
                    isNoteActive
                      ? 'bg-akai-red text-white shadow-[0_0_16px_rgba(229,37,33,0.9)] scale-[0.98]'
                      : 'bg-[#181a20] hover:bg-[#232730] text-slate-400 border-x border-b border-[#0f1115] shadow-[0_3px_5px_rgba(0,0,0,0.8)]'
                  }`}
                  style={{
                    left: `calc(${leftPct * 100}% - 2.2% + 4px)`,
                  }}
                />
              );
            });
          })()}
        </div>
      </div>
    </div>
  );
};
