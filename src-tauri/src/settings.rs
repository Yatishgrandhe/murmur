//! Persistent settings + dictation history (JSON in the app config dir). API keys live in the OS keychain.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Global shortcut, Tauri accelerator syntax ("Alt+Space").
    pub shortcut: String,
    /// "claude" | "codex" | "anthropic" | "openai" | "off"
    pub provider: String,
    /// "clean" | "polished" | "casual" | "email" | "custom"
    pub style: String,
    pub custom_prompt: String,
    /// Whisper model id (see stt::MODELS).
    pub model: String,
    /// "auto" or an ISO code like "en".
    pub language: String,
    /// Claude Code model alias used with a Claude plan ("haiku" is fastest).
    pub claude_model: String,
    /// Optional Codex model override ("" = the user's Codex default).
    pub codex_model: String,
    pub anthropic_model: String,
    pub openai_model: String,
    /// Words / names the speech model and the rewriter must spell exactly.
    pub dictionary: Vec<String>,
    pub restore_clipboard: bool,
    pub sounds: bool,
    pub onboarded: bool,
    /// Hands-free start by voice ("Hey Murmur"). Keeps the mic open while on.
    pub wake_word: bool,
    pub wake_phrase: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            shortcut: default_shortcut().into(),
            provider: "claude".into(),
            style: "clean".into(),
            custom_prompt: String::new(),
            model: "base.en".into(),
            language: "en".into(),
            claude_model: "haiku".into(),
            codex_model: String::new(),
            anthropic_model: "claude-haiku-5-5".into(),
            openai_model: "gpt-5-mini".into(),
            dictionary: vec![],
            restore_clipboard: true,
            sounds: true,
            onboarded: false,
            wake_word: false,
            wake_phrase: "Hey Murmur".into(),
        }
    }
}

pub fn default_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "Alt+Space"
    } else {
        "Ctrl+Shift+Space"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryItem {
    pub ts: u64,
    pub raw: String,
    pub text: String,
    pub provider: String,
    pub seconds: f32,
    pub ms: u64,
}

pub fn load<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn save<T: Serialize>(path: &Path, v: &T) {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(b) = serde_json::to_vec_pretty(v) {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, b).is_ok() {
            let _ = std::fs::rename(tmp, path);
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("Murmur")
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("Murmur")
}

const KEYRING_SERVICE: &str = "app.murmur.dictation";

pub fn get_secret(name: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, name).ok()?.get_password().ok().filter(|s| !s.is_empty())
}

pub fn set_secret(name: &str, value: &str) -> Result<(), String> {
    let e = keyring::Entry::new(KEYRING_SERVICE, name).map_err(|e| e.to_string())?;
    if value.is_empty() {
        let _ = e.delete_credential();
        Ok(())
    } else {
        e.set_password(value).map_err(|e| e.to_string())
    }
}
