import { writeFileSync, mkdirSync } from 'node:fs';
mkdirSync('assets/demo', { recursive: true });
const tpqn = 96, notes = [60,62,64,65,67,69,71,72];
const tb = []; // track bytes
const push = (...b) => tb.push(...b);
const vlq = (n) => (n < 128 ? [n] : [0x80 | (n >> 7), n & 0x7f]); // n < 16384 here
for (const n of notes) { push(0x00, 0x90, n, 0x64); push(...vlq(tpqn), 0x80, n, 0x00); }
push(0x00, 0xff, 0x2f, 0x00);
const len = tb.length;
const header = [0x4d,0x54,0x68,0x64, 0,0,0,6, 0,0, 0,1, (tpqn>>8)&0xff, tpqn&0xff];
const trk = [0x4d,0x54,0x72,0x6b, (len>>24)&0xff,(len>>16)&0xff,(len>>8)&0xff,len&0xff, ...tb];
writeFileSync('assets/demo/scale.mid', Buffer.from([...header, ...trk]));
console.log('wrote assets/demo/scale.mid');
