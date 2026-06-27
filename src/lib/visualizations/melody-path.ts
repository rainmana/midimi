import type { Visualization, NoteEvent } from './types';
import type { Note } from '../types';
import { pitchToY, windowSlice, leapIntensity } from './geometry';
import { TRACK_HUES } from './cosmic-aurora';

const PAST = 3, FUTURE = 7, PLAYHEAD_FRAC = 0.30;

interface Bloom { x: number; y: number; hue: number; life: number; r: number; }

export function createMelodyPath(): Visualization {
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let w = 0, h = 0, dpr = 1;
  let byTrack: Note[][] = [];
  let blooms: Bloom[] = [];
  let smoothLevel = 0;

  const noteY = (note: number) => pitchToY(note, h * 0.12, h * 0.88);

  return {
    id: 'melody-path',
    name: "Melody's Path",
    setup(c) { canvas = c; ctx = c.getContext('2d')!; },
    resize(width, height) {
      dpr = window.devicePixelRatio || 1; w = width; h = height;
      canvas.width = Math.floor(width * dpr); canvas.height = Math.floor(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    },
    loadTimeline(notes: Note[]) {
      byTrack = [];
      for (const nn of notes) (byTrack[nn.track] ??= []).push(nn);
      for (const t of byTrack) if (t) t.sort((a, b) => a.start_sec - b.start_sec);
      blooms = [];
    },
    onNoteOn(n: NoteEvent) {
      blooms.push({
        x: PLAYHEAD_FRAC * w,
        y: noteY(n.note),
        hue: TRACK_HUES[n.track % TRACK_HUES.length],
        life: 0,
        r: 10 + (n.velocity / 127) * 26,
      });
    },
    onNoteOff() {},
    onFrame(playhead, level, _bands) {
      smoothLevel += (level - smoothLevel) * 0.2;
      const playheadX = PLAYHEAD_FRAC * w;
      const pxPerSec = w / (PAST + FUTURE);
      const t0 = playhead - PAST, t1 = playhead + FUTURE;

      ctx.globalCompositeOperation = 'source-over';
      ctx.fillStyle = 'rgba(5, 4, 12, 0.30)';
      ctx.fillRect(0, 0, w, h);

      ctx.strokeStyle = 'rgba(120,110,170,0.12)';
      ctx.lineWidth = 1;
      for (const p of [36, 48, 60, 72, 84, 96]) {
        const y = noteY(p);
        ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke();
      }

      ctx.globalCompositeOperation = 'lighter';
      ctx.lineCap = 'round';
      for (let ti = 0; ti < byTrack.length; ti++) {
        const track = byTrack[ti];
        if (!track || track.length === 0) continue;
        const hue = TRACK_HUES[ti % TRACK_HUES.length];
        const vis = windowSlice(track, t0, t1);
        for (let i = 0; i < vis.length; i++) {
          const a = vis[i];
          const ax = playheadX + (a.start_sec - playhead) * pxPerSec;
          const ay = noteY(a.note);
          if (i > 0) {
            const b = vis[i - 1];
            const bx = playheadX + (b.start_sec - playhead) * pxPerSec;
            const by = noteY(b.note);
            const leap = leapIntensity(b.note, a.note);
            ctx.strokeStyle = `hsla(${hue}, 95%, ${60 + leap * 25}%, ${0.35 + leap * 0.5})`;
            ctx.lineWidth = (2 + leap * 4) * (1 + 0.5 * smoothLevel);
            ctx.beginPath(); ctx.moveTo(bx, by); ctx.lineTo(ax, ay); ctx.stroke();
          }
          ctx.fillStyle = `hsla(${hue}, 90%, 72%, 0.9)`;
          ctx.beginPath(); ctx.arc(ax, ay, 2.5, 0, Math.PI * 2); ctx.fill();
        }
      }

      for (let i = blooms.length - 1; i >= 0; i--) {
        const o = blooms[i];
        o.life += 1 / 60;
        if (o.life > 1) { blooms.splice(i, 1); continue; }
        const k = 1 - o.life;
        const rr = o.r * (1 + 0.6 * smoothLevel);
        const g = ctx.createRadialGradient(o.x, o.y, 0, o.x, o.y, rr * 2.5);
        g.addColorStop(0, `hsla(${o.hue}, 100%, 82%, ${0.9 * k})`);
        g.addColorStop(1, `hsla(${o.hue}, 100%, 60%, 0)`);
        ctx.fillStyle = g;
        ctx.beginPath(); ctx.arc(o.x, o.y, rr * 2.5, 0, Math.PI * 2); ctx.fill();
      }

      ctx.globalCompositeOperation = 'source-over';
      ctx.strokeStyle = 'rgba(159, 252, 255, 0.85)';
      ctx.lineWidth = 1.5;
      ctx.beginPath(); ctx.moveTo(playheadX, 0); ctx.lineTo(playheadX, h); ctx.stroke();
    },
    teardown() { byTrack = []; blooms = []; },
  };
}
