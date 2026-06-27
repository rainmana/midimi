import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { MidiData, Playhead, SoundfontRow, LibraryRow, Setting } from './types';

export const openMidi = (path: string) => invoke<MidiData>('open_midi', { path });
export const play = () => invoke('play');
export const pause = () => invoke('pause');
export const seek = (seconds: number) => invoke('seek', { seconds });
export const setTempo = (ratio: number) => invoke('set_tempo', { ratio });
export const setVolume = (volume: number) => invoke('set_volume', { volume });
export const loadSoundfont = (path: string) => invoke<SoundfontRow>('load_soundfont', { path });
export const setSoundfont = (id: number) => invoke('set_soundfont', { id });
export const listSoundfonts = () => invoke<SoundfontRow[]>('list_soundfonts');
export const listRecent = () => invoke<LibraryRow[]>('list_recent');
export const getSettings = () => invoke<Setting[]>('get_settings');
export const setSetting = (key: string, value: string) => invoke('set_setting', { key, value });
export const demoPath = () => invoke<string>('demo_path');

export const listenPlayhead = (cb: (p: Playhead) => void): Promise<UnlistenFn> =>
  listen<Playhead>('playhead', (e) => cb(e.payload));

export async function pickMidi(): Promise<string | null> {
  const r = await open({ multiple: false, filters: [{ name: 'MIDI', extensions: ['mid', 'midi'] }] });
  return typeof r === 'string' ? r : null;
}
export async function pickSoundfont(): Promise<string | null> {
  const r = await open({ multiple: false, filters: [{ name: 'SoundFont', extensions: ['sf2'] }] });
  return typeof r === 'string' ? r : null;
}
