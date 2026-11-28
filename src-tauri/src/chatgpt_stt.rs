//! Free speech-to-text with a ChatGPT plan, through the official Codex app's sign-in.
//!
//! Codex's own backend client posts dictation clips to `https://chatgpt.com/backend-api/transcribe`
//! (multipart `file`, the ChatGPT access token and `chatgpt-account-id` header; the reply is
//! `{"text": ...}`). Murmur sends the same request with the login Codex already saved, so
//! transcription is covered by the user's ChatGPT plan: no API key, no per-minute charge.
//!
//! This is the endpoint Codex uses, not a published API, so it can change. Murmur never refreshes
//! or rewrites Codex's tokens itself: when the access token is old it runs Codex once (a one-word
//! `codex exec`), which refreshes the login the normal way, and then reads the file again.
//!
//! There is no Claude equivalent Murmur may use: Anthropic has no speech-to-text API and does not
//! allow third-party apps to use Claude.ai logins.

use crate::vad::{is_phantom, trim_silence, wav_bytes};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ENDPOINT: &str = "https://chatgpt.com/backend-api/transcribe";

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

fn auth_path() -> PathBuf {
    let home = std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| dirs::home_dir().map(|h| h.join(".codex")));
    home.unwrap_or_default().join("auth.json")
}

/// The ChatGPT login Codex saved.
struct Login {
    token: String,
    account: Option<String>,
    /// Access-token expiry (unix seconds), if it could be read.
    exp: Option<u64>,
}

/// Why there is no usable login, in words for the user.
fn read_login() -> Result<Login, String> {
    let raw = std::fs::read(auth_path()).map_err(|_| {
        if crate::cli::find("codex").is_none() {
            "Install Codex and sign in with ChatGPT (Murmur › AI) to use ChatGPT speech recognition.".to_string()
        } else {
            "Sign in to Codex with ChatGPT (Murmur › AI) to use ChatGPT speech recognition. If you're signed in, Codex keeps its login in the keychain: set cli_auth_credentials_store = \"file\" in ~/.codex/config.toml.".to_string()
        }
    })?;
    let v: serde_json::Value = serde_json::from_slice(&raw).map_err(|_| "Codex's login file couldn't be read. Sign in to Codex again.".to_string())?;
    let t = &v["tokens"];
    let token = t["access_token"].as_str().filter(|s| !s.is_empty()).ok_or("Codex is signed in with an API key, not ChatGPT. Sign in with ChatGPT (codex login) to use your plan for speech.")?;
    Ok(Login { token: token.to_string(), account: t["account_id"].as_str().map(str::to_string), exp: jwt_exp(token) })
}

/// Whether Codex has a ChatGPT login Murmur can use.
pub fn available() -> bool {
    read_login().is_ok()
}

pub fn detail() -> String {
    match read_login() {
        Ok(_) => "ChatGPT plan, through Codex's sign-in".into(),
        Err(e) => e,
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn b64url(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            b'=' => break,
            _ => return None,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// `exp` claim of a JWT (not verified: only used to decide when to let Codex refresh).
fn jwt_exp(jwt: &str) -> Option<u64> {
    let payload = b64url(jwt.split('.').nth(1)?)?;
    serde_json::from_slice::<serde_json::Value>(&payload).ok()?["exp"].as_u64()
}

fn stale(l: &Login, margin: u64) -> bool {
    l.exp.is_some_and(|e| e <= now() + margin)
}

/// Runs Codex once so it refreshes its own ChatGPT login (Codex refreshes on use). Takes a few
/// seconds; runs at most once at a time.
pub fn refresh_via_codex() -> bool {
    static BUSY: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
    let _g = BUSY.lock();
    if read_login().is_ok_and(|l| !stale(&l, 3600)) {
        return true; // another call already refreshed it
    }
    let Some(bin) = crate::cli::find_prefer_exe("codex") else { return false };
    let mut c = crate::cli::command(&bin);
    c.args(["exec", "--skip-git-repo-check", "--ephemeral", "--sandbox", "read-only", "--color", "never", "Reply with just: ok"]);
    let ok = crate::cli::run_within(&mut c, Duration::from_secs(60)).is_ok();
    crate::report("chatgpt_refresh", serde_json::json!({"ok": ok}));
    ok && read_login().is_ok_and(|l| !stale(&l, 60))
}

/// Refreshes in the background when the login expires within a day, so a dictation never waits.
pub fn keep_fresh() {
    if read_login().is_ok_and(|l| stale(&l, 24 * 3600)) {
        std::thread::spawn(refresh_via_codex);
    }
}

/// Opens the connection while the user speaks (and refreshes an old login).
pub fn warm_up() {
    keep_fresh();
    std::thread::spawn(|| {
        let _ = client().head("https://chatgpt.com/backend-api/").timeout(Duration::from_secs(5)).send();
    });
}

fn post(wav: &[u8], l: &Login) -> Result<(u16, serde_json::Value), String> {
    use reqwest::blocking::multipart::{Form, Part};
    let part = Part::bytes(wav.to_vec()).file_name("audio.wav").mime_str("audio/wav").map_err(|e| e.to_string())?;
    let mut req = client().post(ENDPOINT).bearer_auth(&l.token).header("User-Agent", "codex-cli").multipart(Form::new().part("file", part));
    if let Some(a) = &l.account {
        req = req.header("chatgpt-account-id", a);
    }
    let resp = req.send().map_err(|e| {
        if e.is_timeout() {
            "Network error: ChatGPT speech recognition timed out".to_string()
        } else {
            format!("Network error: {e}")
        }
    })?;
    let code = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    Ok((code, serde_json::from_str(&body).unwrap_or(serde_json::Value::Null)))
}

fn friendly(code: u16, v: &serde_json::Value) -> String {
    let detail = v["detail"].as_str().or_else(|| v["error"]["message"].as_str()).or_else(|| v["detail"]["message"].as_str()).unwrap_or("");
    match code {
        401 => "Your ChatGPT sign-in expired. Sign in to Codex again in Murmur › AI.".into(),
        403 => format!("ChatGPT refused the request{}. Your plan may not include it.", if detail.is_empty() { String::new() } else { format!(" ({detail})") }),
        404 | 405 | 410 => "ChatGPT's transcription service moved. Switch Voice to Built-in or OpenAI API until Murmur is updated.".into(),
        429 => "ChatGPT's speech limit was reached. Try again in a bit.".into(),
        500..=599 => "ChatGPT is busy. Try again in a moment.".into(),
        _ => format!("ChatGPT speech error {code}{}", if detail.is_empty() { String::new() } else { format!(": {detail}") }),
    }
}

/// Transcribes a finished 16 kHz recording with the ChatGPT plan. Returns "" when nothing was said.
pub fn transcribe(audio: &[f32]) -> Result<String, String> {
    let Some(t) = trim_silence(audio) else { return Ok(String::new()) };
    let wav = wav_bytes(&t.audio);
    let mut login = read_login()?;
    if stale(&login, 60) {
        refresh_via_codex();
        login = read_login()?;
    }
    let (mut code, mut v) = post(&wav, &login)?;
    if code == 401 && refresh_via_codex() {
        login = read_login()?;
        (code, v) = post(&wav, &login)?;
    }
    crate::report("chatgpt_stt", serde_json::json!({"status": code}));
    if !(200..300).contains(&code) {
        return Err(friendly(code, &v));
    }
    let text = v["text"].as_str().unwrap_or("").trim().to_string();
    if is_phantom(&text, t.voiced_ms) {
        return Ok(String::new());
    }
    Ok(crate::stt::clean_transcript(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_jwt_exp() {
        // header.{"exp":1700000000,"sub":"x"}.sig
        let jwt = "eyJhbGciOiJub25lIn0.eyJleHAiOjE3MDAwMDAwMDAsInN1YiI6IngifQ.c2ln";
        assert_eq!(jwt_exp(jwt), Some(1_700_000_000));
        assert_eq!(jwt_exp("not-a-jwt"), None);
    }

    #[test]
    fn base64url() {
        assert_eq!(b64url("aGVsbG8").unwrap(), b"hello");
        assert_eq!(b64url("aGk_").unwrap(), [0x68, 0x69, 0x3f]);
    }

    #[test]
    fn staleness() {
        let l = Login { token: String::new(), account: None, exp: Some(now() + 100) };
        assert!(stale(&l, 3600));
        assert!(!stale(&l, 10));
        assert!(!stale(&Login { token: String::new(), account: None, exp: None }, 3600));
    }

    #[test]
    fn friendly_errors() {
        assert!(friendly(401, &serde_json::Value::Null).contains("Sign in"));
        assert!(friendly(429, &serde_json::Value::Null).contains("limit"));
        assert!(friendly(404, &serde_json::Value::Null).contains("Built-in"));
    }
}
