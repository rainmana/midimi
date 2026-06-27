import type { Visualization } from './types';
import { createCosmicAurora } from './cosmic-aurora';
import { createMelodyPath } from './melody-path';

export interface VizEntry { id: string; name: string; create: () => Visualization; }

export const VIZZES: VizEntry[] = [
  { id: 'cosmic-aurora', name: 'Cosmic Aurora', create: createCosmicAurora },
  { id: 'melody-path', name: "Melody's Path", create: createMelodyPath },
];

export function createViz(id: string): Visualization {
  return (VIZZES.find((v) => v.id === id) ?? VIZZES[0]).create();
}
