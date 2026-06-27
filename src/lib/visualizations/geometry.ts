import type { Note } from '../types';

export function pitchToY(note: number, top: number, bottom: number, minPitch = 21, maxPitch = 108): number {
  const f = Math.max(0, Math.min(1, (note - minPitch) / (maxPitch - minPitch)));
  return bottom - f * (bottom - top); // higher pitch -> nearer top
}

/** Notes whose start_sec is within [t0, t1] (inclusive). `sorted` must be ascending by start_sec. */
export function windowSlice(sorted: Note[], t0: number, t1: number): Note[] {
  return sorted.filter((nn) => nn.start_sec >= t0 && nn.start_sec <= t1);
}

/** 0..1 emphasis for an interval; ~0 for a step, ~1 for an octave or larger. */
export function leapIntensity(prevNote: number, nextNote: number): number {
  return Math.max(0, Math.min(1, Math.abs(nextNote - prevNote) / 12));
}
