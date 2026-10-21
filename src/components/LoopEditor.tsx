import React, { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import { api, type LooperSnapshot, type LoopNote } from '../types/ipc';

interface LoopEditorProps {
  snapshot: LooperSnapshot | null;
  selectedTrack: number; // 0 = Drums, 1 = Keys
  onSelectTrack: (track: number) => void;
}

// 8 Standard MPC Drum Pad Definitions (matching PadMatrix Bank A & B standard mappings)
const DRUM_PAD_LANES = [
  { note: 51, name: 'Ride', pad: 'P8', isBlack: false },
  { note: 49, name: 'Crash', pad: 'P7', isBlack: false },
  { note: 46, name: 'Open Hat', pad: 'P4', isBlack: false },
  { note: 42, name: 'Closed Hat', pad: 'P3', isBlack: false },
  { note: 39, name: 'Clap', pad: 'P2', isBlack: false },
  { note: 38, name: 'Snare', pad: 'P2', isBlack: false },
  { note: 37, name: 'Rimshot', pad: 'P1', isBlack: false },
  { note: 36, name: 'Kick', pad: 'P1', isBlack: false },
];

const NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

// 2 Full Octaves C3 to C5 (Notes 48 to 72)
const CHROMATIC_KEYS = (() => {
  const keys = [];
  for (let n = 72; n >= 48; n--) {
    const semitone = n % 12;
    const isBlack = [1, 3, 6, 8, 10].includes(semitone);
    const octave = Math.floor(n / 12) - 1;
    const name = `${NOTE_NAMES[semitone]}${octave}`;
    keys.push({ note: n, name, isBlack });
  }
  return keys;
})();

export const LoopEditor: React.FC<LoopEditorProps> = ({
  snapshot,
  selectedTrack,
  onSelectTrack,
}) => {
  const [snapMode, setSnapMode] = useState<'1/16' | '1/8' | 'off'>('1/16');
  const [isFolded, setIsFolded] = useState<boolean>(true); // Ableton-style fold for keys
  const [optimisticNotes, setOptimisticNotes] = useState<LoopNote[] | null>(null);
  const [isDragging, setIsDragging] = useState<boolean>(false);
  const [activeTimingInfo, setActiveTimingInfo] = useState<string | null>(null);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const gridAreaRef = useRef<HTMLDivElement>(null);

  const barCount = snapshot?.bar_count ?? 4;
  const totalSteps = barCount * 16; // 16 sixteenths per bar
  const progress = snapshot?.progress ?? 0;

  // Track notes from snapshot or optimistic local state
  const serverNotes = useMemo(() => {
    if (selectedTrack === 0) {
      return snapshot?.track1_notes ?? [];
    }
    return snapshot?.track2_notes ?? [];
  }, [selectedTrack, snapshot?.track1_notes, snapshot?.track2_notes]);

  // Sync optimistic state with server notes when not dragging
  useEffect(() => {
    if (!isDragging) {
      setOptimisticNotes(serverNotes);
    }
  }, [serverNotes, isDragging]);

  const currentNotes = optimisticNotes ?? serverNotes;

  // Waveform peaks
  const waveformPeaks = useMemo(() => {
    if (selectedTrack === 0) {
      return snapshot?.waveform_peaks_t1 ?? [];
    }
    return snapshot?.waveform_peaks_t2 ?? [];
  }, [selectedTrack, snapshot?.waveform_peaks_t1, snapshot?.waveform_peaks_t2]);

  // Active lanes computation (with FOLD support for piano roll)
  const lanes = useMemo(() => {
    if (selectedTrack === 0) {
      return DRUM_PAD_LANES;
    }
    if (isFolded && currentNotes.length > 0) {
      const activePitches = new Set(currentNotes.map((n) => n.note));
      const filtered = CHROMATIC_KEYS.filter((k) => activePitches.has(k.note));
      return filtered.length > 0 ? filtered : CHROMATIC_KEYS.slice(8, 20);
    }
    return CHROMATIC_KEYS;
  }, [selectedTrack, isFolded, currentNotes]);

  // Draw background waveform peaks
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    canvas.width = canvas.parentElement?.clientWidth ?? 600;
    canvas.height = canvas.parentElement?.clientHeight ?? 240;
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    if (waveformPeaks.length === 0) return;

    ctx.fillStyle = selectedTrack === 0 ? '#e52521' : '#f5a623';
    ctx.globalAlpha = 0.18;

    const barW = canvas.width / waveformPeaks.length;
    for (let i = 0; i < waveformPeaks.length; i++) {
      const peak = waveformPeaks[i];
      const h = Math.min(canvas.height * 0.85, peak * canvas.height);
      const y = (canvas.height - h) / 2;
      ctx.fillRect(i * barW, y, Math.max(1, barW - 1), h);
    }
    ctx.globalAlpha = 1.0;
  }, [waveformPeaks, selectedTrack]);

  // Quantize action
  const handleQuantize = useCallback(() => {
    const gridStep = snapMode === '1/8' ? 2.0 : 1.0;
    // Optimistic local update
    if (optimisticNotes) {
      setOptimisticNotes((prev) =>
        (prev ?? []).map((n) => ({
          ...n,
          start_step: (Math.round(n.start_step / gridStep) * gridStep) % totalSteps,
          off_beat: false,
        }))
      );
    }
    api.looperQuantize(selectedTrack, gridStep).catch(console.error);
  }, [selectedTrack, snapMode, totalSteps, optimisticNotes]);

  // Clear notes action
  const handleClearNotes = useCallback(() => {
    setOptimisticNotes([]);
    api.looperClearNotes(selectedTrack).catch(console.error);
  }, [selectedTrack]);

  // Delete individual note
  const handleDeleteNote = useCallback((noteId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    setOptimisticNotes((prev) => (prev ?? []).filter((n) => n.id !== noteId));
    api.looperDeleteNote(selectedTrack, noteId).catch(console.error);
  }, [selectedTrack]);

  // Interactive Note Dragging
  const handleMouseDownNote = (
    note: LoopNote,
    e: React.MouseEvent<HTMLDivElement>
  ) => {
    e.stopPropagation();
    e.preventDefault();
    setIsDragging(true);

    const startX = e.clientX;
    const startY = e.clientY;
    const initialStep = note.start_step;
    const initialNoteNum = note.note;
    const gridEl = gridAreaRef.current;
    if (!gridEl) return;

    let latestStep = initialStep;
    let latestPitch = initialNoteNum;

    const onMouseMove = (moveEvt: MouseEvent) => {
      const gridRect = gridEl.getBoundingClientRect();
      const deltaX = moveEvt.clientX - startX;
      const deltaSteps = (deltaX / gridRect.width) * totalSteps;
      let newStep = Math.max(0, Math.min(totalSteps - 0.2, initialStep + deltaSteps));

      // Snapping
      if (snapMode === '1/16') {
        newStep = Math.round(newStep);
      } else if (snapMode === '1/8') {
        newStep = Math.round(newStep / 2) * 2;
      }

      // Vertical row drag (transpose pitch / pad)
      const deltaY = moveEvt.clientY - startY;
      const rowHeight = 32; // 32px row height
      const rowDelta = Math.round(deltaY / rowHeight);

      const currentLaneIdx = lanes.findIndex((l) => l.note === initialNoteNum);
      let targetLaneIdx = currentLaneIdx !== -1 ? currentLaneIdx + rowDelta : 0;
      targetLaneIdx = Math.max(0, Math.min(lanes.length - 1, targetLaneIdx));
      const targetNoteNumber = lanes[targetLaneIdx].note;

      latestStep = newStep;
      latestPitch = targetNoteNumber;

      const isAligned = Math.abs(newStep - Math.round(newStep)) < 0.05;
      const bar = Math.floor(newStep / 16) + 1;
      const beat = ((newStep % 16) / 4) + 1;
      setActiveTimingInfo(
        `${lanes[targetLaneIdx]?.name ?? ''} • Bar ${bar}.${beat.toFixed(1)} ${
          isAligned ? '(Aligned)' : '(Off-beat)'
        }`
      );

      // Optimistic update
      setOptimisticNotes((prev) =>
        (prev ?? []).map((n) =>
          n.id === note.id
            ? { ...n, start_step: newStep, note: targetNoteNumber, off_beat: !isAligned }
            : n
        )
      );
    };

    const onMouseUp = () => {
      setIsDragging(false);
      setActiveTimingInfo(null);
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);

      // Persist to Rust engine
      api.looperUpdateNote(selectedTrack, note.id, latestStep, latestPitch).catch(console.error);
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  };

  const offBeatCount = useMemo(
    () => currentNotes.filter((n) => n.off_beat).length,
    [currentNotes]
  );

  return (
    <div className="bg-[#121418] border border-[#262a33] rounded-xl shadow-xl flex flex-col overflow-hidden select-none font-mono text-xs">
      {/* Pro Toolbar Header */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-[#181b22] border-b border-[#262a33] gap-2 flex-wrap">
        {/* Track Selection Switcher */}
        <div className="flex items-center gap-1.5">
          <button
            onClick={() => onSelectTrack(0)}
            className={`px-2.5 py-1 rounded text-xs font-bold transition-all flex items-center gap-1.5 ${
              selectedTrack === 0
                ? 'bg-[#e52521] text-white shadow'
                : 'bg-[#1f232b] text-slate-400 hover:text-white border border-[#2e3440]'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${selectedTrack === 0 ? 'bg-white' : 'bg-transparent'}`} />
            <span>T1: DRUMS</span>
          </button>
          <button
            onClick={() => onSelectTrack(1)}
            className={`px-2.5 py-1 rounded text-xs font-bold transition-all flex items-center gap-1.5 ${
              selectedTrack === 1
                ? 'bg-[#e52521] text-white shadow'
                : 'bg-[#1f232b] text-slate-400 hover:text-white border border-[#2e3440]'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${selectedTrack === 1 ? 'bg-white' : 'bg-transparent'}`} />
            <span>T2: KEYS</span>
          </button>
        </div>

        {/* Center: Live Timing Readout or Info */}
        <div className="flex items-center gap-2">
          {activeTimingInfo ? (
            <span className="px-2 py-0.5 rounded bg-amber-500/20 text-amber-300 font-bold border border-amber-500/40 animate-pulse">
              {activeTimingInfo}
            </span>
          ) : (
            <span
              className={`px-2 py-0.5 rounded font-bold border ${
                offBeatCount > 0
                  ? 'bg-amber-500/10 text-amber-400 border-amber-500/30'
                  : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
              }`}
            >
              {offBeatCount > 0 ? `${offBeatCount} OFF-BEAT` : '100% IN TIME'}
            </span>
          )}
        </div>

        {/* Right Editing Controls */}
        <div className="flex items-center gap-2">
          {/* Fold button (Piano roll only) */}
          {selectedTrack === 1 && (
            <button
              onClick={() => setIsFolded(!isFolded)}
              className={`px-2 py-1 rounded border font-bold text-xs transition-all ${
                isFolded
                  ? 'bg-[#1f232b] border-[#f5a623]/60 text-[#f5a623]'
                  : 'bg-[#1f232b] border-[#2e3440] text-slate-400 hover:text-white'
              }`}
              title="Fold piano roll to only show played notes"
            >
              FOLD: {isFolded ? 'ON' : 'ALL'}
            </button>
          )}

          {/* Snap Mode */}
          <div className="flex items-center bg-[#0f1115] rounded p-0.5 border border-[#222630]">
            <span className="text-[10px] text-slate-500 px-1 font-bold">SNAP</span>
            <button
              onClick={() => setSnapMode('1/16')}
              className={`px-1.5 py-0.5 rounded text-[11px] font-bold ${
                snapMode === '1/16' ? 'bg-[#e52521] text-white shadow' : 'text-slate-400 hover:text-white'
              }`}
            >
              1/16
            </button>
            <button
              onClick={() => setSnapMode('1/8')}
              className={`px-1.5 py-0.5 rounded text-[11px] font-bold ${
                snapMode === '1/8' ? 'bg-[#e52521] text-white shadow' : 'text-slate-400 hover:text-white'
              }`}
            >
              1/8
            </button>
            <button
              onClick={() => setSnapMode('off')}
              className={`px-1.5 py-0.5 rounded text-[11px] font-bold ${
                snapMode === 'off' ? 'bg-[#e52521] text-white shadow' : 'text-slate-400 hover:text-white'
              }`}
            >
              FREE
            </button>
          </div>

          {/* Quantize Button */}
          <button
            onClick={handleQuantize}
            title="Snap all off-beat notes to closest grid division"
            className="px-2.5 py-1 rounded bg-[#f5a623]/20 hover:bg-[#f5a623] hover:text-[#121418] text-[#f5a623] border border-[#f5a623]/50 font-bold text-xs transition-all active:scale-95 shadow"
          >
            QUANTIZE
          </button>

          {/* Clear Notes */}
          <button
            onClick={handleClearNotes}
            title="Clear all recorded notes on this track"
            className="px-2 py-1 rounded bg-[#1f232b] hover:bg-rose-950/60 text-slate-400 hover:text-rose-300 border border-[#2e3440] font-bold text-xs transition-all active:scale-95"
          >
            CLEAR
          </button>
        </div>
      </div>

      {/* Main Grid Viewport */}
      <div className="relative bg-[#0d0e12] flex flex-col" style={{ height: '220px' }}>
        {/* Time Ruler (Bars & Beats) */}
        <div className="flex border-b border-[#222630] bg-[#14171d] h-5 flex-shrink-0 z-20 font-bold text-[10px] text-slate-400">
          <div className="w-20 flex-shrink-0 border-r border-[#222630] px-2 flex items-center justify-between text-slate-500">
            <span>{selectedTrack === 0 ? 'PADS' : 'KEYS'}</span>
          </div>
          <div className="flex-1 flex divide-x divide-[#262c37]">
            {Array.from({ length: barCount }).map((_, barIdx) => (
              <div key={barIdx} className="flex-1 grid grid-cols-4 divide-x divide-[#1a1e26] text-center">
                <div className="flex items-center justify-center bg-[#181b22]/50 text-white font-extrabold">
                  {barIdx + 1}.1
                </div>
                <div className="flex items-center justify-center">{barIdx + 1}.2</div>
                <div className="flex items-center justify-center">{barIdx + 1}.3</div>
                <div className="flex items-center justify-center">{barIdx + 1}.4</div>
              </div>
            ))}
          </div>
        </div>

        {/* Scrollable Lanes Container */}
        <div className="flex-1 overflow-y-auto relative flex">
          {/* Left Lane Headers (Pad Names or Piano Keys) */}
          <div className="w-20 flex-shrink-0 flex flex-col border-r border-[#222630] bg-[#121419] z-20 sticky left-0 divide-y divide-[#1d212a]">
            {lanes.map((lane) => {
              const isPiano = selectedTrack === 1;
              const isBlack = 'isBlack' in lane ? lane.isBlack : false;
              const isOctaveC = isPiano && lane.name.startsWith('C') && !lane.name.includes('#');

              return (
                <div
                  key={lane.note}
                  className={`h-8 px-2 flex items-center justify-between text-xs font-bold ${
                    isBlack ? 'bg-[#0e1014] text-slate-400' : 'bg-[#181a20] text-slate-200'
                  }`}
                >
                  <span className={isOctaveC ? 'text-white underline font-extrabold' : ''}>
                    {lane.name}
                  </span>
                  <span className="text-[9px] text-slate-500 font-normal">
                    {'pad' in lane ? String((lane as { pad: string }).pad) : lane.note}
                  </span>
                </div>
              );
            })}
          </div>

          {/* Grid Area */}
          <div ref={gridAreaRef} className="flex-1 relative min-w-[500px]">
            {/* Background Waveform Canvas Overlay */}
            <canvas ref={canvasRef} className="absolute inset-0 pointer-events-none w-full h-full" />

            {/* Vertical Beat Grid Lines */}
            <div className="absolute inset-0 flex divide-x divide-[#262c37] pointer-events-none">
              {Array.from({ length: barCount }).map((_, barIdx) => (
                <div key={barIdx} className="flex-1 grid grid-cols-4 divide-x divide-[#1f242e]">
                  <div className="h-full grid grid-cols-4 divide-x divide-[#161920]">
                    <div />
                    <div />
                    <div />
                    <div />
                  </div>
                  <div className="h-full grid grid-cols-4 divide-x divide-[#161920]">
                    <div />
                    <div />
                    <div />
                    <div />
                  </div>
                  <div className="h-full grid grid-cols-4 divide-x divide-[#161920]">
                    <div />
                    <div />
                    <div />
                    <div />
                  </div>
                  <div className="h-full grid grid-cols-4 divide-x divide-[#161920]">
                    <div />
                    <div />
                    <div />
                    <div />
                  </div>
                </div>
              ))}
            </div>

            {/* Moving Playhead Cursor */}
            <div
              className="absolute top-0 bottom-0 w-px bg-[#e52521] shadow-[0_0_8px_#e52521] z-30 pointer-events-none transition-none"
              style={{ left: `${Math.max(0, Math.min(100, progress * 100))}%` }}
            >
              <div className="w-2 h-2 -ml-[3px] bg-[#e52521] rotate-45 transform origin-center" />
            </div>

            {/* Note Lanes */}
            <div className="flex flex-col divide-y divide-[#1a1e27] relative">
              {lanes.map((lane) => {
                const laneNotes = currentNotes.filter((n) => n.note === lane.note);
                const isBlack = 'isBlack' in lane ? lane.isBlack : false;

                return (
                  <div
                    key={lane.note}
                    className={`h-8 relative ${isBlack ? 'bg-[#0f1115]/60' : 'bg-transparent'}`}
                  >
                    {laneNotes.map((note) => {
                      const leftPct = (note.start_step / totalSteps) * 100;
                      const widthPct = Math.max(1.8, (note.length_steps / totalSteps) * 100);

                      const isOff = note.off_beat;
                      const bgClass = isOff
                        ? 'bg-[#f5a623] text-[#121418] border border-[#ffcf70] shadow-[0_0_8px_rgba(245,166,35,0.7)] animate-pulse'
                        : 'bg-[#e52521] text-white border border-[#ff6663] shadow-[0_0_6px_rgba(229,37,33,0.5)]';

                      return (
                        <div
                          key={note.id}
                          onMouseDown={(e) => handleMouseDownNote(note, e)}
                          onContextMenu={(e) => handleDeleteNote(note.id, e)}
                          title={`${lane.name} • Step ${note.start_step.toFixed(2)} ${
                            isOff ? '(Off-beat: drag to align)' : '(Aligned)'
                          }. Right-click to delete.`}
                          className={`note-block absolute top-1 bottom-1 rounded px-1.5 flex items-center justify-between text-[10px] font-bold font-mono ${bgClass} z-10 cursor-grab active:cursor-grabbing hover:brightness-110`}
                          style={{
                            left: `${leftPct}%`,
                            width: `${widthPct}%`,
                            minWidth: '24px',
                          }}
                        >
                          <span className="truncate pointer-events-none">{lane.name}</span>
                          {isOff && <span className="w-1.5 h-1.5 rounded-full bg-[#121418] ml-1 flex-shrink-0" />}
                        </div>
                      );
                    })}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>

      {/* Footer Info & Note Count */}
      <div className="flex items-center justify-between px-3 py-1 bg-[#14161c] border-t border-[#222630] text-[10px] text-slate-500">
        <span>Drag note horizontally to align timing • Drag vertically to transpose pitch • Right-click to delete</span>
        <span className="text-slate-400 font-bold">{currentNotes.length} NOTES</span>
      </div>
    </div>
  );
};
