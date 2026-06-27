import type { Visualization } from './types';
import { createCosmicAurora } from './cosmic-aurora';

export interface VizEntry { id: string; name: string; create: () => Visualization; }

export const VIZZES: VizEntry[] = [
  { id: 'cosmic-aurora', name: 'Cosmic Aurora', create: createCosmicAurora },
];

export function createViz(id: string): Visualization {
  return (VIZZES.find((v) => v.id === id) ?? VIZZES[0]).create();
}
