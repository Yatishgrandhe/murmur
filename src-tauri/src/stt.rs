//! On-device speech-to-text with whisper.cpp (Metal on Apple Silicon).

use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Serialize, Clone, Copy)]
pub struct ModelInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub file: &'static str,
    pub size_mb: u32,
    pub english_only: bool,
}

pub const MODELS: &[ModelInfo] = &[
    ModelInfo { id: "tiny.en", label: "Swift", blurb: "Fastest · English", file: "ggml-tiny.en.bin", size_mb: 75, english_only: true },
    ModelInfo { id: "base.en", label: "Balanced", blurb: "Recommended · English", file: "ggml-base.en.bin", size_mb: 142, english_only: true },
    ModelInfo { id: "small.en", label: "Precise", blurb: "Most accurate · English", file: "ggml-small.en.bin", size_mb: 466, english_only: true },
    ModelInfo { id: "large-v3-turbo-q5_0", label: "Worldwide", blurb: "Best quality · 99 languages", file: "ggml-large-v3-turbo-q5_0.bin", size_mb: 547, english_only: false },
];

pub fn model(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.id == id)
}

pub fn models_dir() -> PathBuf {
    crate::settings::models_base_dir().join("models")
}

pub fn model_path(id: &str) -> Option<PathBuf> {
    model(id).map(|m| models_dir().join(m.file))
}

pub fn installed(id: &str) -> bool {
    model_path(id).map(|p| p.is_file()).unwrap_or(false)
}

pub fn url(m: &ModelInfo) -> String {
    format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", m.file)
}

#[derive(Default)]
pub struct Stt {
    ctx: Mutex<Option<(String, WhisperContext)>>,
}

impl Stt {
    pub fn new() -> Stt {
        // route whisper.cpp's chatty stderr logging into the (silent) `log` facade
        if std::env::var_os("MURMUR_WHISPER_LOG").is_none() { whisper_rs::install_logging_hooks(); }
        Stt::default()
    }

    pub fn ensure_loaded(&self, id: &str) -> Result<(), String> {
        let mut g = self.ctx.lock();
        if matches!(&*g, Some((cur, _)) if cur == id) {
            return Ok(());
        }
        let path = model_path(id).ok_or("Unknown voice model")?;
        if !path.is_file() {
            return Err("Download a voice model in Murmur settings first.".into());
        }
        let ctx = WhisperContext::new_with_params(&path, WhisperContextParameters::default())
            .map_err(|e| format!("Couldn't load voice model: {e}"))?;
        // first inference compiles GPU kernels (~0.5 s); pay that now, not on the user's first dictation
        if let Ok(mut st) = ctx.create_state() {
            let mut p = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            p.set_print_progress(false);
            p.set_print_realtime(false);
            p.set_print_special(false);
            p.set_print_timestamps(false);
            p.set_single_segment(true);
            let _ = st.full(p, &vec![0.0f32; 16_000]);
        }
        *g = Some((id.to_string(), ctx));
        Ok(())
    }

    pub fn unload(&self) {
        *self.ctx.lock() = None;
    }

    pub fn transcribe(&self, id: &str, audio: &[f32], language: &str, vocabulary: &[String]) -> Result<String, String> {
        self.ensure_loaded(id)?;
        let g = self.ctx.lock();
        let (_, ctx) = g.as_ref().unwrap();
        let mut state = ctx.create_state().map_err(|e| e.to_string())?;
        let mut p = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let english_only = model(id).map(|m| m.english_only).unwrap_or(false);
        let lang = if english_only { "en" } else if language == "auto" || language.is_empty() { "auto" } else { language };
        p.set_language(Some(lang));
        p.set_n_threads(std::thread::available_parallelism().map(|n| n.get().min(8) as i32).unwrap_or(4));
        p.set_print_progress(false);
        p.set_print_realtime(false);
        p.set_print_special(false);
        p.set_print_timestamps(false);
        p.set_no_timestamps(true);
        p.set_suppress_blank(true);
        p.set_suppress_nst(true);
        let prompt = initial_prompt(vocabulary);
        if !prompt.is_empty() {
            p.set_initial_prompt(&prompt);
        }
        // pad very short clips: whisper is unreliable under ~1 s
        let mut buf = audio.to_vec();
        if buf.len() < 16_000 + 4_000 {
            buf.resize(16_000 + 4_000, 0.0);
        }
        state.full(p, &buf).map_err(|e| format!("Transcription failed: {e}"))?;
        let mut text = String::new();
        for i in 0..state.full_n_segments() {
            if let Some(Ok(s)) = state.get_segment(i).map(|seg| seg.to_str_lossy()) {
                text.push_str(&s);
            }
        }
        Ok(clean_transcript(&text))
    }
}

fn initial_prompt(vocab: &[String]) -> String {
    let words: Vec<&str> = vocab.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).take(60).collect();
    if words.is_empty() {
        String::new()
    } else {
        format!("Vocabulary: {}.", words.join(", "))
    }
}

/// Removes whisper's non-speech tags and its classic silence hallucinations.
pub fn clean_transcript(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    // skip text inside [..], (..) and *..* (sound-effect annotations)
    let (mut brackets, mut stars) = (0u32, false);
    for c in s.chars() {
        match c {
            '*' => stars = !stars,
            '[' | '(' => brackets += 1,
            ']' | ')' if brackets > 0 => brackets -= 1,
            _ if brackets > 0 || stars => {}
            _ => out.push(c),
        }
    }
    let t = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = t.to_lowercase();
    const GHOSTS: [&str; 7] = ["thank you.", "thanks for watching!", "thank you for watching.", "you", "bye.", ".", "thanks for watching."];
    if GHOSTS.contains(&lower.as_str()) {
        return String::new();
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_tags_and_ghosts() {
        assert_eq!(clean_transcript(" [BLANK_AUDIO] "), "");
        assert_eq!(clean_transcript(" Hello (coughs) there  [Music] world"), "Hello there world");
        assert_eq!(clean_transcript(" *laughs* okay"), "okay");
        assert_eq!(clean_transcript(" Thank you."), "");
        assert_eq!(clean_transcript(" Thank you for the update."), "Thank you for the update.");
    }

    #[test]
    fn prompt_from_vocab() {
        assert_eq!(initial_prompt(&[]), "");
        assert_eq!(initial_prompt(&["Kubernetes".into(), " ".into(), "Yatish".into()]), "Vocabulary: Kubernetes, Yatish.");
    }

    #[test]
    fn model_table() {
        assert!(model("base.en").is_some());
        assert!(url(model("tiny.en").unwrap()).ends_with("ggml-tiny.en.bin"));
    }
}
