export interface Note { track: number; channel: number; note: number; start_sec: number; dur_sec: number; velocity: number; }
export interface TrackInfo { index: number; name: string | null; }
export interface MidiData { title: string | null; duration_sec: number; tracks: TrackInfo[]; notes: Note[]; }
export interface Playhead { time: number; duration: number; level: number; bands: number[]; playing: boolean; }
export interface SoundfontRow { id: number; path: string; name: string; is_builtin: boolean; }
export interface LibraryRow { id: number; path: string; title: string | null; duration_sec: number; }
export interface Setting { key: string; value: string; }
