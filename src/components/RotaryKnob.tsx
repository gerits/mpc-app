import React, { useState, useRef } from 'react';

interface RotaryKnobProps {
  knobIndex: number; // 0..=7
  label: string;
  value: number; // 0.0 ..= 1.0
  formatValue?: (val: number) => string;
  onChange: (knobIndex: number, val: number) => void;
  defaultValue?: number;
}

export const RotaryKnob: React.FC<RotaryKnobProps> = ({
  knobIndex,
  label,
  value,
  formatValue,
  onChange,
  defaultValue = 0.5,
}) => {
  const [isDragging, setIsDragging] = useState(false);
  const dragStartY = useRef<number>(0);
  const startValue = useRef<number>(value);

  // Rotation: -135deg (min) to +135deg (max)
  const angle = -135 + value * 270;

  const handlePointerDown = (e: React.PointerEvent) => {
    e.preventDefault();
    setIsDragging(true);
    dragStartY.current = e.clientY;
    startValue.current = value;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!isDragging) return;
    const deltaY = dragStartY.current - e.clientY;
    // 150px drag for full sweep from 0 to 1
    const deltaVal = deltaY / 150;
    const newVal = Math.max(0, Math.min(1, startValue.current + deltaVal));
    onChange(knobIndex, newVal);
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    if (isDragging) {
      setIsDragging(false);
      try {
        (e.target as HTMLElement).releasePointerCapture(e.pointerId);
      } catch {
        // ignore
      }
    }
  };

  const handleDoubleClick = () => {
    onChange(knobIndex, defaultValue);
  };

  // SVG Arc Calculation for indicator ring
  const radius = 22;
  const circumference = 2 * Math.PI * radius;
  // 270 degrees sweep = 0.75 of circumference
  const arcLength = circumference * 0.75;
  const strokeDashoffset = arcLength * (1 - value);

  const displayVal = formatValue ? formatValue(value) : `${Math.round(value * 100)}%`;

  return (
    <div
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onDoubleClick={handleDoubleClick}
      className="flex flex-col items-center select-none cursor-ns-resize group"
    >
      {/* Knob Dial with Outer LED Arc */}
      <div className="relative w-14 h-14 flex items-center justify-center">
        {/* SVG Arc Progress Ring */}
        <svg className="absolute inset-0 w-full h-full -rotate-[135deg] pointer-events-none" viewBox="0 0 56 56">
          {/* Background Track */}
          <circle
            cx="28"
            cy="28"
            r={radius}
            fill="none"
            stroke="#1c2027"
            strokeWidth="3.5"
            strokeDasharray={`${arcLength} ${circumference}`}
            strokeLinecap="round"
          />
          {/* Active Colored Value Arc */}
          <circle
            cx="28"
            cy="28"
            r={radius}
            fill="none"
            stroke={isDragging ? '#ff3a36' : '#e52521'}
            strokeWidth="3.5"
            strokeDasharray={`${arcLength} ${circumference}`}
            strokeDashoffset={strokeDashoffset}
            strokeLinecap="round"
            className="transition-all duration-75"
          />
        </svg>

        {/* Circular Aluminum Knob Face */}
        <div
          className={`w-10 h-10 rounded-full bg-gradient-to-b from-[#2a2f3a] to-[#14171d] border-2 border-[#3c4453] shadow-[0_4px_8px_rgba(0,0,0,0.6),inset_0_1px_1px_rgba(255,255,255,0.15)] flex items-center justify-center transition-transform ${
            isDragging ? 'scale-105 border-akai-red' : 'group-hover:border-slate-400'
          }`}
          style={{
            transform: `rotate(${angle}deg)`,
          }}
        >
          {/* Red Indicator Pointer Notch */}
          <div className="w-1 h-3.5 bg-akai-red rounded-full absolute -top-0.5 shadow-[0_0_4px_rgba(229,37,33,0.8)]" />
          {/* Center brushed cap cap-dot */}
          <div className="w-4 h-4 rounded-full bg-[#181b22] border border-[#2b313c] shadow-inner" />
        </div>
      </div>

      {/* Parameter Label */}
      <div className="text-[10px] font-mono font-bold tracking-wider text-slate-300 uppercase mt-1 text-center truncate max-w-[70px]">
        {label}
      </div>

      {/* Formatted Value Readout */}
      <div
        className={`text-[9px] font-mono font-semibold transition-colors ${
          isDragging ? 'text-akai-red font-bold' : 'text-slate-400 group-hover:text-slate-200'
        }`}
      >
        {displayVal}
      </div>
    </div>
  );
};
