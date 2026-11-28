//! On-device speech-to-text with the Whisper model that ships inside the installer
//! (tiny.en, q5_1 quantized, 32 MB), run by whisper.cpp: Metal on Apple Silicon, CPU on Windows.
//! Free, offline, and nothing leaves the computer.
//!
//! Speed: the model is loaded and run once at launch (so the first dictation doesn't pay for
//! loading or GPU warm-up), silence is trimmed first, and the encoder only looks at as much audio
//! as was spoken (`audio_ctx`) instead of Whisper's fixed 30-second window.

use crate::vad;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::OnceLock;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub const MODEL_FILE: &str = "ggml-tiny.en-q5_1.bin";
const RATE: usize = 16_000;

static PATH: OnceLock<PathBuf> = OnceLock::new();
static CTX: Mutex<Option<WhisperContext>> = Mutex::new(None);
static LOAD_ERR: Mutex<Option<String>> = Mutex::new(None);

/// Finds the bundled model (`<resources>/models/` in an installed app, `src-tauri/resources/`
/// in a development build) and loads it in the background.
pub fn init(resource_dir: Option<PathBuf>) {
    let candidates = [
        resource_dir.as_ref().map(|d| d.join("models").join(MODEL_FILE)),
        resource_dir.as_ref().map(|d| d.join("resources").join(MODEL_FILE)),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join(MODEL_FILE)),
    ];
    if let Some(p) = candidates.into_iter().flatten().find(|p| p.is_file()) {
        let _ = PATH.set(p);
    }
    std::thread::spawn(|| {
        let _ = load();
    });
}

pub fn model_path() -> Option<&'static PathBuf> {
    PATH.get()
}

fn threads() -> i32 {
    std::thread::available_parallelism().map(|n| n.get().clamp(2, 8) as i32).unwrap_or(4)
}

fn params(prompt: &str, audio_ctx: i32) -> FullParams<'_, '_> {
    let mut p = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    p.set_language(Some("en"));
    p.set_n_threads(threads());
    p.set_print_progress(false);
    p.set_print_realtime(false);
    p.set_print_special(false);
    p.set_print_timestamps(false);
    p.set_no_timestamps(true);
    p.set_single_segment(true);
    p.set_no_context(true);
    p.set_suppress_blank(true);
    p.set_suppress_nst(true);
    // no temperature fallback: one greedy pass keeps the time predictable
    p.set_temperature_inc(0.0);
    p.set_audio_ctx(audio_ctx);
    if !prompt.is_empty() {
        p.set_initial_prompt(prompt);
    }
    p
}

/// Loads the model once (and runs it on a second of silence so Metal compiles its kernels now).
fn load() -> Result<(), String> {
    let mut g = CTX.lock();
    if g.is_some() {
        return Ok(());
    }
    let path = PATH.get().ok_or("The speech model is missing from this install. Reinstall Murmur.")?;
    if std::env::var_os("MURMUR_WHISPER_LOG").is_none() {
        whisper_rs::install_logging_hooks();
    }
    let t = std::time::Instant::now();
    let ctx = WhisperContext::new_with_params(path, WhisperContextParameters::default()).map_err(|e| {
        let m = format!("Couldn't load the speech model: {e}");
        *LOAD_ERR.lock() = Some(m.clone());
        m
    })?;
    if let Ok(mut st) = ctx.create_state() {
        let _ = st.full(params("", 512), &vec![0.0f32; RATE]);
    }
    crate::report("local_stt_loaded", serde_json::json!({"ms": t.elapsed().as_millis() as u64}));
    *g = Some(ctx);
    *LOAD_ERR.lock() = None;
    Ok(())
}

/// Frees the model. Must run before the process exits (ggml's Metal device asserts at exit that
/// every buffer was released).
pub fn shutdown() {
    *CTX.lock() = None;
}

pub fn ready() -> bool {
    PATH.get().is_some() && LOAD_ERR.lock().is_none()
}

pub fn detail() -> String {
    if let Some(e) = LOAD_ERR.lock().clone() {
        return e;
    }
    if PATH.get().is_none() {
        return "The speech model is missing from this install. Reinstall Murmur.".into();
    }
    "Whisper tiny.en on this computer".into()
}

/// Spelling hints from the Dictionary, in the style Whisper continues from.
fn initial_prompt(vocab: &[String]) -> String {
    let words: Vec<&str> = vocab.iter().map(|w| w.trim()).filter(|w| !w.is_empty()).take(40).collect();
    if words.is_empty() {
        String::new()
    } else {
        format!("Glossary: {}.", words.join(", "))
    }
}

/// Encoder context for a clip: Whisper uses 50 positions per second of audio. A little headroom
/// keeps accuracy; the full window is 1500 (30 s).
fn audio_ctx(samples: usize) -> i32 {
    let secs = samples as f32 / RATE as f32;
    ((secs * 50.0).ceil() as i32 + 64).clamp(256, 1500)
}

/// Transcribes a finished 16 kHz recording on this computer. Returns "" when nothing was said.
pub fn transcribe(audio: &[f32], vocab: &[String]) -> Result<String, String> {
    let Some(t) = vad::trim_silence(audio) else { return Ok(String::new()) };
    load()?;
    let mut buf = t.audio;
    // Whisper is unreliable on very short clips: pad to at least 1.2 s
    if buf.len() < RATE * 6 / 5 {
        buf.resize(RATE * 6 / 5, 0.0);
    }
    // quiet microphones: bring the peak up to a normal level (at most 20x)
    let peak = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if peak > 0.0 && peak < 0.5 {
        let gain = (0.7 / peak).min(20.0);
        buf.iter_mut().for_each(|v| *v *= gain);
    }
    let prompt = initial_prompt(vocab);
    let g = CTX.lock();
    let ctx = g.as_ref().ok_or("The speech model didn't load")?;
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;
    state.full(params(&prompt, audio_ctx(buf.len())), &buf).map_err(|e| format!("Transcription failed: {e}"))?;
    let mut text = String::new();
    for i in 0..state.full_n_segments() {
        if let Some(seg) = state.get_segment(i) {
            if let Ok(s) = seg.to_str_lossy() {
                text.push_str(&s);
            }
        }
    }
    let text = text.trim().to_string();
    if vad::is_phantom(&text, t.voiced_ms) || (!prompt.is_empty() && text == prompt) || text.starts_with('[') || text.starts_with('(') {
        return Ok(String::new());
    }
    Ok(crate::stt::clean_transcript(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_scales_with_length() {
        assert_eq!(audio_ctx(RATE), 256);
        assert_eq!(audio_ctx(RATE * 10), 564);
        assert_eq!(audio_ctx(RATE * 60), 1500);
    }

    #[test]
    fn prompt_from_dictionary() {
        assert_eq!(initial_prompt(&[]), "");
        assert_eq!(initial_prompt(&["Murmur".into(), " ".into(), "Tauri".into()]), "Glossary: Murmur, Tauri.");
    }
}

/// Speed check on real recordings: `MURMUR_TEST_WAVS=a.wav,b.wav cargo test --release --lib bench -- --ignored --nocapture`
#[cfg(test)]
mod bench {
    #[test]
    #[ignore]
    fn bench() {
        let Ok(list) = std::env::var("MURMUR_TEST_WAVS") else { return };
        super::init(None);
        let t = std::time::Instant::now();
        super::load().unwrap();
        println!("load+warm: {} ms", t.elapsed().as_millis());
        for f in list.split(',') {
            let mut r = hound::WavReader::open(f).unwrap();
            let x: Vec<f32> = r.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect();
            for _ in 0..2 {
                let t = std::time::Instant::now();
                let out = super::transcribe(&x, &[]).unwrap();
                println!("{:.1}s audio -> {} ms: {out}", x.len() as f32 / 16000.0, t.elapsed().as_millis());
            }
        }
        super::shutdown();
    }
}
