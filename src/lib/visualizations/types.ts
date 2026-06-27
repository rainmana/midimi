import type { Note } from '../types';

export interface NoteEvent { track: number; channel: number; note: number; velocity: number; }

// The plugin seam: future user visualizations implement this same interface.
export interface Visualization {
  id: string;
  name: string;
  setup(canvas: HTMLCanvasElement): void;
  onNoteOn(note: NoteEvent): void;
  onNoteOff(note: NoteEvent): void;
  onFrame(playhead: number, level: number, bands: number[]): void;
  resize(width: number, height: number): void;
  teardown(): void;
  /** Optional: full note timeline on song load (for structure-revealing vises). */
  loadTimeline?(notes: Note[], durationSec: number): void;
}
