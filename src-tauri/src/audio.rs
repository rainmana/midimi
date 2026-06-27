use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Producer, RingBuffer};
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

use crate::analysis::{rms, BandAnalyzer};

pub const N_BANDS: usize = 16;
const FFT_SIZE: usize = 1024;
const BLOCK: usize = 1024; // frames per render iteration (FFT_SIZE == BLOCK keeps analysis simple)

#[derive(Clone, Default)]
pub struct Snapshot {
    pub position_sec: f64,
    pub duration_sec: f64,
    pub level: f32,
    pub bands: Vec<f32>,
    pub playing: bool,
}

enum Cmd {
    Load { sf: Arc<SoundFont>, midi: Arc<MidiFile>, duration: f64 },
    Play,
    Pause,
    Seek(f64),
    SetSpeed(f64),
    SetVolume(f32),
}

pub struct AudioEngine {
    tx: Sender<Cmd>,
    snapshot: Arc<Mutex<Snapshot>>,
    _stream: cpal::Stream, // kept alive; dropping it stops audio
    _render: JoinHandle<()>,
}

impl AudioEngine {
    pub fn new() -> Result<AudioEngine, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no audio output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        if supported.sample_format() != cpal::SampleFormat::F32 {
            return Err(format!("unsupported sample format {:?} (expected f32)", supported.sample_format()));
        }
        let sample_rate: u32 = supported.sample_rate();
        // ponytail: force stereo; desktop default outputs are ~always f32 stereo. Revisit if a
        // device rejects this config.
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (producer, mut consumer) = RingBuffer::<f32>::new(BLOCK * 2 * 8);
        let err_fn = |e: cpal::Error| eprintln!("audio stream error: {e}");
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Real-time thread: only pop. Underrun => silence, never block/allocate/lock.
                    for s in data.iter_mut() {
                        *s = consumer.pop().unwrap_or(0.0);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;

        let snapshot = Arc::new(Mutex::new(Snapshot { bands: vec![0.0; N_BANDS], ..Default::default() }));
        let (tx, rx) = channel::<Cmd>();
        let snap2 = snapshot.clone();
        let render = std::thread::spawn(move || render_loop(rx, producer, snap2, sample_rate));

        Ok(AudioEngine { tx, snapshot, _stream: stream, _render: render })
    }

    pub fn shared(&self) -> Arc<Mutex<Snapshot>> { self.snapshot.clone() }
    pub fn load(&self, sf: Arc<SoundFont>, midi: Arc<MidiFile>, duration: f64) { let _ = self.tx.send(Cmd::Load { sf, midi, duration }); }
    pub fn play(&self) { let _ = self.tx.send(Cmd::Play); }
    pub fn pause(&self) { let _ = self.tx.send(Cmd::Pause); }
    pub fn seek(&self, sec: f64) { let _ = self.tx.send(Cmd::Seek(sec)); }
    pub fn set_speed(&self, ratio: f64) { let _ = self.tx.send(Cmd::SetSpeed(ratio)); }
    pub fn set_volume(&self, v: f32) { let _ = self.tx.send(Cmd::SetVolume(v)); }
}

fn render_loop(rx: Receiver<Cmd>, mut producer: Producer<f32>, snapshot: Arc<Mutex<Snapshot>>, sample_rate: u32) {
    let mut analyzer = BandAnalyzer::new(FFT_SIZE, N_BANDS, sample_rate as f32);
    let mut seq: Option<MidiFileSequencer> = None;
    let mut midi: Option<Arc<MidiFile>> = None;
    let mut duration = 0.0_f64;
    let mut playing = false;
    let mut volume = 1.0_f32;

    let mut left = vec![0.0_f32; BLOCK];
    let mut right = vec![0.0_f32; BLOCK];
    let mut mono = vec![0.0_f32; FFT_SIZE];

    loop {
        // Drain control messages (non-blocking).
        loop {
            match rx.try_recv() {
                Ok(Cmd::Load { sf, midi: m, duration: d }) => {
                    let settings = SynthesizerSettings::new(sample_rate as i32);
                    match Synthesizer::new(&sf, &settings) {
                        Ok(synth) => {
                            let mut s = MidiFileSequencer::new(synth);
                            s.play(&m, false);
                            seq = Some(s);
                            midi = Some(m);
                            duration = d;
                            playing = true; // auto-play on open
                        }
                        Err(e) => eprintln!("synthesizer init failed: {e}"),
                    }
                }
                Ok(Cmd::Play) => playing = true,
                Ok(Cmd::Pause) => playing = false,
                Ok(Cmd::SetVolume(v)) => volume = v.clamp(0.0, 2.0),
                Ok(Cmd::SetSpeed(r)) => { if let Some(s) = seq.as_mut() { s.set_speed(r.max(0.0)); } }
                Ok(Cmd::Seek(target)) => {
                    if let (Some(s), Some(m)) = (seq.as_mut(), midi.as_ref()) {
                        // rustysynth has NO seek: replay from 0 then render-and-discard to target.
                        s.play(m, false);
                        while s.get_position() < target && !s.end_of_sequence() {
                            s.render(&mut left, &mut right);
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return, // engine dropped
            }
        }

        let can_render = playing && seq.is_some() && producer.slots() >= BLOCK * 2;
        if can_render {
            let s = seq.as_mut().unwrap();
            s.render(&mut left, &mut right); // planar L/R
            for i in 0..BLOCK {
                let _ = producer.push(left[i] * volume);
                let _ = producer.push(right[i] * volume);
            }
            for i in 0..FFT_SIZE { mono[i] = 0.5 * (left[i] + right[i]); }
            let bands = analyzer.analyze(&mono);
            let level = rms(&left);
            let pos = s.get_position();
            if s.end_of_sequence() { playing = false; }
            if let Ok(mut snap) = snapshot.lock() {
                snap.position_sec = pos;
                snap.duration_sec = duration;
                snap.level = level;
                snap.bands = bands;
                snap.playing = playing;
            }
        } else {
            if let Ok(mut snap) = snapshot.lock() { snap.playing = playing; }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// Render an entire sequence offline to interleaved stereo f32. Used by the test now and by
/// WAV/MP3 export later (feed the result to `hound` / an MP3 encoder).
pub fn render_offline(sf: &Arc<SoundFont>, midi: &Arc<MidiFile>, sample_rate: i32) -> Vec<f32> {
    let settings = SynthesizerSettings::new(sample_rate);
    let synth = Synthesizer::new(sf, &settings).expect("synthesizer");
    let mut seq = MidiFileSequencer::new(synth);
    seq.play(midi, false);
    let mut left = vec![0.0_f32; 4096];
    let mut right = vec![0.0_f32; 4096];
    let mut out = Vec::new();
    let cap_secs = midi.get_length() + 5.0; // safety bound
    while !seq.end_of_sequence() {
        seq.render(&mut left, &mut right);
        for i in 0..left.len() { out.push(left[i]); out.push(right[i]); }
        if out.len() as f64 / 2.0 / sample_rate as f64 > cap_secs { break; }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use rustysynth::{MidiFile, SoundFont};

    fn tiny_midi() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"MThd");
        v.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0, 96]);
        v.extend_from_slice(b"MTrk");
        let track: [u8; 12] = [0x00,0x90,0x3C,0x64, 0x60,0x80,0x3C,0x00, 0x00,0xFF,0x2F,0x00];
        v.extend_from_slice(&(track.len() as u32).to_be_bytes());
        v.extend_from_slice(&track);
        v
    }

    #[test]
    fn offline_render_produces_audible_stereo() {
        let sf_path = "../assets/soundfonts/GeneralUser-GS.sf2";
        if !std::path::Path::new(sf_path).exists() {
            eprintln!("skipping offline_render test: run ./scripts/fetch-soundfont.sh first");
            return;
        }
        let sf_bytes = std::fs::read(sf_path).unwrap();
        let sf = Arc::new(SoundFont::new(&mut std::io::Cursor::new(sf_bytes)).unwrap());
        let midi = Arc::new(MidiFile::new(&mut std::io::Cursor::new(tiny_midi())).unwrap());
        let pcm = render_offline(&sf, &midi, 44_100);
        // ~0.5s of stereo at 44.1k ≈ 44100 interleaved samples (allow generous slack for release tail).
        assert!(pcm.len() >= 20_000, "expected a meaningful number of samples, got {}", pcm.len());
        assert!(pcm.len() % 2 == 0, "interleaved stereo => even length");
        let peak = pcm.iter().fold(0.0_f32, |m, &s| m.max(s.abs()));
        assert!(peak > 0.001, "rendered audio should not be silent, peak={peak}");
    }
}
