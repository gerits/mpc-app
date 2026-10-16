import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface MidiDeviceInfo {
  index: number;
  name: string;
  is_akai: boolean;
}

export interface SoundFontMetadata {
  id: string;
  name: string;
  filename: string;
  path: string;
  size_bytes: number;
  is_bundled: boolean;
  preset_count: number;
}

export interface CuratedPreset {
  id: string;
  name: string;
  category: string;
  soundfont_filename: string;
  is_drum: boolean;
  bank: number;
  patch: number;
}

export interface SoundFontCatalog {
  soundfonts: SoundFontMetadata[];
  curated_presets: CuratedPreset[];
  active_melodic_preset_id: string | null;
  active_drum_preset_id: string | null;
}

export interface ChannelVolumes {
  master: number;
  keys: number;
  drums: number;
}

export interface SystemStatus {
  audio_running: boolean;
  sample_rate: number;
  buffer_size: number;
  audio_device: string | null;
  midi_device: string | null;
  active_melodic_preset: string | null;
  active_drum_preset: string | null;
}

export type TrackState = 'empty' | 'recording' | 'playing' | 'overdubbing' | 'muted';

export interface LooperSnapshot {
  bpm: number;
  bar_count: number;
  current_bar: number;
  current_beat: number;
  progress: number;
  is_running: boolean;
  track1_state: TrackState;
  track2_state: TrackState;
  track1_has_audio: boolean;
  track2_has_audio: boolean;
}

export interface AudioMeterPayload {
  peak_left: number;
  peak_right: number;
  rms_left: number;
  rms_right: number;
}

export type RoutedEvent =
  | { type: 'KeyboardNoteOn'; data: { channel: number; note: number; velocity: number } }
  | { type: 'KeyboardNoteOff'; data: { channel: number; note: number } }
  | { type: 'DrumPadOn'; data: { pad_index: number; note: number; velocity: number } }
  | { type: 'DrumPadOff'; data: { pad_index: number; note: number } }
  | { type: 'MacroKnob'; data: { knob_index: number; value: number } }
  | { type: 'PitchBend'; data: { value: number } }
  | { type: 'Modulation'; data: { value: number } }
  | { type: 'SustainPedal'; data: { is_down: boolean } }
  | { type: 'Raw'; data: unknown };

// -------------------------------------------------------------------------------------------------
// Typed Tauri IPC Client API
// -------------------------------------------------------------------------------------------------

export const api = {
  getSystemStatus: () => invoke<SystemStatus>('get_system_status'),
  getAudioDevices: () => invoke<string[]>('get_audio_devices'),
  setBufferSize: (bufferSize: number) => invoke<void>('set_buffer_size', { bufferSize }),
  openSoundfontsFolder: () => invoke<string>('open_soundfonts_folder'),
  getMidiDevices: () => invoke<MidiDeviceInfo[]>('get_midi_devices'),
  connectMidiDevice: (deviceName: string) =>
    invoke<string>('connect_midi_device', { deviceName }),
  disconnectMidiDevice: () => invoke<void>('disconnect_midi_device'),

  getSoundfonts: () => invoke<SoundFontCatalog>('get_soundfonts'),
  selectCuratedPreset: (presetId: string) =>
    invoke<CuratedPreset>('select_curated_preset', { presetId }),
  selectSoundfontProgram: (
    filename: string,
    isDrum: boolean,
    bank: number,
    patch: number
  ) =>
    invoke<void>('select_soundfont_program', {
      filename,
      isDrum,
      bank,
      patch,
    }),
  importUserSoundfont: (sourcePath: string) =>
    invoke<SoundFontMetadata>('import_user_soundfont', { sourcePath }),

  setMacroParameter: (index: number, value: number) =>
    invoke<void>('set_macro_parameter', { index, value }),
  getMacroParameters: () => invoke<[number, number, number, number, number, number, number, number]>('get_macro_parameters'),

  setVolume: (track: 'master' | 'keys' | 'drums', volume: number) =>
    invoke<void>('set_volume', { track, volume }),
  getVolumes: () => invoke<ChannelVolumes>('get_volumes'),

  looperRecord: (track: number) => invoke<void>('looper_record', { track }),
  looperOverdub: (track: number) => invoke<void>('looper_overdub', { track }),
  looperPlay: () => invoke<void>('looper_play'),
  looperStop: () => invoke<void>('looper_stop'),
  looperClear: (track: number) => invoke<void>('looper_clear', { track }),
  looperClearAll: () => invoke<void>('looper_clear_all'),
  looperSetBpm: (bpm: number) => invoke<void>('looper_set_bpm', { bpm }),
  looperSetBars: (bars: number) => invoke<void>('looper_set_bars', { bars }),
  looperToggleMute: (track: number) => invoke<void>('looper_toggle_mute', { track }),
  looperExport: (customPath?: string) =>
    invoke<string>('looper_export', { customPath: customPath ?? null }),
  getLooperSnapshot: () => invoke<LooperSnapshot>('get_looper_snapshot'),
  panicAudio: () => invoke<void>('panic_audio'),

  sendNoteOn: (channel: number, note: number, velocity = 100) =>
    invoke<void>('send_note_on', { channel, note, velocity }),
  sendNoteOff: (channel: number, note: number) =>
    invoke<void>('send_note_off', { channel, note }),
  sendPadOn: (padIndex: number, note: number, velocity = 110) =>
    invoke<void>('send_pad_on', { padIndex, note, velocity }),
  sendPadOff: (padIndex: number, note: number) =>
    invoke<void>('send_pad_off', { padIndex, note }),
  sendPitchBend: (value: number) => invoke<void>('send_pitch_bend', { value }),
  sendModulation: (value: number) => invoke<void>('send_modulation', { value }),
};

// -------------------------------------------------------------------------------------------------
// Realtime Event Subscriptions
// -------------------------------------------------------------------------------------------------

export function onMidiEvent(callback: (event: RoutedEvent) => void): Promise<UnlistenFn> {
  return listen<RoutedEvent>('midi-event', (e) => callback(e.payload));
}

export function onAudioMeter(callback: (meter: AudioMeterPayload) => void): Promise<UnlistenFn> {
  return listen<AudioMeterPayload>('audio-meter', (e) => callback(e.payload));
}

export function onLooperSnapshot(callback: (snap: LooperSnapshot) => void): Promise<UnlistenFn> {
  return listen<LooperSnapshot>('looper-snapshot', (e) => callback(e.payload));
}
