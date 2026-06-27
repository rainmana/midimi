import type { Visualization, NoteEvent } from './types';

interface Orb { x: number; y: number; vx: number; vy: number; r: number; life: number; maxLife: number; hue: number; }

const TRACK_HUES = [190, 280, 320, 50, 150, 220, 0, 100];

export function createCosmicAurora(): Visualization {
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let w = 0, h = 0, dpr = 1;
  let orbs: Orb[] = [];
  let t = 0;
  let smoothLevel = 0;
  let bandsSmooth: number[] = [];

  const pitchToX = (note: number) => {
    const lo = 21, hi = 108;
    const f = Math.max(0, Math.min(1, (note - lo) / (hi - lo)));
    return f * w;
  };

  return {
    id: 'cosmic-aurora',
    name: 'Cosmic Aurora',
    setup(c) { canvas = c; ctx = c.getContext('2d')!; },
    resize(width, height) {
      dpr = window.devicePixelRatio || 1;
      w = width; h = height;
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    },
    onNoteOn(n: NoteEvent) {
      const hue = TRACK_HUES[n.track % TRACK_HUES.length];
      const vel = n.velocity / 127;
      const f = Math.max(0, Math.min(1, (n.note - 21) / (108 - 21)));
      orbs.push({
        x: pitchToX(n.note),
        y: h * (0.25 + 0.5 * (1 - f)) + (Math.random() - 0.5) * 40,
        vx: (Math.random() - 0.5) * 12,
        vy: -8 - vel * 22,
        r: 6 + vel * 26,
        life: 0,
        maxLife: 1.1 + vel * 1.4,
        hue,
      });
      if (orbs.length > 600) orbs.splice(0, orbs.length - 600);
    },
    onNoteOff() {},
    onFrame(_playhead, level, bands) {
      t += 1 / 60;
      smoothLevel += (level - smoothLevel) * 0.2;
      if (bandsSmooth.length !== bands.length) bandsSmooth = bands.slice();
      for (let i = 0; i < bands.length; i++) bandsSmooth[i] += (bands[i] - bandsSmooth[i]) * 0.25;

      // Trail (slight persistence => motion blur).
      ctx.globalCompositeOperation = 'source-over';
      ctx.fillStyle = 'rgba(5, 4, 12, 0.28)';
      ctx.fillRect(0, 0, w, h);

      // Aurora ribbons (additive).
      ctx.globalCompositeOperation = 'lighter';
      for (let r = 0; r < 3; r++) {
        const baseHue = 170 + r * 50 + Math.sin(t * 0.1 + r) * 20;
        const amp = h * (0.06 + 0.10 * (bandsSmooth[r * 3] ?? smoothLevel));
        const yBase = h * (0.30 + r * 0.14);
        ctx.beginPath();
        for (let x = 0; x <= w; x += 12) {
          const y = yBase
            + Math.sin(x * 0.006 + t * (0.4 + r * 0.2)) * amp
            + Math.sin(x * 0.013 - t * 0.7) * amp * 0.5;
          x === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
        }
        const grad = ctx.createLinearGradient(0, yBase - amp, 0, yBase + amp);
        grad.addColorStop(0, `hsla(${baseHue}, 90%, 65%, 0)`);
        grad.addColorStop(0.5, `hsla(${baseHue}, 90%, 65%, ${0.10 + 0.25 * smoothLevel})`);
        grad.addColorStop(1, `hsla(${baseHue}, 90%, 65%, 0)`);
        ctx.strokeStyle = grad;
        ctx.lineWidth = 26 + 60 * smoothLevel;
        ctx.stroke();
      }

      // Note orbs (additive glow + bright core).
      for (let i = orbs.length - 1; i >= 0; i--) {
        const o = orbs[i];
        o.life += 1 / 60;
        if (o.life > o.maxLife) { orbs.splice(i, 1); continue; }
        o.x += o.vx / 60;
        o.y += o.vy / 60;
        o.vy += 6 / 60;
        const k = 1 - o.life / o.maxLife;
        const rr = o.r * (0.6 + 0.4 * k) * (1 + 0.6 * smoothLevel);
        const g = ctx.createRadialGradient(o.x, o.y, 0, o.x, o.y, rr * 3);
        g.addColorStop(0, `hsla(${o.hue}, 100%, 75%, ${0.9 * k})`);
        g.addColorStop(0.3, `hsla(${o.hue}, 100%, 60%, ${0.35 * k})`);
        g.addColorStop(1, `hsla(${o.hue}, 100%, 50%, 0)`);
        ctx.fillStyle = g;
        ctx.beginPath();
        ctx.arc(o.x, o.y, rr * 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = `hsla(${o.hue}, 100%, 92%, ${k})`;
        ctx.beginPath();
        ctx.arc(o.x, o.y, rr * 0.5, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalCompositeOperation = 'source-over';
    },
    teardown() { orbs = []; },
  };
}
