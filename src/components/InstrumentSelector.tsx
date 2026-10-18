import React, { useState, useEffect } from 'react';
import { Music, ChevronLeft, ChevronRight } from 'lucide-react';
import { api, type CuratedPreset, type SoundFontCatalog } from '../types/ipc';

interface InstrumentSelectorProps {
  onPresetChanged?: (preset: CuratedPreset) => void;
}

const CATEGORIES = ['All', 'Keys', 'Synth', 'Strings', 'Bass', 'Drums'];

export const InstrumentSelector: React.FC<InstrumentSelectorProps> = ({
  onPresetChanged,
}) => {
  const [catalog, setCatalog] = useState<SoundFontCatalog | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<string>('All');
  const [activeMelodicId, setActiveMelodicId] = useState<string>('grand-piano');

  const fetchCatalog = () => {
    api.getSoundfonts()
      .then((cat) => {
        setCatalog(cat);
        if (cat.active_melodic_preset_id) {
          setActiveMelodicId(cat.active_melodic_preset_id);
        }
      })
      .catch(console.error);
  };

  useEffect(() => {
    fetchCatalog();
  }, []);

  const presets = catalog?.curated_presets ?? [];

  const filteredPresets = presets.filter((p) => {
    if (selectedCategory === 'All') return !p.is_drum;
    if (selectedCategory === 'Drums') return p.is_drum;
    return p.category.toLowerCase() === selectedCategory.toLowerCase();
  });

  const currentPresetIndex = filteredPresets.findIndex((p) => p.id === activeMelodicId);
  const currentPreset = currentPresetIndex >= 0
    ? filteredPresets[currentPresetIndex]
    : filteredPresets[0];

  const handleSelectPreset = (presetId: string) => {
    setActiveMelodicId(presetId);
    api.selectCuratedPreset(presetId)
      .then((preset) => {
        if (onPresetChanged) onPresetChanged(preset);
      })
      .catch(console.error);
  };

  const handlePrev = () => {
    if (filteredPresets.length === 0) return;
    const newIdx = (currentPresetIndex - 1 + filteredPresets.length) % filteredPresets.length;
    handleSelectPreset(filteredPresets[newIdx].id);
  };

  const handleNext = () => {
    if (filteredPresets.length === 0) return;
    const newIdx = (currentPresetIndex + 1) % filteredPresets.length;
    handleSelectPreset(filteredPresets[newIdx].id);
  };

  return (
    <div className="flex flex-col gap-3">
      {/* Category Pills */}
      <div className="flex items-center pb-2 border-b border-chassis-750 gap-1.5 overflow-x-auto">
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            onClick={() => setSelectedCategory(cat)}
            className={`px-2.5 py-0.5 rounded text-[11px] font-mono font-semibold transition-all ${
              selectedCategory === cat
                ? 'bg-akai-red text-white shadow-sm'
                : 'bg-chassis-900 text-slate-400 hover:text-white border border-chassis-750'
            }`}
          >
            {cat}
          </button>
        ))}
      </div>

      {/* Preset Card with Left/Right Quick Navigator */}
      <div className="bg-chassis-900 border border-chassis-700 rounded-lg p-3 flex items-center justify-between gap-3 shadow-inner">
        <button
          onClick={handlePrev}
          title="Previous SoundFont Preset"
          className="p-2 rounded bg-chassis-800 hover:bg-chassis-750 text-slate-300 hover:text-white border border-chassis-700 transition-all active:scale-95"
        >
          <ChevronLeft className="w-5 h-5 text-slate-300" />
        </button>

        <div className="flex-1 text-center font-mono overflow-hidden">
          <div className="text-[10px] text-slate-500 uppercase tracking-widest font-bold flex items-center justify-center gap-1.5 mb-1">
            <Music className="w-3 h-3 text-akai-red" />
            <span>SOUNDFONT PRESET</span>
          </div>

          {/* Preset Title Dropdown */}
          <select
            value={currentPreset?.id ?? ''}
            onChange={(e) => handleSelectPreset(e.target.value)}
            className="bg-transparent text-sm font-bold text-[#2cf583] tracking-wide cursor-pointer focus:outline-none text-center max-w-full truncate"
          >
            {filteredPresets.map((p) => (
              <option key={p.id} value={p.id} className="bg-chassis-900 text-slate-200">
                {p.name}
              </option>
            ))}
          </select>
        </div>

        <button
          onClick={handleNext}
          title="Next SoundFont Preset"
          className="p-2 rounded bg-chassis-800 hover:bg-chassis-750 text-slate-300 hover:text-white border border-chassis-700 transition-all active:scale-95"
        >
          <ChevronRight className="w-5 h-5 text-slate-300" />
        </button>
      </div>
    </div>
  );
};
