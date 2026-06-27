export interface Theme { id: string; name: string; vars: Record<string, string>; }

export const THEMES: Theme[] = [
  { id: 'cosmic', name: 'Cosmic', vars: { '--bg':'#05040c','--surface':'#0b0922cc','--border':'#2a2350','--text':'#dfe4ff','--muted':'#8b86b8','--accent':'#19f0c8','--accent2':'#d36bff' } },
  { id: 'nebula', name: 'Nebula Rose', vars: { '--bg':'#0a0510','--surface':'#1a0f24cc','--border':'#43275c','--text':'#ffe9f6','--muted':'#b58fb0','--accent':'#ff6bd0','--accent2':'#8a6bff' } },
  { id: 'abyss', name: 'Abyss', vars: { '--bg':'#02060a','--surface':'#06131ccc','--border':'#143041','--text':'#d6f7ff','--muted':'#6f97a6','--accent':'#2bd6ff','--accent2':'#19f0c8' } },
];

export function applyTheme(id: string): string {
  const t = THEMES.find((x) => x.id === id) ?? THEMES[0];
  for (const [k, v] of Object.entries(t.vars)) document.documentElement.style.setProperty(k, v);
  return t.id;
}
