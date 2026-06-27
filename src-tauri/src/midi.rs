use serde::Serialize;
use std::collections::HashMap;
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

#[derive(Serialize, Clone, Debug)]
pub struct Note {
    pub track: usize,
    pub channel: u8,
    pub note: u8,
    pub start_sec: f64,
    pub dur_sec: f64,
    pub velocity: u8,
}

#[derive(Serialize, Clone, Debug)]
pub struct TrackInfo {
    pub index: usize,
    pub name: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct MidiData {
    pub title: Option<String>,
    pub duration_sec: f64,
    pub tracks: Vec<TrackInfo>,
    pub notes: Vec<Note>,
}

// Convert an absolute tick to seconds using a sorted (tick, us_per_quarter) tempo map.
// ponytail: linear scan over tempo changes; real files have a handful. Switch to a
// cumulative prefix + binary search only if a file ever ships thousands of tempo events.
fn tick_to_sec(tick: u64, tempo_map: &[(u64, f64)], ticks_per_beat: f64) -> f64 {
    let mut sec = 0.0_f64;
    let mut last_tick = 0_u64;
    let mut us = 500_000.0_f64; // default 120 BPM until the first Set Tempo
    for &(t, u) in tempo_map.iter() {
        if t >= tick {
            break;
        }
        sec += (t - last_tick) as f64 * (us / 1_000_000.0) / ticks_per_beat;
        last_tick = t;
        us = u;
    }
    sec + (tick - last_tick) as f64 * (us / 1_000_000.0) / ticks_per_beat
}

pub fn parse_midi(bytes: &[u8]) -> Result<MidiData, String> {
    let smf = Smf::parse(bytes).map_err(|e| format!("invalid MIDI: {e}"))?;

    // Metrical => ticks-per-quarter + a tempo map. SMPTE => fixed ticks-per-second.
    let (ticks_per_beat, smpte_tps) = match smf.header.timing {
        Timing::Metrical(tpb) => (tpb.as_int() as f64, None),
        Timing::Timecode(fps, sub) => (1.0, Some(fps.as_f32() as f64 * sub as f64)),
    };

    // Pass 1: build a GLOBAL tempo map (absolute tick -> us/quarter) across all tracks.
    // In format-1 files all tracks share one tick clock; Set Tempo usually lives in track 0
    // but governs every track, so we hoist it to be global.
    let mut tempo_map: Vec<(u64, f64)> = Vec::new();
    if smpte_tps.is_none() {
        for track in smf.tracks.iter() {
            let mut abs: u64 = 0;
            for ev in track.iter() {
                abs += ev.delta.as_int() as u64;
                if let TrackEventKind::Meta(MetaMessage::Tempo(us)) = ev.kind {
                    tempo_map.push((abs, us.as_int() as f64));
                }
            }
        }
        tempo_map.sort_by_key(|&(t, _)| t);
        tempo_map.dedup_by_key(|&mut (t, _)| t);
    }

    let to_sec = |tick: u64| -> f64 {
        match smpte_tps {
            Some(tps) => tick as f64 / tps,
            None => tick_to_sec(tick, &tempo_map, ticks_per_beat),
        }
    };

    // Pass 2: notes + track names.
    let mut notes = Vec::new();
    let mut tracks = Vec::with_capacity(smf.tracks.len());
    for (track_index, track) in smf.tracks.iter().enumerate() {
        let mut abs: u64 = 0;
        let mut name: Option<String> = None;
        let mut open: HashMap<(u8, u8), (f64, u8)> = HashMap::new();
        for ev in track.iter() {
            abs += ev.delta.as_int() as u64;
            match ev.kind {
                TrackEventKind::Meta(MetaMessage::TrackName(n)) => {
                    name = Some(String::from_utf8_lossy(n).into_owned());
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    match message {
                        MidiMessage::NoteOn { key, vel } => {
                            let (k, v) = (key.as_int(), vel.as_int());
                            if v == 0 {
                                if let Some((start, vel0)) = open.remove(&(ch, k)) {
                                    notes.push(Note { track: track_index, channel: ch, note: k,
                                        start_sec: start, dur_sec: to_sec(abs) - start, velocity: vel0 });
                                }
                            } else {
                                open.insert((ch, k), (to_sec(abs), v));
                            }
                        }
                        MidiMessage::NoteOff { key, .. } => {
                            let k = key.as_int();
                            if let Some((start, vel0)) = open.remove(&(ch, k)) {
                                notes.push(Note { track: track_index, channel: ch, note: k,
                                    start_sec: start, dur_sec: to_sec(abs) - start, velocity: vel0 });
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        // Close any notes left hanging at the track's final tick.
        let end = to_sec(abs);
        for ((ch, k), (start, vel0)) in open.drain() {
            notes.push(Note { track: track_index, channel: ch, note: k,
                start_sec: start, dur_sec: (end - start).max(0.0), velocity: vel0 });
        }
        tracks.push(TrackInfo { index: track_index, name });
    }

    notes.sort_by(|a, b| a.start_sec.partial_cmp(&b.start_sec).unwrap_or(std::cmp::Ordering::Equal));
    let duration_sec = notes.iter().map(|n| n.start_sec + n.dur_sec).fold(0.0_f64, f64::max);
    let title = tracks.iter().find_map(|t| t.name.clone());

    Ok(MidiData { title, duration_sec, tracks, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal SMF: format 0, 1 track, 96 ticks/quarter, default tempo (120 BPM => quarter = 0.5s).
    // Events: NoteOn ch0 key60 vel100 @t0 ; NoteOff ch0 key60 @ +96 ticks ; EndOfTrack.
    fn tiny_midi() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"MThd");
        v.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0, 96]); // len=6, fmt=0, ntrks=1, div=96
        v.extend_from_slice(b"MTrk");
        let track: [u8; 12] = [
            0x00, 0x90, 0x3C, 0x64, // dt0  NoteOn  ch0 key60 vel100
            0x60, 0x80, 0x3C, 0x00, // dt96 NoteOff ch0 key60 vel0
            0x00, 0xFF, 0x2F, 0x00, // dt0  EndOfTrack
        ];
        v.extend_from_slice(&(track.len() as u32).to_be_bytes());
        v.extend_from_slice(&track);
        v
    }

    #[test]
    fn parses_one_note_with_correct_timing() {
        let data = tiny_midi();
        let md = parse_midi(&data).expect("should parse");
        assert_eq!(md.tracks.len(), 1);
        assert_eq!(md.notes.len(), 1, "exactly one note");
        let n = &md.notes[0];
        assert_eq!(n.note, 60);
        assert_eq!(n.velocity, 100);
        assert!((n.start_sec - 0.0).abs() < 1e-6, "starts at 0");
        assert!((n.dur_sec - 0.5).abs() < 1e-3, "96 ticks @120BPM = 0.5s, got {}", n.dur_sec);
        assert!((md.duration_sec - 0.5).abs() < 1e-3);
    }

    #[test]
    fn note_on_velocity_zero_is_note_off() {
        // NoteOn vel0 should close the note, not open a second.
        let mut v = Vec::new();
        v.extend_from_slice(b"MThd");
        v.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0, 96]);
        v.extend_from_slice(b"MTrk");
        let track: [u8; 12] = [
            0x00, 0x90, 0x40, 0x50, // NoteOn key64 vel80
            0x30, 0x90, 0x40, 0x00, // NoteOn key64 vel0  == NoteOff (dt48)
            0x00, 0xFF, 0x2F, 0x00,
        ];
        v.extend_from_slice(&(track.len() as u32).to_be_bytes());
        v.extend_from_slice(&track);
        let md = parse_midi(&v).unwrap();
        assert_eq!(md.notes.len(), 1);
        assert!((md.notes[0].dur_sec - 0.25).abs() < 1e-3);
    }
}
