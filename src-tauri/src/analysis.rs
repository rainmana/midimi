use std::sync::Arc;
use rustfft::{num_complex::Complex, Fft, FftPlanner};

pub struct BandAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buffer: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    band_edges: Vec<usize>,
}

impl BandAnalyzer {
    pub fn new(fft_size: usize, n_bands: usize, sample_rate: f32) -> Self {
        // Plan ONCE; re-planning per frame is the dominant perf footgun.
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let denom = (fft_size - 1) as f32;
        let window: Vec<f32> = (0..fft_size)
            .map(|n| 0.5 * (1.0 - (std::f32::consts::TAU * n as f32 / denom).cos()))
            .collect();
        let scratch = vec![Complex { re: 0.0, im: 0.0 }; fft.get_inplace_scratch_len()];

        // Log-spaced band edges in bin indices over the usable 1..=N/2 range.
        let nyq_bin = fft_size / 2;
        let bin_hz = sample_rate / fft_size as f32;
        let f_min = 40.0_f32;
        let f_max = sample_rate * 0.5;
        let mut band_edges = Vec::with_capacity(n_bands + 1);
        for b in 0..=n_bands {
            let frac = b as f32 / n_bands as f32;
            let f = f_min * (f_max / f_min).powf(frac);
            band_edges.push(((f / bin_hz).round() as usize).clamp(1, nyq_bin));
        }
        Self {
            fft,
            window,
            buffer: vec![Complex { re: 0.0, im: 0.0 }; fft_size],
            scratch,
            band_edges,
        }
    }

    /// `samples` must be >= fft_size long. Returns one magnitude per band (~0..1).
    pub fn analyze(&mut self, samples: &[f32]) -> Vec<f32> {
        let n = self.buffer.len();
        for i in 0..n {
            self.buffer[i] = Complex { re: samples[i] * self.window[i], im: 0.0 };
        }
        self.fft.process_with_scratch(&mut self.buffer, &mut self.scratch);
        let norm = 1.0 / (n as f32 * 0.5);
        let mut out = Vec::with_capacity(self.band_edges.len() - 1);
        for w in self.band_edges.windows(2) {
            let lo = w[0];
            let hi = w[1].max(w[0] + 1);
            let mut sum = 0.0_f32;
            for bin in lo..hi {
                sum += self.buffer[bin].norm();
            }
            out.push(((sum / (hi - lo) as f32) * norm).min(1.0));
        }
        out
    }
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn loud_band_matches_input_frequency() {
        let sr = 44_100.0_f32;
        let n = 1024;
        let n_bands = 8;
        // Pure tone at 2000 Hz.
        let freq = 2000.0_f32;
        let samples: Vec<f32> = (0..n).map(|i| (TAU * freq * i as f32 / sr).sin()).collect();
        let mut a = BandAnalyzer::new(n, n_bands, sr);
        let bands = a.analyze(&samples);
        assert_eq!(bands.len(), n_bands);
        // The hottest band should be the one whose frequency range contains 2000 Hz,
        // and it should clearly dominate a low band.
        let max_idx = bands.iter().enumerate().max_by(|x, y| x.1.partial_cmp(y.1).unwrap()).unwrap().0;
        assert!(max_idx >= 3, "2kHz should land in an upper band, got band {max_idx}");
        assert!(bands[max_idx] > bands[0] * 5.0, "tone band must dominate the lowest band");
    }

    #[test]
    fn rms_of_silence_is_zero_and_signal_is_positive() {
        assert!(rms(&[0.0; 256]) < 1e-9);
        assert!(rms(&[0.5; 256]) > 0.4);
    }
}
