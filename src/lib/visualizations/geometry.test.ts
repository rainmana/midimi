import { describe, it, expect } from 'vitest';
import { pitchToY, windowSlice, leapIntensity } from './geometry';
import type { Note } from '../types';

const n = (note: number, start_sec: number): Note => ({ track: 0, channel: 0, note, start_sec, dur_sec: 0.5, velocity: 100 });

describe('pitchToY', () => {
  it('maps higher pitch nearer the top and clamps', () => {
    expect(pitchToY(108, 0, 100)).toBeCloseTo(0);
    expect(pitchToY(21, 0, 100)).toBeCloseTo(100);
    expect(pitchToY(200, 0, 100)).toBeCloseTo(0);
    expect(pitchToY(0, 0, 100)).toBeCloseTo(100);
    expect(pitchToY(72, 0, 100)).toBeLessThan(pitchToY(60, 0, 100));
  });
});

describe('windowSlice', () => {
  it('returns only notes within the inclusive window', () => {
    const notes = [n(60, 0), n(62, 2), n(64, 5), n(65, 9)];
    expect(windowSlice(notes, 1, 6).map((x) => x.note)).toEqual([62, 64]);
  });
});

describe('leapIntensity', () => {
  it('is low for a step and ~1 for an octave', () => {
    expect(leapIntensity(60, 62)).toBeLessThan(0.3);
    expect(leapIntensity(60, 72)).toBeCloseTo(1);
    expect(leapIntensity(60, 90)).toBeCloseTo(1);
  });
});
