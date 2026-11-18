//! Microphone capture on a dedicated thread (cpal streams aren't Send on macOS),
//! live level metering for the overlay waves, resampling to 16 kHz mono for Whisper,
//! and tiny synthesized UI sounds.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

type Captured = Result<(Vec<f32>, u32), String>;

pub struct Recording {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<Captured>>,
}

impl Recording {
    /// Starts capturing. `on_level` is called ~30×/s with a 0..1 loudness value.
    pub fn start(on_level: impl Fn(f32) + Send + 'static) -> Recording {
        let stop = Arc::new(AtomicBool::new(false));
        let s2 = stop.clone();
        let handle = std::thread::Builder::new()
            .name("murmur-mic".into())
            .spawn(move || capture(s2, on_level))
            .expect("spawn mic thread");
        Recording { stop, handle: Some(handle) }
    }

    /// Stops and returns 16 kHz mono samples.
    pub fn finish(mut self) -> Result<Vec<f32>, String> {
        self.stop.store(true, Ordering::SeqCst);
        let (samples, rate) = self
            .handle
            .take()
            .unwrap()
            .join()
            .map_err(|_| "Microphone thread crashed".to_string())??;
        if !samples.is_empty() && samples.iter().all(|&s| s == 0.0) {
            return Err(mic_denied_message());
        }
        Ok(resample_to_16k(&samples, rate))
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn mic_denied_message() -> String {
    if cfg!(target_os = "macos") {
        "Microphone is blocked. Allow Murmur in System Settings › Privacy & Security › Microphone.".into()
    } else {
        "Microphone is blocked. Allow desktop apps in Settings › Privacy › Microphone.".into()
    }
}

/// Name/format of the microphone used last (for the diagnostic log).
pub static LAST_DEVICE: Mutex<String> = Mutex::new(String::new());

/// Streams the microphone continuously (wake word), handing over 16 kHz mono chunks ~10×/s
/// until `stop` is set.
pub fn stream_mic(stop: Arc<AtomicBool>, mut on_chunk: impl FnMut(&[f32])) -> Result<(), String> {
    let buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let level = Arc::new(AtomicU32::new(0));
    let (stream, rate) = open_input(buf.clone(), level)?;
    // Windows privacy settings that block the mic make it deliver exact zeros forever.
    // ~3 s of them in a row means "blocked", so report it instead of listening to nothing.
    let mut zero_run = 0usize;
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(100));
        let raw = std::mem::take(&mut *buf.lock());
        if !raw.is_empty() {
            let chunk = resample_to_16k(&raw, rate);
            if cfg!(windows) {
                if chunk.iter().all(|&s| s == 0.0) {
                    zero_run += chunk.len();
                } else {
                    zero_run = 0;
                }
                if zero_run >= 3 * 16_000 {
                    drop(stream);
                    return Err(mic_denied_message());
                }
            }
            on_chunk(&chunk);
        }
    }
    drop(stream);
    Ok(())
}

fn open_input(buf: Arc<Mutex<Vec<f32>>>, level: Arc<AtomicU32>) -> Result<(cpal::Stream, u32), String> {
    let host = cpal::default_host();
    let dev = host.default_input_device().ok_or("No microphone found")?;
    let cfg = dev.default_input_config().map_err(|e| format!("Microphone unavailable: {e}"))?;
    *LAST_DEVICE.lock() = format!("{} · {} Hz · {} ch · {:?}", dev.name().unwrap_or_default(), cfg.sample_rate().0, cfg.channels(), cfg.sample_format());
    let rate = cfg.sample_rate().0;
    let ch = cfg.channels() as usize;
    let err_fn = |e| eprintln!("murmur: audio stream error: {e}");
    let stream_cfg: cpal::StreamConfig = cfg.clone().into();

    macro_rules! build {
        ($t:ty, $conv:expr) => {{
            let (b, l) = (buf.clone(), level.clone());
            dev.build_input_stream(
                &stream_cfg,
                move |d: &[$t], _: &_| {
                    let conv = $conv;
                    push(d.chunks(ch).map(|f| f.iter().map(|&s| conv(s)).sum::<f32>() / ch as f32), &b, &l)
                },
                err_fn,
                None,
            )
        }};
    }
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => build!(f32, |s: f32| s),
        cpal::SampleFormat::I16 => build!(i16, |s: i16| s as f32 / 32768.0),
        cpal::SampleFormat::U16 => build!(u16, |s: u16| (s as f32 - 32768.0) / 32768.0),
        cpal::SampleFormat::I32 => build!(i32, |s: i32| s as f32 / 2147483648.0),
        f => return Err(format!("Unsupported microphone format {f:?}")),
    }
    .map_err(|e| format!("Couldn't open microphone: {e}"))?;
    stream.play().map_err(|e| format!("Couldn't start microphone: {e}"))?;
    Ok((stream, rate))
}

fn capture(stop: Arc<AtomicBool>, on_level: impl Fn(f32)) -> Captured {
    let buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(48_000 * 30)));
    let level = Arc::new(AtomicU32::new(0));
    let (stream, rate) = open_input(buf.clone(), level.clone())?;

    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(33));
        on_level(f32::from_bits(level.load(Ordering::Relaxed)));
    }
    drop(stream);
    let samples = std::mem::take(&mut *buf.lock());
    Ok((samples, rate))
    // note: replay() mirrors this loop for self-tests
}

fn push(mono: impl Iterator<Item = f32>, buf: &Mutex<Vec<f32>>, level: &AtomicU32) {
    let mut b = buf.lock();
    let start = b.len();
    b.extend(mono);
    let chunk = &b[start..];
    if chunk.is_empty() {
        return;
    }
    let v = loudness(chunk);
    let prev = f32::from_bits(level.load(Ordering::Relaxed));
    let smoothed = if v > prev { v } else { prev * 0.82 + v * 0.18 };
    level.store(smoothed.to_bits(), Ordering::Relaxed);
}

/// Perceptual-ish loudness: speech RMS ~0.01–0.2 → 0..1.
fn loudness(chunk: &[f32]) -> f32 {
    let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len().max(1) as f32).sqrt();
    ((rms.max(1e-5).log10() + 2.6) / 2.0).clamp(0.0, 1.0)
}

impl Recording {
    /// Self-test source: plays back recorded samples in real time instead of the microphone,
    /// driving the same level meter, so the whole app can be exercised without a person talking.
    pub fn replay(samples: Vec<f32>, rate: u32, on_level: impl Fn(f32) + Send + 'static) -> Recording {
        let stop = Arc::new(AtomicBool::new(false));
        let s2 = stop.clone();
        let handle = std::thread::spawn(move || {
            let chunk = (rate / 30).max(1) as usize;
            let mut level = 0.0f32;
            let mut chunks = samples.chunks(chunk);
            while !s2.load(Ordering::SeqCst) {
                let v = chunks.next().map(loudness).unwrap_or(0.0);
                level = if v > level { v } else { level * 0.82 + v * 0.18 };
                on_level(level);
                std::thread::sleep(Duration::from_millis(33));
            }
            Ok((samples, rate))
        });
        Recording { stop, handle: Some(handle) }
    }
}

/// Reads a WAV file as mono f32 (used by the self-test).
pub fn read_wav(path: &str) -> Result<(Vec<f32>, u32), String> {
    let mut r = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = r.spec();
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().map_while(Result::ok).collect(),
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>().map_while(Result::ok).map(|x| x as f32 / scale).collect()
        }
    };
    let mono = raw.chunks(spec.channels as usize).map(|c| c.iter().sum::<f32>() / c.len() as f32).collect();
    Ok((mono, spec.sample_rate))
}

/// Resample mono audio to 16 kHz. Integer ratios use box-filter decimation;
/// others a moving-average anti-alias filter + linear interpolation (plenty for speech).
pub fn resample_to_16k(x: &[f32], rate: u32) -> Vec<f32> {
    const OUT: u32 = 16_000;
    if rate == OUT || x.is_empty() {
        return x.to_vec();
    }
    if rate.is_multiple_of(OUT) {
        let f = (rate / OUT) as usize;
        return x.chunks(f).map(|c| c.iter().sum::<f32>() / c.len() as f32).collect();
    }
    let ratio = rate as f64 / OUT as f64;
    let src: Vec<f32> = if ratio > 1.0 {
        let w = ratio.ceil() as usize;
        let mut acc = 0.0f32;
        let mut out = Vec::with_capacity(x.len());
        for i in 0..x.len() {
            acc += x[i];
            if i >= w {
                acc -= x[i - w];
            }
            out.push(acc / (i + 1).min(w) as f32);
        }
        out
    } else {
        x.to_vec()
    };
    let n = ((x.len() as f64) / ratio).floor() as usize;
    (0..n)
        .map(|i| {
            let p = i as f64 * ratio;
            let j = p.floor() as usize;
            let t = (p - j as f64) as f32;
            let a = src[j.min(src.len() - 1)];
            let b = src[(j + 1).min(src.len() - 1)];
            a + (b - a) * t
        })
        .collect()
}

/// Loudness gate so silence doesn't reach Whisper (which hallucinates "Thank you." on silence).
pub fn has_speech(x: &[f32]) -> bool {
    if x.len() < 16_000 / 4 {
        return false;
    }
    // loudest 100 ms window
    x.chunks(1600)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .fold(0.0, f32::max)
        > 0.006
}

#[derive(Clone, Copy)]
pub enum Blip {
    Start,
    Stop,
    Error,
}

/// Plays a short, soft synthesized chime on the default output device.
pub fn blip(kind: Blip) {
    std::thread::spawn(move || {
        let _ = (|| -> Option<()> {
            let dev = cpal::default_host().default_output_device()?;
            let cfg = dev.default_output_config().ok()?;
            let rate = cfg.sample_rate().0 as f32;
            let ch = cfg.channels() as usize;
            let notes: &[(f32, f32)] = match kind {
                Blip::Start => &[(659.25, 0.07), (987.77, 0.11)],
                Blip::Stop => &[(987.77, 0.06), (739.99, 0.10)],
                Blip::Error => &[(293.66, 0.09), (246.94, 0.16)],
            };
            let mut wave = Vec::new();
            for &(f, d) in notes {
                let n = (d * rate) as usize;
                for i in 0..n {
                    let t = i as f32 / rate;
                    let env = (t / 0.006).min(1.0) * (-(t / d) * 4.0).exp();
                    let s = (std::f32::consts::TAU * f * t).sin() * 0.6 + (std::f32::consts::TAU * f * 2.0 * t).sin() * 0.12;
                    wave.push(s * env * 0.16);
                }
            }
            let len = wave.len();
            let wave = Arc::new(wave);
            let pos = Arc::new(Mutex::new(0usize));
            let stream_cfg = cfg.config();
            // The wave is f32 in -1..1; convert it to whatever sample format the device wants.
            macro_rules! out {
                ($t:ty, $conv:expr) => {{
                    let (w2, p2) = (wave.clone(), pos.clone());
                    let conv = $conv;
                    dev.build_output_stream(
                        &stream_cfg,
                        move |out: &mut [$t], _: &_| {
                            let mut p = p2.lock();
                            for frame in out.chunks_mut(ch) {
                                let v = w2.get(*p).copied().unwrap_or(0.0);
                                *p += 1;
                                let s: $t = conv(v);
                                frame.iter_mut().for_each(|x| *x = s);
                            }
                        },
                        |_| {},
                        None,
                    )
                }};
            }
            let stream = match cfg.sample_format() {
                cpal::SampleFormat::F32 => out!(f32, |v: f32| v),
                cpal::SampleFormat::I16 => out!(i16, |v: f32| (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16),
                cpal::SampleFormat::I32 => out!(i32, |v: f32| (v.clamp(-1.0, 1.0) * i32::MAX as f32) as i32),
                _ => return None,
            }
            .ok()?;
            stream.play().ok()?;
            std::thread::sleep(Duration::from_secs_f32(len as f32 / rate + 0.08));
            Some(())
        })();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_lengths() {
        let x = vec![0.5f32; 48_000];
        assert_eq!(resample_to_16k(&x, 48_000).len(), 16_000);
        assert_eq!(resample_to_16k(&vec![0.5f32; 44_100], 44_100).len(), 16_000);
        assert_eq!(resample_to_16k(&vec![0.5f32; 16_000], 16_000).len(), 16_000);
        assert!((resample_to_16k(&x, 48_000)[100] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn resample_keeps_low_tone() {
        // 200 Hz tone at 44.1 kHz should survive with ~unchanged amplitude
        let x: Vec<f32> = (0..44_100).map(|i| (i as f32 / 44_100.0 * 200.0 * std::f32::consts::TAU).sin()).collect();
        let y = resample_to_16k(&x, 44_100);
        let peak = y[1000..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.9 && peak < 1.01, "peak {peak}");
    }

    #[test]
    fn speech_gate() {
        assert!(!has_speech(&vec![0.0; 32_000]));
        assert!(!has_speech(&vec![0.3; 1_000]));
        let tone: Vec<f32> = (0..32_000).map(|i| 0.1 * (i as f32 * 0.05).sin()).collect();
        assert!(has_speech(&tone));
    }
}

/// Decides when a hands-free take is over. Adapts to the mic: tracks the background-noise level and
/// treats anything clearly above it as speech, so quiet voices and laptop mics work. Natural pauses
/// between phrases don't end the take; only a longer pause after real speech does.
pub struct EndOfSpeech {
    floor: f32,
    speech_ms: f32,
    last_speech_ms: f32,
    t_ms: f32,
    fired: bool,
}

impl Default for EndOfSpeech {
    fn default() -> Self {
        Self::new()
    }
}

impl EndOfSpeech {
    /// Pause after speech that ends the take.
    pub const PAUSE_MS: f32 = 2200.0;
    /// Speech needed before a pause can end the take (ignores coughs, clicks, the start chime).
    pub const MIN_SPEECH_MS: f32 = 250.0;
    /// Give up if nothing was said at all.
    pub const NO_SPEECH_MS: f32 = 15_000.0;

    pub fn new() -> Self {
        EndOfSpeech { floor: f32::MAX, speech_ms: 0.0, last_speech_ms: 0.0, t_ms: 0.0, fired: false }
    }

    /// Feed one loudness sample (0..1) taken `dt_ms` after the previous one.
    /// Returns Some(true) when the speaker has finished, Some(false) if they never spoke.
    pub fn push(&mut self, level: f32, dt_ms: f32) -> Option<bool> {
        if self.fired {
            return None;
        }
        self.t_ms += dt_ms;
        // noise floor: follows quiet moments immediately, rises only very slowly
        if level < self.floor {
            self.floor = level;
        } else {
            self.floor += (level - self.floor) * 0.002;
        }
        let threshold = (self.floor + 0.12).max(0.12);
        if level > threshold {
            self.speech_ms += dt_ms;
            self.last_speech_ms = self.t_ms;
        }
        let spoke = self.speech_ms >= Self::MIN_SPEECH_MS;
        let result = if spoke && self.t_ms - self.last_speech_ms >= Self::PAUSE_MS {
            Some(true)
        } else if !spoke && self.t_ms >= Self::NO_SPEECH_MS {
            Some(false)
        } else {
            None
        };
        self.fired = result.is_some();
        result
    }
}

#[cfg(test)]
mod eos_tests {
    use super::EndOfSpeech;

    fn run(levels: &[(f32, u32)]) -> Option<(bool, f32)> {
        let mut e = EndOfSpeech::new();
        let mut t = 0.0;
        for &(level, ms) in levels {
            for _ in 0..(ms / 33) {
                t += 33.0;
                if let Some(r) = e.push(level, 33.0) {
                    return Some((r, t));
                }
            }
        }
        None
    }

    #[test]
    fn quiet_voice_with_pauses_is_not_cut_off() {
        // real MacBook Air levels: quiet speech ≈ 0.25–0.35, room ≈ 0.02, 0.9 s and 1.6 s gaps between phrases
        let r = run(&[(0.02, 400), (0.3, 1500), (0.03, 900), (0.26, 1200), (0.02, 1600), (0.33, 1000), (0.02, 1500)]);
        assert_eq!(r, None, "stopped while the user was still mid-thought");
    }

    #[test]
    fn stops_after_a_real_pause() {
        let (spoke, t) = run(&[(0.02, 300), (0.3, 2000), (0.02, 4000)]).unwrap();
        assert!(spoke);
        assert!((4400.0..4700.0).contains(&t), "fired at {t} ms");
    }

    #[test]
    fn loud_room_still_works() {
        // noisy café: floor 0.25, speech 0.5
        let (spoke, _) = run(&[(0.25, 500), (0.5, 1500), (0.25, 3000)]).unwrap();
        assert!(spoke);
    }

    #[test]
    fn silence_gives_up() {
        let (spoke, t) = run(&[(0.01, 20_000)]).unwrap();
        assert!(!spoke && t >= 15_000.0);
    }

    #[test]
    fn a_click_is_not_speech() {
        let r = run(&[(0.02, 300), (0.6, 99), (0.02, 6000)]);
        assert_eq!(r, None);
    }
}
