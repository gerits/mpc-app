import React, { useState, useEffect } from 'react';
import { FolderOpen, Upload, Music, RefreshCw, HardDrive } from 'lucide-react';
import { api, type SoundFontMetadata } from '../types/ipc';

export const SoundFontManager: React.FC = () => {
  const [soundfonts, setSoundfonts] = useState<SoundFontMetadata[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [importPath, setImportPath] = useState('');

  const loadSoundfonts = () => {
    setIsLoading(true);
    api.getSoundfonts()
      .then((cat) => {
        setSoundfonts(cat.soundfonts);
      })
      .catch((err) => setMessage(`Error loading soundfonts: ${err}`))
      .finally(() => setIsLoading(false));
  };

  useEffect(() => {
    loadSoundfonts();
  }, []);

  const handleOpenFolder = () => {
    api.openSoundfontsFolder()
      .then((path) => {
        setMessage(`Opened SoundFonts directory in file manager: ${path}`);
        setTimeout(() => setMessage(null), 5000);
      })
      .catch((err) => setMessage(`Failed to open folder: ${err}`));
  };

  const handleImportByPath = (path: string) => {
    if (!path.trim()) return;
    setIsLoading(true);
    api.importUserSoundfont(path.trim())
      .then((meta) => {
        setMessage(`Successfully imported ${meta.name} (${meta.preset_count} presets)`);
        setImportPath('');
        loadSoundfonts();
        setTimeout(() => setMessage(null), 5000);
      })
      .catch((err) => {
        setMessage(`Import failed: ${err}`);
        setTimeout(() => setMessage(null), 6000);
      })
      .finally(() => setIsLoading(false));
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  };

  return (
    <div className="flex flex-col gap-4 font-mono text-xs select-none">
      {/* Top Action Bar */}
      <div className="flex items-center justify-between gap-3 pb-3 border-b border-chassis-700">
        <div>
          <h4 className="font-bold text-slate-200 uppercase tracking-wider text-sm flex items-center gap-2">
            <Music className="w-4 h-4 text-akai-red" />
            SoundFont Library (.SF2)
          </h4>
          <p className="text-[11px] text-slate-400 font-sans mt-0.5">
            Manage embedded performance soundfonts and imported instrument banks.
          </p>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={loadSoundfonts}
            disabled={isLoading}
            title="Rescan soundfonts folders"
            className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-chassis-750 hover:bg-chassis-700 text-slate-300 border border-chassis-600 active:scale-95 transition-all"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
            <span>Rescan</span>
          </button>

          <button
            onClick={handleOpenFolder}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-chassis-750 hover:bg-chassis-700 text-slate-200 border border-chassis-600 active:scale-95 transition-all"
          >
            <FolderOpen className="w-3.5 h-3.5 text-amber-400" />
            <span>Open Folder</span>
          </button>
        </div>
      </div>

      {/* Import Form */}
      <div className="bg-chassis-900 border border-chassis-750 rounded-lg p-3 flex flex-col gap-2">
        <span className="font-bold text-slate-300 text-[11px] flex items-center gap-1.5">
          <Upload className="w-3.5 h-3.5 text-akai-red" />
          Import External SoundFont
        </span>
        <div className="flex gap-2">
          <input
            type="text"
            placeholder="Absolute path to .sf2 file (e.g. /path/to/my_instrument.sf2)"
            value={importPath}
            onChange={(e) => setImportPath(e.target.value)}
            className="flex-1 bg-chassis-800 border border-chassis-700 rounded px-3 py-1.5 text-slate-200 text-xs focus:outline-none focus:border-akai-red font-mono"
          />
          <button
            onClick={() => handleImportByPath(importPath)}
            disabled={!importPath.trim() || isLoading}
            className="px-4 py-1.5 rounded bg-akai-red hover:bg-akai-redBright text-white font-bold text-xs disabled:opacity-50 transition-all active:scale-95 shrink-0"
          >
            Import
          </button>
        </div>
      </div>

      {/* Status Notification Message */}
      {message && (
        <div className="p-2.5 rounded bg-chassis-900 border border-emerald-500/40 text-emerald-400 text-[11px] flex items-center justify-between">
          <span className="truncate">{message}</span>
          <button onClick={() => setMessage(null)} className="text-slate-400 hover:text-white ml-2">
            &times;
          </button>
        </div>
      )}

      {/* Installed SoundFonts List */}
      <div className="flex flex-col gap-2 max-h-64 overflow-y-auto pr-1">
        {soundfonts.map((sf) => (
          <div
            key={sf.id}
            className="bg-chassis-850 border border-chassis-700 rounded-lg p-2.5 flex items-center justify-between gap-3 hover:border-chassis-600 transition-colors"
          >
            <div className="flex items-center gap-2.5 overflow-hidden">
              <HardDrive className="w-4 h-4 text-slate-400 shrink-0" />
              <div className="truncate">
                <div className="font-bold text-slate-200 truncate flex items-center gap-2">
                  <span>{sf.name}</span>
                  {sf.is_bundled ? (
                    <span className="px-1.5 py-0.2 rounded text-[9px] bg-emerald-950 text-emerald-400 border border-emerald-800">
                      Bundled
                    </span>
                  ) : (
                    <span className="px-1.5 py-0.2 rounded text-[9px] bg-amber-950 text-amber-400 border border-amber-800">
                      User
                    </span>
                  )}
                </div>
                <div className="text-[10px] text-slate-500 font-mono truncate">
                  {sf.filename} &bull; {formatBytes(sf.size_bytes)} &bull; {sf.preset_count} Presets
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2 shrink-0">
              <span className="text-[10px] text-slate-400 bg-chassis-900 px-2 py-1 rounded border border-chassis-750">
                Ready
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
