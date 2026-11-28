//! Speech-to-text. By default dictation is transcribed on the computer by the Whisper model that
//! ships in the installer ([`crate::local_stt`]); it can also go to ChatGPT through Codex's
//! sign-in ([`crate::chatgpt_stt`]). The "Built-in" choice, other languages without ChatGPT, and
//! the wake word use
//! the recognizer built into the operating system: Apple's Speech framework on macOS (on-device
//! when the Mac supports it) and Microsoft's Windows speech recognition on Windows.
//!
//! macOS works on recorded 16 kHz audio. Windows' recognizer only listens to the microphone
//! itself, so a dictation there starts a live session ([`Live`]) next to Murmur's own recording
//! (which still drives the waves and auto-stop).

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Status {
    /// "apple" or "windows" (or "none" on other platforms)
    pub engine: &'static str,
    pub ready: bool,
    /// The user has not been asked yet (macOS permission prompt).
    pub can_request: bool,
    pub on_device: bool,
    pub detail: String,
}

/// BCP-47 tag for a two-letter language code from settings ("auto" → None = system language).
pub fn locale_tag(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "en" => "en-US",
        "es" => "es-ES",
        "fr" => "fr-FR",
        "de" => "de-DE",
        "it" => "it-IT",
        "pt" => "pt-BR",
        "hi" => "hi-IN",
        "te" => "te-IN",
        "ta" => "ta-IN",
        "zh" => "zh-CN",
        "ja" => "ja-JP",
        "ko" => "ko-KR",
        "ar" => "ar-SA",
        "ru" => "ru-RU",
        "nl" => "nl-NL",
        _ => return None,
    })
}

/// Tidies recognizer output: collapses whitespace and drops a lone filler result.
pub fn clean_transcript(s: &str) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if matches!(t.to_lowercase().as_str(), "." | "um" | "uh" | "hmm") {
        return String::new();
    }
    t
}

/// Voice engine chosen in settings, so [`status`] can report the active one.
static ENGINE: parking_lot::Mutex<(String, String)> = parking_lot::Mutex::new((String::new(), String::new()));

/// Remembers the chosen engine (and language, which decides whether the English model fits).
pub fn configure(s: &crate::settings::Settings) {
    *ENGINE.lock() = (s.voice.clone(), s.language.clone());
}

/// The engine a dictation will use: the chosen one when it is set up, otherwise the next best.
/// The bundled model is English-only, so other languages use ChatGPT (if signed in) or the OS.
fn effective(voice: &str, lang: &str) -> &'static str {
    let english = matches!(lang, "en" | "auto" | "");
    let chatgpt = || crate::chatgpt_stt::available();
    match voice {
        "system" => "system",
        "chatgpt" if chatgpt() => "chatgpt",
        _ if english && crate::local_stt::ready() => "local",
        _ if !english && chatgpt() => "chatgpt",
        _ => "system",
    }
}

/// Murmur transcribes its own recording (local model or ChatGPT), so no OS live session is needed.
pub fn uses_own_audio(s: &crate::settings::Settings) -> bool {
    effective(&s.voice, &s.language) != "system"
}

pub fn engine_for(s: &crate::settings::Settings) -> &'static str {
    effective(&s.voice, &s.language)
}

/// Status of the engine dictation will actually use.
pub fn status() -> Status {
    let (voice, lang) = ENGINE.lock().clone();
    match effective(&voice, &lang) {
        "local" => Status { engine: "local", ready: true, can_request: false, on_device: true, detail: crate::local_stt::detail() },
        "chatgpt" => Status { engine: "chatgpt", ready: true, can_request: false, on_device: false, detail: crate::chatgpt_stt::detail() },
        _ => system_status(),
    }
}

/// Transcribes a finished dictation with the active engine. If the local model or ChatGPT fails
/// (ChatGPT only for passing reasons like the network), the recording is retried with the OS
/// recognizer on macOS. Returns the text and the engine that produced it.
pub fn dictate(live: Option<Result<Live, String>>, audio: &[f32], s: &crate::settings::Settings) -> (Result<String, String>, &'static str) {
    let engine = engine_for(s);
    if engine != "system" {
        let res = if engine == "local" {
            crate::local_stt::transcribe(audio, &s.dictionary)
        } else {
            crate::chatgpt_stt::transcribe(audio)
        };
        let retry_os = |e: &str| cfg!(target_os = "macos") && (engine == "local" || crate::vad::is_transient(e)) && system_status().ready;
        match res {
            Ok(t) => {
                if let Some(Ok(l)) = live {
                    l.cancel();
                }
                return (Ok(t), engine);
            }
            Err(e) if retry_os(&e) => {
                crate::report("stt_fallback", serde_json::json!({"engine": engine, "error": e}));
            }
            Err(e) => {
                if let Some(Ok(l)) = live {
                    l.cancel();
                }
                return (Err(e), engine);
            }
        }
    }
    (transcribe(live, audio, &s.language, &s.dictionary), "system")
}

/// Readiness of the OS recognizer, cached once ready: on Windows checking it compiles a grammar
/// (slow), and it is asked on every key press and by the wake-word loop.
pub fn system_status() -> Status {
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};
    static CACHE: OnceLock<parking_lot::Mutex<Option<(Instant, Status)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some((t, st)) = &*cache.lock() {
        if t.elapsed() < Duration::from_secs(120) {
            return st.clone();
        }
    }
    let st = fresh_status();
    *cache.lock() = if st.ready { Some((Instant::now(), st.clone())) } else { None };
    st
}

#[cfg(target_os = "macos")]
pub use mac::*;
#[cfg(windows)]
pub use win::*;
#[cfg(not(any(target_os = "macos", windows)))]
pub use other::*;

// ───────────────────────────── macOS ─────────────────────────────

#[cfg(target_os = "macos")]
mod mac {
    use super::{clean_transcript, locale_tag, Status};
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::AnyThread;
    use objc2_foundation::{NSArray, NSError, NSLocale, NSString, NSURL};
    use objc2_speech::{
        SFSpeechRecognitionResult, SFSpeechRecognizer, SFSpeechRecognizerAuthorizationStatus as Auth,
        SFSpeechURLRecognitionRequest,
    };
    use std::sync::mpsc;
    use std::time::Duration;

    /// macOS needs no live session: it transcribes the finished recording.
    pub struct Live;

    pub fn start_live(_lang: &str, _vocab: &[String]) -> Option<Result<Live, String>> {
        None
    }

    impl Live {
        pub fn cancel(self) {}
    }

    fn auth() -> Auth {
        unsafe { SFSpeechRecognizer::authorizationStatus() }
    }

    pub fn fresh_status() -> Status {
        let a = auth();
        let on_device = recognizer("auto").map(|r| unsafe { r.supportsOnDeviceRecognition() }).unwrap_or(false);
        let (ready, detail) = match a {
            Auth::Authorized => (
                true,
                if on_device { "Apple speech recognition, on this Mac".into() } else { "Apple speech recognition".into() },
            ),
            Auth::NotDetermined => (false, "Allow Speech Recognition so Murmur can hear you".into()),
            Auth::Restricted => (false, "Speech Recognition is restricted on this Mac".into()),
            _ => (false, "Turn on Murmur in System Settings → Privacy & Security → Speech Recognition".into()),
        };
        Status { engine: "apple", ready, can_request: a == Auth::NotDetermined, on_device, detail }
    }

    /// Shows the macOS permission prompt (only the first time) and waits up to a minute for the answer.
    pub fn request_access() -> bool {
        if auth() == Auth::Authorized {
            return true;
        }
        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(move |s: Auth| {
            let _ = tx.send(s);
        });
        unsafe { SFSpeechRecognizer::requestAuthorization(&block) };
        matches!(rx.recv_timeout(Duration::from_secs(60)), Ok(Auth::Authorized))
    }

    fn recognizer(lang: &str) -> Option<Retained<SFSpeechRecognizer>> {
        unsafe {
            if let Some(tag) = locale_tag(lang) {
                let loc = NSLocale::localeWithLocaleIdentifier(&NSString::from_str(tag));
                if let Some(r) = SFSpeechRecognizer::initWithLocale(SFSpeechRecognizer::alloc(), &loc) {
                    return Some(r);
                }
            }
            SFSpeechRecognizer::init(SFSpeechRecognizer::alloc())
        }
    }

    enum Outcome {
        Text(String),
        Failed(isize, String),
    }

    fn run(path: &std::path::Path, lang: &str, vocab: &[String], on_device: bool) -> Result<Outcome, String> {
        let r = recognizer(lang).ok_or("Speech recognition doesn't support this language")?;
        unsafe {
            if !r.isAvailable() {
                return Err("Speech recognition is unavailable right now".into());
            }
            let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
            let req = SFSpeechURLRecognitionRequest::initWithURL(SFSpeechURLRecognitionRequest::alloc(), &url);
            req.setShouldReportPartialResults(false);
            req.setAddsPunctuation(true);
            req.setRequiresOnDeviceRecognition(on_device);
            let words: Vec<Retained<NSString>> =
                vocab.iter().map(|w| w.trim()).filter(|w| !w.is_empty()).take(100).map(NSString::from_str).collect();
            if !words.is_empty() {
                req.setContextualStrings(&NSArray::from_retained_slice(&words));
            }
            let (tx, rx) = mpsc::channel::<Outcome>();
            let handler = RcBlock::new(move |res: *mut SFSpeechRecognitionResult, err: *mut NSError| {
                if let Some(e) = err.as_ref() {
                    let _ = tx.send(Outcome::Failed(e.code(), e.localizedDescription().to_string()));
                } else if let Some(res) = res.as_ref() {
                    if res.isFinal() {
                        let _ = tx.send(Outcome::Text(res.bestTranscription().formattedString().to_string()));
                    }
                }
            });
            let task = r.recognitionTaskWithRequest_resultHandler(&req, &handler);
            let out = rx.recv_timeout(Duration::from_secs(45));
            if out.is_err() {
                task.cancel();
            }
            out.map_err(|_| "Speech recognition timed out".to_string())
        }
    }

    fn write_wav(audio: &[f32]) -> Result<tempfile::TempPath, String> {
        let f = tempfile::Builder::new().prefix("murmur-").suffix(".wav").tempfile().map_err(|e| e.to_string())?;
        let spec = hound::WavSpec { channels: 1, sample_rate: 16_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut w = hound::WavWriter::new(std::io::BufWriter::new(f.as_file()), spec).map_err(|e| e.to_string())?;
        // short clips are recognized more reliably with a little silence around them
        let pad = [0i16; 4_000];
        // Quiet mics (low input volume) leave Apple's recognizer hearing nothing: bring the peak
        // up to a normal level, at most 30x.
        let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let gain = if peak > 0.0 { (0.8 / peak).clamp(1.0, 30.0) } else { 1.0 };
        for s in pad.iter().copied().chain(audio.iter().map(|v| ((v * gain).clamp(-1.0, 1.0) * 32767.0) as i16)).chain(pad.iter().copied()) {
            w.write_sample(s).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
        Ok(f.into_temp_path())
    }

    /// Error codes that mean "nothing was said" rather than a failure.
    const NO_SPEECH: [isize; 2] = [1110, 203];

    /// Transcribes a finished dictation. Uses on-device recognition when the Mac has it, and
    /// falls back to Apple's servers only if the on-device recognizer fails.
    pub fn transcribe(_live: Option<Result<Live, String>>, audio: &[f32], lang: &str, vocab: &[String]) -> Result<String, String> {
        if auth() != Auth::Authorized {
            return Err("Allow Speech Recognition for Murmur in System Settings → Privacy & Security.".into());
        }
        let path = write_wav(audio)?;
        let local = recognizer(lang).map(|r| unsafe { r.supportsOnDeviceRecognition() }).unwrap_or(false);
        let mut out = run(&path, lang, vocab, local)?;
        let note = |o: &Outcome, on_device: bool| {
            let (kind, code, len) = match o {
                Outcome::Text(t) => ("text", 0, t.len()),
                Outcome::Failed(c, m) => ("failed", *c, m.len()),
            };
            let msg = if let Outcome::Failed(_, m) = o { m.clone() } else { String::new() };
            crate::report("stt_detail", serde_json::json!({"on_device": on_device, "kind": kind, "code": code, "len": len, "error": msg}));
        };
        note(&out, local);
        // on-device heard nothing or failed: one more try with Apple's online recognizer
        let empty = match &out {
            Outcome::Text(t) => t.trim().is_empty(),
            Outcome::Failed(..) => true,
        };
        if local && empty {
            out = run(&path, lang, vocab, false)?;
            note(&out, false);
        }
        match out {
            Outcome::Text(t) => Ok(clean_transcript(&t)),
            Outcome::Failed(code, _) if NO_SPEECH.contains(&code) => Ok(String::new()),
            Outcome::Failed(_, msg) => Err(format!("Speech recognition failed: {msg}")),
        }
    }

    /// Wake-word check on a short clip. Strictly on-device: background audio is never uploaded,
    /// so on a Mac without on-device recognition the wake word is unavailable.
    pub fn transcribe_private(audio: &[f32], lang: &str, vocab: &[String]) -> Result<String, String> {
        if auth() != Auth::Authorized {
            return Err("Speech Recognition not allowed".into());
        }
        if !recognizer(lang).map(|r| unsafe { r.supportsOnDeviceRecognition() }).unwrap_or(false) {
            return Err("This Mac can't recognize speech on-device, so the wake word is off".into());
        }
        let path = write_wav(audio)?;
        match run(&path, lang, vocab, true)? {
            Outcome::Text(t) => Ok(clean_transcript(&t)),
            Outcome::Failed(..) => Ok(String::new()),
        }
    }
}

// ───────────────────────────── Windows ─────────────────────────────

#[cfg(windows)]
mod win {
    use super::{clean_transcript, locale_tag, Status};
    use parking_lot::Mutex;
    use std::sync::{mpsc, Arc};
    use std::time::Duration;
    use windows::core::{Interface, HSTRING};
    use windows::Foundation::{TimeSpan, TypedEventHandler};
    use windows::Globalization::Language;
    use windows::Media::SpeechRecognition::{
        ISpeechRecognitionConstraint, SpeechContinuousRecognitionCompletedEventArgs, SpeechContinuousRecognitionResultGeneratedEventArgs,
        SpeechContinuousRecognitionSession, SpeechRecognitionConfidence, SpeechRecognitionListConstraint, SpeechRecognitionResultStatus,
        SpeechRecognitionScenario, SpeechRecognitionTopicConstraint, SpeechRecognizer,
    };

    /// HRESULT when "Online speech recognition" is off in Windows Settings.
    const PRIVACY_NOT_ACCEPTED: i32 = 0x8004_5509_u32 as i32;
    pub const PRIVACY_HINT: &str = "Turn on Online speech recognition in Windows Settings → Privacy & security → Speech";

    fn init_thread() {
        unsafe {
            let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
        }
    }

    fn recognizer(lang: &str) -> windows::core::Result<SpeechRecognizer> {
        if let Some(tag) = locale_tag(lang) {
            if let Ok(l) = Language::CreateLanguage(&HSTRING::from(tag)) {
                if let Ok(r) = SpeechRecognizer::Create(&l) {
                    return Ok(r);
                }
            }
        }
        SpeechRecognizer::new()
    }

    fn friendly(e: &windows::core::Error) -> String {
        if e.code().0 == PRIVACY_NOT_ACCEPTED {
            PRIVACY_HINT.into()
        } else {
            format!("Windows speech recognition failed: {}", e.message())
        }
    }

    pub fn fresh_status() -> Status {
        // Settings › Privacy & security › Speech › "Online speech recognition"
        let accepted = crate::perms::win_reg::read_dword(r"Software\Microsoft\Speech_OneCore\Settings\OnlineSpeechPrivacy", "HasAccepted");
        if accepted == Some(0) {
            return Status { engine: "windows", ready: false, can_request: false, on_device: false, detail: format!("{PRIVACY_HINT}.") };
        }
        init_thread();
        let probe = (|| -> windows::core::Result<SpeechRecognitionResultStatus> {
            let r = recognizer("auto")?;
            let c = SpeechRecognitionTopicConstraint::Create(SpeechRecognitionScenario::Dictation, &HSTRING::from("dictation"))?;
            r.Constraints()?.Append(&c.cast::<ISpeechRecognitionConstraint>()?)?;
            r.CompileConstraintsAsync()?.join()?.Status()
        })();
        let (ready, detail) = match probe {
            Ok(SpeechRecognitionResultStatus::Success) => (true, "Windows speech recognition".to_string()),
            Ok(s) => (false, format!("Windows speech isn't ready ({s:?}). {PRIVACY_HINT}.")),
            Err(e) => (false, friendly(&e)),
        };
        Status { engine: "windows", ready, can_request: false, on_device: false, detail }
    }

    /// Windows has no permission prompt: open the Speech privacy page instead.
    pub fn request_access() -> bool {
        crate::paste::open_privacy_pane("speech");
        false
    }

    /// A dictation session listening to the microphone alongside Murmur's own recording.
    pub struct Live {
        session: SpeechContinuousRecognitionSession,
        _recognizer: SpeechRecognizer,
        texts: Arc<Mutex<Vec<String>>>,
        error: Arc<Mutex<Option<String>>>,
        done: mpsc::Receiver<()>,
    }

    fn start(lang: &str) -> windows::core::Result<Live> {
        init_thread();
        let r = recognizer(lang)?;
        let c = SpeechRecognitionTopicConstraint::Create(SpeechRecognitionScenario::Dictation, &HSTRING::from("dictation"))?;
        r.Constraints()?.Append(&c.cast::<ISpeechRecognitionConstraint>()?)?;
        let compiled = r.CompileConstraintsAsync()?.join()?;
        if compiled.Status()? != SpeechRecognitionResultStatus::Success {
            return Err(windows::core::Error::new(windows::core::HRESULT(PRIVACY_NOT_ACCEPTED), "constraints"));
        }
        let session = r.ContinuousRecognitionSession()?;
        // never stop on our own: Murmur decides when the dictation ends
        session.SetAutoStopSilenceTimeout(TimeSpan { Duration: 10 * 60 * 10_000_000 })?;
        let texts = Arc::new(Mutex::new(Vec::new()));
        let error = Arc::new(Mutex::new(None));
        let t2 = texts.clone();
        session.ResultGenerated(&TypedEventHandler::<SpeechContinuousRecognitionSession, SpeechContinuousRecognitionResultGeneratedEventArgs>::new(
            move |_, args| {
                if let Ok(a) = args.ok() {
                    let res = a.Result()?;
                    if res.Status()? == SpeechRecognitionResultStatus::Success {
                        t2.lock().push(res.Text()?.to_string());
                    }
                }
                Ok(())
            },
        ))?;
        let (tx, done) = mpsc::channel();
        let e2 = error.clone();
        session.Completed(&TypedEventHandler::<SpeechContinuousRecognitionSession, SpeechContinuousRecognitionCompletedEventArgs>::new(
            move |_, args| {
                if let Ok(a) = args.ok() {
                    let st = a.Status()?;
                    if st != SpeechRecognitionResultStatus::Success && st != SpeechRecognitionResultStatus::UserCanceled {
                        *e2.lock() = Some(format!("Windows speech recognition stopped ({st:?})"));
                    }
                }
                let _ = tx.send(());
                Ok(())
            },
        ))?;
        session.StartAsync()?.join()?;
        Ok(Live { session, _recognizer: r, texts, error, done })
    }

    /// Starts listening (the Windows recognizer uses the microphone itself). Errors are kept
    /// and reported when the dictation finishes.
    pub fn start_live(lang: &str, _vocab: &[String]) -> Option<Result<Live, String>> {
        Some(start(lang).map_err(|e| friendly(&e)))
    }

    impl Live {
        pub fn cancel(self) {
            let _ = self.session.CancelAsync().and_then(|a| a.join());
        }

        fn finish(self) -> Result<String, String> {
            // StopAsync lets the recognizer finalize what it has already heard
            if let Err(e) = self.session.StopAsync().and_then(|a| a.join()) {
                return Err(friendly(&e));
            }
            let _ = self.done.recv_timeout(Duration::from_secs(4));
            let text = self.texts.lock().join(" ");
            if text.trim().is_empty() {
                if let Some(e) = self.error.lock().take() {
                    return Err(e);
                }
            }
            Ok(clean_transcript(&text))
        }
    }

    pub fn transcribe(live: Option<Result<Live, String>>, _audio: &[f32], _lang: &str, _vocab: &[String]) -> Result<String, String> {
        match live {
            Some(Ok(l)) => l.finish(),
            Some(Err(e)) => Err(e),
            None => Err("Windows speech recognition didn't start".into()),
        }
    }

    pub fn transcribe_private(_audio: &[f32], _lang: &str, _vocab: &[String]) -> Result<String, String> {
        Err("not used on Windows".into())
    }

    /// Wake word: Windows' offline phrase recognizer listens for just the wake phrase (nothing
    /// is uploaded). Blocks until `stop()` returns true; calls `on_hit` each time it hears it.
    pub fn listen_for_phrase(phrase: &str, lang: &str, stop: impl Fn() -> bool, on_hit: impl Fn() + Send + 'static) -> Result<(), String> {
        init_thread();
        let run = || -> windows::core::Result<()> {
            let r = recognizer(lang)?;
            let words: Vec<HSTRING> = vec![HSTRING::from(phrase)];
            let list = SpeechRecognitionListConstraint::Create(&windows_collections::IIterable::<HSTRING>::from(words))?;
            r.Constraints()?.Append(&list.cast::<ISpeechRecognitionConstraint>()?)?;
            let compiled = r.CompileConstraintsAsync()?.join()?;
            if compiled.Status()? != SpeechRecognitionResultStatus::Success {
                return Err(windows::core::Error::new(windows::core::HRESULT(-1), "wake phrase"));
            }
            let session = r.ContinuousRecognitionSession()?;
            session.SetAutoStopSilenceTimeout(TimeSpan { Duration: 24 * 3600 * 10_000_000 })?;
            session.ResultGenerated(&TypedEventHandler::<SpeechContinuousRecognitionSession, SpeechContinuousRecognitionResultGeneratedEventArgs>::new(
                move |_, args| {
                    if let Ok(a) = args.ok() {
                        let res = a.Result()?;
                        let conf = res.Confidence()?;
                        if res.Status()? == SpeechRecognitionResultStatus::Success
                            && (conf == SpeechRecognitionConfidence::High || conf == SpeechRecognitionConfidence::Medium)
                        {
                            on_hit();
                        }
                    }
                    Ok(())
                },
            ))?;
            session.StartAsync()?.join()?;
            while !stop() {
                std::thread::sleep(Duration::from_millis(300));
            }
            let _ = session.CancelAsync().and_then(|a| a.join());
            Ok(())
        };
        run().map_err(|e| friendly(&e))
    }
}

// ───────────────────────────── other ─────────────────────────────

#[cfg(not(any(target_os = "macos", windows)))]
mod other {
    use super::Status;

    pub struct Live;
    impl Live {
        pub fn cancel(self) {}
    }
    pub fn start_live(_lang: &str, _vocab: &[String]) -> Option<Result<Live, String>> {
        None
    }
    pub fn fresh_status() -> Status {
        Status { engine: "none", ready: false, can_request: false, on_device: false, detail: "Speech recognition is available on macOS and Windows".into() }
    }
    pub fn request_access() -> bool {
        false
    }
    pub fn transcribe(_live: Option<Result<Live, String>>, _audio: &[f32], _lang: &str, _vocab: &[String]) -> Result<String, String> {
        Err("Speech recognition is available on macOS and Windows".into())
    }
    pub fn transcribe_private(_audio: &[f32], _lang: &str, _vocab: &[String]) -> Result<String, String> {
        Err("unavailable".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans() {
        assert_eq!(clean_transcript("  hello   there "), "hello there");
        assert_eq!(clean_transcript("Um"), "");
        assert_eq!(clean_transcript("Thank you for the update."), "Thank you for the update.");
    }

    #[test]
    fn locales() {
        assert_eq!(locale_tag("en"), Some("en-US"));
        assert_eq!(locale_tag("auto"), None);
    }
}
