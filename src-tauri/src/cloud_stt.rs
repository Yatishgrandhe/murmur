//! Cloud speech-to-text through the OpenAI API (`gpt-transcribe` by default), using the same
//! OpenAI API key as the "OpenAI API" rewrite option. It replaces the OS recognizer when the
//! Voice setting is "OpenAI" and a key is saved.
//!
//! Anthropic has no speech-to-text API, and Claude/ChatGPT plan logins may not be reused by
//! third-party apps, so an OpenAI API key is the one supported way to get cloud transcription.
//! Claude (or ChatGPT) still does the rewrite afterwards, from the text.
//!
//! Before uploading, silence is trimmed with a small energy VAD: it makes the upload shorter and
//! stops the model "hearing" phrases like "Thank you." in silence.

use crate::settings;
use std::sync::OnceLock;
use std::time::Duration;

const ENDPOINT: &str = "https://api.openai.com/v1/audio/transcriptions";
const RATE: usize = 16_000;
const FRAME: usize = RATE / 50; // 20 ms

pub const DEFAULT_MODEL: &str = "gpt-transcribe";

/// One shared client so the TLS connection is reused between dictations.
fn client() -> &'static reqwest::blocking::Client {
    static C: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(40))
            .connect_timeout(Duration::from_secs(8))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_default()
    })
}

pub fn has_key() -> bool {
    settings::get_secret("openai_key").is_some()
}

/// Opens the connection to OpenAI while the user is still talking, so the upload after they let
/// go doesn't pay for DNS + TLS. The response (401 without auth) is ignored.
pub fn warm_up() {
    std::thread::spawn(|| {
        let _ = client().head("https://api.openai.com/v1/models").timeout(Duration::from_secs(5)).send();
    });
}

// ───────────────────────────── voice activity ─────────────────────────────

fn rms(x: &[f32]) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt()
}

/// Result of trimming: the samples to upload and how much of them is voiced.
pub struct Trimmed {
    pub audio: Vec<f32>,
    pub voiced_ms: usize,
}

/// Energy VAD: a frame is speech when its RMS is well above the noise floor (the quietest 10% of
/// frames) and above an absolute minimum. Keeps 300 ms around speech; returns None when there is
/// less than ~150 ms of speech in total.
pub fn trim_silence(x: &[f32]) -> Option<Trimmed> {
    let frames: Vec<f32> = x.chunks(FRAME).map(rms).collect();
    if frames.is_empty() {
        return None;
    }
    let mut sorted = frames.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let floor = sorted[sorted.len() / 10].max(1e-4);
    let thresh = (floor * 3.0).max(0.004);
    let voiced: Vec<bool> = frames.iter().map(|&e| e > thresh).collect();
    let n_voiced = voiced.iter().filter(|&&v| v).count();
    if n_voiced < 8 {
        return None;
    }
    let first = voiced.iter().position(|&v| v)?;
    let last = voiced.iter().rposition(|&v| v)?;
    let pad = 15; // 300 ms
    let start = first.saturating_sub(pad) * FRAME;
    let end = ((last + 1 + pad) * FRAME).min(x.len());
    Some(Trimmed { audio: x[start..end].to_vec(), voiced_ms: n_voiced * 20 })
}

/// 16 kHz mono 16-bit PCM WAV in memory, normalized so quiet microphones still upload a clear signal.
pub fn wav_bytes(audio: &[f32]) -> Vec<u8> {
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let gain = if peak > 0.0 { (0.8 / peak).clamp(1.0, 20.0) } else { 1.0 };
    let data_len = (audio.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&(RATE as u32).to_le_bytes());
    b.extend_from_slice(&(RATE as u32 * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for v in audio {
        b.extend_from_slice(&(((v * gain).clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    b
}

// ───────────────────────────── hallucinations ─────────────────────────────

/// Phrases speech models produce from silence or breath noise (subtitle credits in their
/// training data). Dropped only when there was very little actual speech.
const PHANTOMS: [&str; 14] = [
    "thank you", "thanks", "thank you very much", "thanks for watching", "thank you for watching",
    "thanks for watching!", "you", "so", "the", "oh", "bye", "okay", "subtitles by the amara.org community", "please subscribe",
];

fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.').collect::<String>().trim_matches(|c: char| c == '.' || c.is_whitespace()).to_string()
}

pub fn is_phantom(text: &str, voiced_ms: usize) -> bool {
    let n = norm(text);
    if n.is_empty() {
        return true;
    }
    if n.starts_with("subtitles by") || n.starts_with("transcribed by") || n.starts_with("captions by") {
        return true;
    }
    voiced_ms < 1_200 && PHANTOMS.iter().any(|p| norm(p) == n)
}

// ───────────────────────────── request ─────────────────────────────

/// Keywords for `gpt-transcribe`: no angle brackets or line breaks, or the request is rejected.
fn keywords(vocab: &[String]) -> Vec<String> {
    vocab
        .iter()
        .map(|w| w.replace(['<', '>', '\r', '\n'], " ").trim().to_string())
        .filter(|w| !w.is_empty())
        .take(50)
        .collect()
}

fn friendly(code: u16, v: &serde_json::Value) -> String {
    let err = &v["error"];
    let msg = err["message"].as_str().unwrap_or("request failed");
    let kind = format!("{} {}", err["type"].as_str().unwrap_or(""), err["code"].as_str().unwrap_or(""));
    match code {
        401 => "OpenAI rejected the API key. Check it in Murmur › AI › OpenAI API.".into(),
        403 => format!("OpenAI refused the request: {msg}"),
        429 if kind.contains("quota") || kind.contains("billing") || kind.contains("credit") => {
            "Your OpenAI account is out of credit or hit its spend limit.".into()
        }
        429 => "OpenAI is rate-limiting requests. Try again in a moment.".into(),
        500..=599 => "OpenAI's speech service is busy. Try again in a moment.".into(),
        _ => format!("OpenAI speech error {code}: {msg}"),
    }
}

/// Whether a failure is worth retrying with the OS recognizer (network/server trouble), as
/// opposed to something the user must fix (key, credit).
pub fn is_transient(err: &str) -> bool {
    err.starts_with("Network") || err.contains("busy") || err.contains("rate-limiting") || err.contains("timed out")
}

fn send(wav: &[u8], model: &str, lang: &str, vocab: &[String], rich: bool) -> Result<(u16, serde_json::Value), String> {
    use reqwest::blocking::multipart::{Form, Part};
    let key = settings::get_secret("openai_key").ok_or("Add your OpenAI API key in Murmur › AI to use OpenAI speech recognition.")?;
    let part = Part::bytes(wav.to_vec()).file_name("dictation.wav").mime_str("audio/wav").map_err(|e| e.to_string())?;
    let mut form = Form::new().part("file", part).text("model", model.to_string());
    if !rich {
        // bare retry: only the required fields
        return post(form, &key);
    }
    form = form.text("response_format", "json");
    let words = keywords(vocab);
    let iso = if lang == "auto" { None } else { Some(lang.to_string()) };
    if model == "gpt-transcribe" {
        for w in &words {
            form = form.text("keywords[]", w.clone());
        }
        if let Some(l) = &iso {
            form = form.text("languages[]", l.clone());
        }
    } else {
        if let Some(l) = &iso {
            form = form.text("language", l.clone());
        }
        form = form.text("temperature", "0");
        if !words.is_empty() {
            form = form.text("prompt", format!("Glossary: {}.", words.join(", ")));
        }
    }
    post(form, &key)
}

fn post(form: reqwest::blocking::multipart::Form, key: &str) -> Result<(u16, serde_json::Value), String> {
    let resp = client().post(ENDPOINT).bearer_auth(key).multipart(form).send().map_err(|e| {
        if e.is_timeout() {
            "Network error: OpenAI speech recognition timed out".to_string()
        } else {
            format!("Network error: {e}")
        }
    })?;
    let code = resp.status().as_u16();
    let v: serde_json::Value = resp.json().unwrap_or(serde_json::Value::Null);
    Ok((code, v))
}

/// Transcribes a finished 16 kHz recording. Returns "" when nothing was said.
pub fn transcribe(audio: &[f32], model: &str, lang: &str, vocab: &[String]) -> Result<String, String> {
    let Some(t) = trim_silence(audio) else { return Ok(String::new()) };
    let wav = wav_bytes(&t.audio);
    let model = if model.trim().is_empty() { DEFAULT_MODEL } else { model.trim() };
    let (mut code, mut v) = send(&wav, model, lang, vocab, true)?;
    // a 400 about an optional field (keywords, languages, prompt): retry with the bare request
    if code == 400 {
        (code, v) = send(&wav, model, lang, &[], false)?;
    }
    if !(200..300).contains(&code) {
        return Err(friendly(code, &v));
    }
    let text = v["text"].as_str().unwrap_or("").trim().to_string();
    let glossary = format!("Glossary: {}.", keywords(vocab).join(", "));
    if is_phantom(&text, t.voiced_ms) || text == glossary {
        return Ok(String::new());
    }
    Ok(crate::stt::clean_transcript(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(ms: usize, amp: f32) -> Vec<f32> {
        (0..RATE * ms / 1000).map(|i| amp * (i as f32 * 0.07).sin()).collect()
    }

    #[test]
    fn silence_is_skipped() {
        assert!(trim_silence(&vec![0.0; RATE * 2]).is_none());
        assert!(trim_silence(&tone(2000, 0.001)).is_none());
    }

    #[test]
    fn trims_around_speech() {
        let mut x = vec![0.0005; RATE];
        x.extend(tone(1000, 0.3));
        x.extend(vec![0.0005; RATE]);
        let t = trim_silence(&x).unwrap();
        assert!(t.voiced_ms >= 900 && t.voiced_ms <= 1020, "{}", t.voiced_ms);
        // 1 s of speech + 2 × 300 ms padding
        assert!(t.audio.len() <= RATE * 17 / 10 && t.audio.len() >= RATE * 15 / 10, "{}", t.audio.len());
    }

    #[test]
    fn wav_header() {
        let b = wav_bytes(&[0.0, 0.5, -0.5]);
        assert_eq!(&b[0..4], b"RIFF");
        assert_eq!(b.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes(b[24..28].try_into().unwrap()), 16_000);
    }

    #[test]
    fn phantoms() {
        assert!(is_phantom("Thank you.", 400));
        assert!(!is_phantom("Thank you.", 3000));
        assert!(is_phantom("Subtitles by the Amara.org community", 5000));
        assert!(!is_phantom("Ship it on Tuesday.", 400));
        assert!(is_phantom("  ", 4000));
    }

    #[test]
    fn keyword_sanitizing() {
        let k = keywords(&["Murmur".into(), " <Tauri>\n".into(), "".into()]);
        assert_eq!(k, vec!["Murmur".to_string(), "Tauri".to_string()]);
    }
}
