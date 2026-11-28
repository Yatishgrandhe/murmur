//! Shared audio helpers for speech recognition: an energy VAD that trims silence (and decides
//! "nothing was said"), 16 kHz WAV encoding, and a filter for phrases speech models invent from
//! silence ("Thank you.", "Thanks for watching!").

const RATE: usize = 16_000;
const FRAME: usize = RATE / 50; // 20 ms

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
/// Whether a failure is worth retrying with the OS recognizer (network/server trouble), as
/// opposed to something the user must fix (sign-in).
pub fn is_transient(err: &str) -> bool {
    err.starts_with("Network") || err.contains("busy") || err.contains("timed out") || err.contains("limit was reached")
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

}
