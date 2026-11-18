//! Murmur: hold a shortcut, speak, and polished text appears wherever you're typing.
//!
//! Flow: shortcut down → mic + AI helper warm up → shortcut up (or tap again in hands-free
//! mode) → Whisper transcribes on-device → Claude / ChatGPT rewrites → pasted into the
//! focused app. Esc cancels.

pub mod audio;
pub mod cli;
pub mod llm;
pub mod paste;
pub mod settings;
pub mod stt;
pub mod wake;

use futures_util::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use settings::{HistoryItem, Settings};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Idle,
    Holding,
    HandsFree,
    Processing,
}

struct Session {
    rec: audio::Recording,
    pending: Result<llm::Pending, String>,
    started: Instant,
}

pub struct Core {
    settings: Mutex<Settings>,
    mode: Mutex<Mode>,
    session: Mutex<Option<Session>>,
    stt: stt::Stt,
    history: Mutex<Vec<HistoryItem>>,
    registered: Mutex<Option<String>>,
    vibrancy: Mutex<bool>,
    /// Self-test: recorded audio used instead of the microphone.
    test_audio: Mutex<Option<(Vec<f32>, u32)>>,
}

type CoreRef<'a> = State<'a, Arc<Core>>;

fn settings_path() -> PathBuf {
    settings::config_dir().join("settings.json")
}
fn history_path() -> PathBuf {
    settings::data_dir().join("history.json")
}
fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[derive(Serialize, Clone)]
struct Phase {
    phase: &'static str,
    text: Option<String>,
    detail: Option<String>,
    hands_free: bool,
}

fn emit_phase(app: &AppHandle, phase: &'static str, text: Option<String>, detail: Option<String>, hands_free: bool) {
    let _ = app.emit("phase", Phase { phase, text, detail, hands_free });
}

// ───────────────────────────── dictation state machine ─────────────────────────────

fn core(app: &AppHandle) -> Arc<Core> {
    app.state::<Arc<Core>>().inner().clone()
}

fn on_press(app: &AppHandle) {
    let c = core(app);
    let mut m = c.mode.lock();
    match *m {
        Mode::Idle => {
            *m = Mode::Holding;
            drop(m);
            begin(app, &c, false);
        }
        Mode::HandsFree => {
            drop(m);
            finish(app, &c);
        }
        _ => {}
    }
}

fn on_release(app: &AppHandle) {
    let c = core(app);
    let mut m = c.mode.lock();
    if *m != Mode::Holding {
        return;
    }
    let held = c.session.lock().as_ref().map(|s| s.started.elapsed()).unwrap_or_default();
    if held < Duration::from_millis(300) {
        // a quick tap = hands-free: keep listening until the shortcut is pressed again
        *m = Mode::HandsFree;
        drop(m);
        emit_phase(app, "listening", None, None, true);
    } else {
        drop(m);
        finish(app, &c);
    }
}

fn begin(app: &AppHandle, c: &Arc<Core>, hands_free: bool) {
    let s = c.settings.lock().clone();
    if !stt::installed(&s.model) {
        *c.mode.lock() = Mode::Idle;
        emit_phase(app, "error", None, Some("Download a voice model to start dictating.".into()), false);
        show_main(app, Some("voice"));
        return;
    }
    report("press", serde_json::json!({"provider": s.provider, "model": s.model, "accessibility": paste::accessibility_ok()}));
    position_overlay(app);
    if s.sounds {
        audio::blip(audio::Blip::Start);
    }
    let (a2, c2) = (app.clone(), c.clone());
    let vad = Arc::new(Mutex::new((audio::EndOfSpeech::new(), Instant::now())));
    let on_level = move |level: f32| {
        let _ = a2.emit_to("overlay", "level", level);
        // Hands-free: stop by itself once you've spoken and then paused.
        if *c2.mode.lock() != Mode::HandsFree {
            return;
        }
        let mut v = vad.lock();
        let dt = v.1.elapsed().as_secs_f32() * 1000.0;
        v.1 = Instant::now();
        let trigger = v.0.push(level, dt);
        if let Some(spoke) = trigger {
            let (a3, c3) = (a2.clone(), c2.clone());
            std::thread::spawn(move || {
                report("auto_stop", serde_json::json!({"spoke": spoke}));
                if spoke {
                    finish(&a3, &c3)
                } else {
                    cancel_with(&a3, "No speech heard. Check Murmur's microphone permission.")
                }
            });
        }
    };
    let rec = match c.test_audio.lock().clone() {
        Some((samples, rate)) => audio::Recording::replay(samples, rate, on_level),
        None => audio::Recording::start(on_level),
    };
    emit_phase(app, "listening", None, None, hands_free);
    let _ = app.global_shortcut().register("Escape");
    // Warm up the rewriter now so its startup overlaps with speaking.
    let pending = llm::Pending::start(&s);
    *c.session.lock() = Some(Session { rec, pending, started: Instant::now() });
    let c2 = c.clone();
    std::thread::spawn(move || {
        let _ = c2.stt.ensure_loaded(&s.model);
    });
}

fn finish(app: &AppHandle, c: &Arc<Core>) {
    report("release", serde_json::json!(null));
    {
        let mut m = c.mode.lock();
        if !matches!(*m, Mode::Holding | Mode::HandsFree) {
            return;
        }
        *m = Mode::Processing;
    }
    let _ = app.global_shortcut().unregister("Escape");
    let Some(sess) = c.session.lock().take() else {
        *c.mode.lock() = Mode::Idle;
        return;
    };
    let (app, c) = (app.clone(), c.clone());
    std::thread::spawn(move || {
        pipeline(&app, &c, sess);
        *c.mode.lock() = Mode::Idle;
    });
}

fn cancel(app: &AppHandle) {
    cancel_with(app, "Cancelled");
}

fn cancel_with(app: &AppHandle, msg: &str) {
    let c = core(app);
    {
        let mut m = c.mode.lock();
        if !matches!(*m, Mode::Holding | Mode::HandsFree) {
            return;
        }
        *m = Mode::Idle;
    }
    let _ = app.global_shortcut().unregister("Escape");
    report("cancel", serde_json::json!(null));
    if let Some(sess) = c.session.lock().take() {
        drop(sess.rec);
        if let Ok(p) = sess.pending {
            std::thread::spawn(move || p.cancel());
        }
    }
    emit_phase(app, "idle", None, Some(msg.into()), false);
}

fn pipeline(app: &AppHandle, c: &Arc<Core>, sess: Session) {
    let s = c.settings.lock().clone();
    if s.sounds {
        audio::blip(audio::Blip::Stop);
    }
    let t0 = Instant::now();
    let Session { rec, pending, .. } = sess;
    let fail = |msg: String, pending: Result<llm::Pending, String>| {
        report("error", serde_json::json!(msg));
        if let Ok(p) = pending {
            p.cancel();
        }
        if s.sounds {
            audio::blip(audio::Blip::Error);
        }
        emit_phase(app, "error", None, Some(msg), false);
    };

    let audio = match rec.finish() {
        Ok(a) => a,
        Err(e) => return fail(e, pending),
    };
    let seconds = audio.len() as f32 / 16_000.0;
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    report("audio", serde_json::json!({"seconds": seconds, "peak": peak, "speech": audio::has_speech(&audio), "device": *audio::LAST_DEVICE.lock()}));
    if !audio::has_speech(&audio) {
        if let Ok(p) = pending {
            p.cancel();
        }
        emit_phase(app, "idle", None, Some("Didn't catch that".into()), false);
        return;
    }
    emit_phase(app, "transcribing", None, None, false);
    let raw = match c.stt.transcribe(&s.model, &audio, &s.language, &s.dictionary) {
        Ok(t) => llm::apply_voice_commands(&t),
        Err(e) => return fail(e, pending),
    };
    if raw.is_empty() {
        if let Ok(p) = pending {
            p.cancel();
        }
        emit_phase(app, "idle", None, Some("Didn't catch that".into()), false);
        return;
    }

    deliver(app, c, &s, raw, pending, seconds, t0);
}

/// Rewrites `raw` (if an AI is set up), types/copies the result, and records it in history.
fn deliver(app: &AppHandle, c: &Arc<Core>, s: &Settings, raw: String, pending: Result<llm::Pending, String>, seconds: f32, t0: Instant) {
    let mut warning = None;
    let mut used = s.provider.clone();
    let text = match pending {
        Ok(p) if p.is_off() => {
            used = "off".into();
            raw.clone()
        }
        Ok(p) => match llm::trivial_for(&raw, &s.style) {
            Some(t) => {
                p.cancel();
                t
            }
            None => {
                emit_phase(app, "rewriting", Some(raw.clone()), Some(if s.style == "prompt" { format!("prompt:{}", llm::provider_label(&s.provider)) } else { llm::provider_label(&s.provider).into() }), false);
                p.finish(&raw).unwrap_or_else(|e| {
                    warning = Some(e);
                    used = "off".into();
                    raw.clone()
                })
            }
        },
        Err(e) => {
            warning = Some(e);
            used = "off".into();
            raw.clone()
        }
    };

    let mut copied = false;
    let selftest_no_paste = std::env::var_os("MURMUR_SELFTEST").is_some() && std::env::var_os("MURMUR_SELFTEST_PASTE").is_none();
    if selftest_no_paste {
        // self-test: don't type into whatever app happens to be focused
    } else if paste::accessibility_ok() {
        if let Err(e) = paste::paste(&text, s.restore_clipboard) {
            let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.clone()));
            warning = Some(format!("Couldn't paste ({e}). It's on your clipboard."));
        }
    } else {
        // no paste permission: hand the rephrased text over via the clipboard instead
        let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.clone()));
        copied = true;
    }
    report("result", serde_json::json!({"raw": raw, "text": text, "ms": t0.elapsed().as_millis() as u64, "warning": warning, "provider": used, "copied": copied}));
    let detail = if copied && warning.is_none() { Some("copied".to_string()) } else { warning };
    emit_phase(app, "done", Some(text.clone()), detail, false);

    let item = HistoryItem { ts: now_ms(), raw, text, provider: used, seconds, ms: t0.elapsed().as_millis() as u64 };
    let mut h = c.history.lock();
    h.insert(0, item);
    h.truncate(500);
    settings::save(&history_path(), &*h);
    drop(h);
    let _ = app.emit("history-changed", ());
}

// ───────────────────────────── wake word ─────────────────────────────

/// Runs for the life of the app. While the wake word is on (and a voice model is installed),
/// keeps a mic stream open, cuts it into short utterances and checks each one on-device.
fn wake_loop(app: AppHandle, c: Arc<Core>) {
    let mut announced = false;
    loop {
        let s = c.settings.lock().clone();
        if !s.wake_word || !stt::installed(&s.model) {
            announced = false;
            std::thread::sleep(Duration::from_millis(700));
            continue;
        }
        if !announced {
            report("wake", serde_json::json!({"listening": true, "phrase": s.wake_phrase}));
            announced = true;
        }
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut seg = wake::Segmenter::new();
        let (app2, c2, stop2) = (app.clone(), c.clone(), stop.clone());
        let res = audio::stream_mic(stop.clone(), move |chunk| {
            let s = c2.settings.lock().clone();
            if !s.wake_word {
                stop2.store(true, std::sync::atomic::Ordering::SeqCst);
                return;
            }
            if *c2.mode.lock() != Mode::Idle {
                seg.reset(); // a dictation is running; don't listen to it
                return;
            }
            for clip in seg.push(chunk) {
                // only the wake check runs here: nothing is logged or sent unless it matches
                let Ok(text) = c2.stt.transcribe(&s.model, &clip, &s.language, std::slice::from_ref(&s.wake_phrase)) else { continue };
                let hit = wake::match_wake(&text, &s.wake_phrase);
                // diagnostics without content: clip length and whether it matched, never the words
                report("wake_clip", serde_json::json!({"seconds": clip.len() as f32 / 16_000.0, "matched": hit.is_some()}));
                if let Some(rest) = hit {
                    report("wake_hit", serde_json::json!({"with_command": !rest.is_empty()}));
                    on_wake(&app2, &c2, rest);
                    seg.reset();
                    break;
                }
            }
        });
        if let Err(e) = res {
            report("error", serde_json::json!(format!("wake word: {e}")));
            std::thread::sleep(Duration::from_secs(3));
        }
    }
}

fn on_wake(app: &AppHandle, c: &Arc<Core>, rest: String) {
    let words = rest.split_whitespace().count();
    if words >= 2 {
        // "Hey Murmur, tell John I'm late" → rewrite and type the command right away
        {
            let mut m = c.mode.lock();
            if *m != Mode::Idle {
                return;
            }
            *m = Mode::Processing;
        }
        position_overlay(app);
        let s = c.settings.lock().clone();
        if s.sounds {
            audio::blip(audio::Blip::Stop);
        }
        let raw = llm::apply_voice_commands(&rest);
        let pending = llm::Pending::start(&s);
        deliver(app, c, &s, raw, pending, 0.0, Instant::now());
        *c.mode.lock() = Mode::Idle;
    } else {
        // just "Hey Murmur" → start listening hands-free (stops by itself after a pause)
        let mut m = c.mode.lock();
        if *m != Mode::Idle {
            return;
        }
        *m = Mode::HandsFree;
        drop(m);
        begin(app, c, true);
    }
}

// ───────────────────────────── self-test ─────────────────────────────
//
// MURMUR_SELFTEST=clip.wav runs one full dictation with the clip standing in for the microphone,
// through the real windows, then exits. Findings are written as JSON lines to MURMUR_REPORT.

/// Diagnostic log: always written to the app log (~/Library/Logs/Murmur/murmur.log on macOS),
/// and also to MURMUR_REPORT during self-tests. Contains no audio, only what happened.
fn report(kind: &str, data: serde_json::Value) {
    let line = serde_json::json!({"t": now_ms(), "kind": kind, "data": data});
    let mut paths = vec![log_path()];
    if let Some(p) = std::env::var_os("MURMUR_REPORT") {
        paths.push(PathBuf::from(p));
    }
    for path in paths {
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{line}");
        }
    }
}

pub fn log_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        dirs::home_dir().unwrap_or_default().join("Library/Logs/Murmur/murmur.log")
    } else {
        settings::data_dir().join("murmur.log")
    }
}

fn window_states(app: &AppHandle) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for label in ["main", "overlay"] {
        if let Some(w) = app.get_webview_window(label) {
            let pos = w.outer_position().ok();
            let size = w.outer_size().ok();
            out.insert(label.into(), serde_json::json!({
                "visible": w.is_visible().unwrap_or(false),
                "focused": w.is_focused().unwrap_or(false),
                "x": pos.map(|p| p.x), "y": pos.map(|p| p.y),
                "w": size.map(|s| s.width), "h": size.map(|s| s.height),
                "scale": w.scale_factor().unwrap_or(1.0),
            }));
        }
    }
    serde_json::Value::Object(out)
}

#[tauri::command]
fn ui_report(page: String, data: serde_json::Value) {
    report(&format!("ui:{page}"), data);
}

fn run_selftest(app: AppHandle, c: Arc<Core>, wav: String) {
    let (samples, rate) = match audio::read_wav(&wav) {
        Ok(x) => x,
        Err(e) => {
            report("error", serde_json::json!(format!("read wav: {e}")));
            return app.exit(2);
        }
    };
    let secs = samples.len() as f32 / rate as f32;
    if let Ok(p) = std::env::var("MURMUR_SELFTEST_PROVIDER") {
        c.settings.lock().provider = p;
    }
    if let Ok(st) = std::env::var("MURMUR_SELFTEST_STYLE") {
        c.settings.lock().style = st;
    }
    *c.test_audio.lock() = Some((samples, rate));
    std::thread::sleep(Duration::from_secs(3));
    report("windows:start", window_states(&app));
    on_press(&app);
    if std::env::var_os("MURMUR_SELFTEST_TAP").is_some() {
        // quick tap → hands-free; Murmur must stop by itself when the clip goes quiet
        std::thread::sleep(Duration::from_millis(120));
        on_release(&app);
        std::thread::sleep(Duration::from_secs_f32(secs.min(6.0) / 2.0));
        report("windows:listening", window_states(&app));
    } else {
        std::thread::sleep(Duration::from_secs_f32(secs.min(6.0) / 2.0));
        report("windows:listening", window_states(&app));
        std::thread::sleep(Duration::from_secs_f32(secs.min(6.0) / 2.0 + 0.3));
        on_release(&app);
    }
    let t = Instant::now();
    while *c.mode.lock() != Mode::Idle && t.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(20));
    }
    report("pipeline", serde_json::json!({"finished_ms": t.elapsed().as_millis() as u64, "mode": format!("{:?}", *c.mode.lock())}));
    std::thread::sleep(Duration::from_millis(900));
    report("windows:done", window_states(&app));
    std::thread::sleep(Duration::from_millis(2500));
    report("exit", serde_json::json!(null));
    app.exit(0);
}

// ───────────────────────────── windows ─────────────────────────────

/// Lets the pill float over other apps' full-screen Spaces and every desktop, without taking focus.
/// (tao's alwaysOnTop uses the floating level, which can't draw over full-screen apps.)
#[cfg(target_os = "macos")]
fn raise_overlay(w: &tauri::WebviewWindow) {
    use objc2::{msg_send, runtime::AnyObject};
    const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
    const MOVE_TO_ACTIVE_SPACE: usize = 1 << 1;
    const MANAGED: usize = 1 << 2;
    const STATIONARY: usize = 1 << 4;
    const IGNORES_CYCLE: usize = 1 << 6;
    const FULLSCREEN_PRIMARY: usize = 1 << 7;
    const FULLSCREEN_AUXILIARY: usize = 1 << 8;
    const FULLSCREEN_NONE: usize = 1 << 9;
    // screen-saver level: what Electron uses for overlays that must show over full-screen apps
    const OVERLAY_LEVEL: isize = 1000;
    let w2 = w.clone();
    let _ = w.run_on_main_thread(move || {
        let Ok(ptr) = w2.ns_window() else { return };
        // SAFETY: Tauri owns this live NSWindow and we're on the main thread.
        let ns = unsafe { &*(ptr as *mut AnyObject) };
        unsafe {
            let cur: usize = msg_send![ns, collectionBehavior];
            let next = (cur & !(MOVE_TO_ACTIVE_SPACE | MANAGED | FULLSCREEN_PRIMARY | FULLSCREEN_NONE))
                | CAN_JOIN_ALL_SPACES
                | STATIONARY
                | IGNORES_CYCLE
                | FULLSCREEN_AUXILIARY;
            let _: () = msg_send![ns, setCollectionBehavior: next];
            let _: () = msg_send![ns, setLevel: OVERLAY_LEVEL];
            let _: () = msg_send![ns, setHidesOnDeactivate: false];
            let _: () = msg_send![ns, orderFrontRegardless];
        }
    });
}

/// Turns the pill's NSWindow into a non-activating NSPanel. macOS only lets this kind of window
/// (as used by Raycast, Loom, Wispr Flow) sit on top of other apps' native full-screen Spaces;
/// a plain NSWindow stays on the normal desktop no matter its level or collection behavior.
/// Done by giving the existing window an NSPanel subclass (no instance variables of its own).
#[cfg(target_os = "macos")]
fn make_panel(w: &tauri::WebviewWindow) {
    use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
    use objc2::{msg_send, sel};
    extern "C" {
        fn object_setClass(obj: *mut AnyObject, cls: *const AnyClass) -> *const AnyClass;
        fn object_getClass(obj: *const AnyObject) -> *const AnyClass;
        fn class_getInstanceSize(cls: *const AnyClass) -> usize;
    }
    extern "C" fn no(_: *mut AnyObject, _: Sel) -> Bool {
        Bool::NO
    }
    const NONACTIVATING_PANEL: usize = 1 << 7; // NSWindowStyleMaskNonactivatingPanel
    let w2 = w.clone();
    let _ = w.run_on_main_thread(move || {
        let Ok(ptr) = w2.ns_window() else { return };
        let ns = ptr as *mut AnyObject;
        let Some(panel_base) = AnyClass::get(c"NSPanel") else { return };
        let cls = AnyClass::get(c"MurmurOverlayPanel").unwrap_or_else(|| {
            let mut b = ClassBuilder::new(c"MurmurOverlayPanel", panel_base).expect("declare panel class");
            unsafe {
                b.add_method(sel!(canBecomeKeyWindow), no as extern "C" fn(*mut AnyObject, Sel) -> Bool);
                b.add_method(sel!(canBecomeMainWindow), no as extern "C" fn(*mut AnyObject, Sel) -> Bool);
            }
            b.register()
        });
        unsafe {
            // Only swap if the current window class keeps no extra per-instance data
            // (otherwise code reading it after the swap could crash).
            let current = object_getClass(ns);
            let (cur_size, panel_size) = (class_getInstanceSize(current), class_getInstanceSize(cls));
            if cur_size > panel_size {
                report("panel", serde_json::json!({"converted": false, "current_size": cur_size, "panel_size": panel_size}));
                return;
            }
            object_setClass(ns, cls);
            let style: usize = msg_send![&*ns, styleMask];
            let _: () = msg_send![&*ns, setStyleMask: style | NONACTIVATING_PANEL];
            let _: () = msg_send![&*ns, setFloatingPanel: true];
            let _: () = msg_send![&*ns, setBecomesKeyOnlyIfNeeded: true];
        }
        report("panel", serde_json::json!({"converted": true}));
    });
    raise_overlay(w);
}

#[cfg(not(target_os = "macos"))]
fn raise_overlay(w: &tauri::WebviewWindow) {
    let _ = w.set_always_on_top(true);
}

fn position_overlay(app: &AppHandle) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let cursor = app.cursor_position().ok();
    let monitors = app.available_monitors().unwrap_or_default();
    let mon = cursor
        .and_then(|p| {
            monitors.iter().find(|m| {
                let (pos, sz) = (m.position(), m.size());
                p.x >= pos.x as f64 && p.x < pos.x as f64 + sz.width as f64 && p.y >= pos.y as f64 && p.y < pos.y as f64 + sz.height as f64
            })
        })
        .cloned()
        .or_else(|| app.primary_monitor().ok().flatten());
    let (Some(m), Ok(size)) = (mon, w.outer_size()) else { return };
    let (pos, sz, scale) = (m.position(), m.size(), m.scale_factor());
    let x = pos.x + (sz.width as i32 - size.width as i32) / 2;
    let y = pos.y + sz.height as i32 - size.height as i32 - (64.0 * scale) as i32;
    let _ = w.set_position(PhysicalPosition::new(x, y));
    raise_overlay(&w);
}

fn show_main(app: &AppHandle, section: Option<&str>) {
    // Stay an accessory (menu-bar) app even with this window open: macOS only lets accessory
    // apps put windows over other apps' full-screen Spaces, which the dictation pill needs.
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        if let Some(s) = section {
            let _ = w.emit("navigate", s);
        }
    }
}

/// Load the voice model in the background so the first dictation doesn't pay for it
/// (the very first load also compiles GPU kernels, which can take several seconds).
fn preload(c: &Arc<Core>) {
    let c = c.clone();
    std::thread::spawn(move || {
        let model = c.settings.lock().model.clone();
        if stt::installed(&model) {
            let _ = c.stt.ensure_loaded(&model);
        }
    });
}

// ───────────────────────────── commands ─────────────────────────────

#[derive(Serialize)]
struct ModelState {
    #[serde(flatten)]
    info: stt::ModelInfo,
    installed: bool,
}

#[tauri::command]
fn get_state(c: CoreRef, app: AppHandle) -> serde_json::Value {
    let models: Vec<ModelState> = stt::MODELS.iter().map(|m| ModelState { info: *m, installed: stt::installed(m.id) }).collect();
    serde_json::json!({
        "settings": *c.settings.lock(),
        "models": models,
        "history": *c.history.lock(),
        "platform": std::env::consts::OS,
        "vibrancy": *c.vibrancy.lock(),
        "defaultShortcut": settings::default_shortcut(),
        "launchAtLogin": app.autolaunch().is_enabled().unwrap_or(false),
        "version": app.package_info().version.to_string(),
    })
}

#[tauri::command]
fn save_settings(app: AppHandle, c: CoreRef, new: Settings) -> Result<(), String> {
    let old = c.settings.lock().clone();
    if new.shortcut != old.shortcut {
        register_shortcut(&app, &c, &new.shortcut)?;
    }
    let model_changed = new.model != old.model;
    if model_changed {
        c.stt.unload();
    }
    settings::save(&settings_path(), &new);
    *c.settings.lock() = new;
    if model_changed {
        preload(c.inner());
    }
    Ok(())
}

fn register_shortcut(app: &AppHandle, c: &Core, accel: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    let sc: tauri_plugin_global_shortcut::Shortcut = accel.parse().map_err(|e| format!("Invalid shortcut: {e}"))?;
    if let Some(prev) = c.registered.lock().take() {
        let _ = gs.unregister(prev.as_str());
    }
    match gs.register(sc) {
        Ok(()) => {
            *c.registered.lock() = Some(accel.to_string());
            Ok(())
        }
        Err(e) => Err(format!("That shortcut is taken by another app ({e}). Try a different one.")),
    }
}

#[tauri::command]
async fn provider_status() -> serde_json::Value {
    tauri::async_runtime::spawn_blocking(|| {
        let (claude, codex) = std::thread::scope(|s| {
            let a = s.spawn(|| cli::status("claude"));
            let b = s.spawn(|| cli::status("codex"));
            (a.join().unwrap_or_default(), b.join().unwrap_or_default())
        });
        serde_json::json!({
            "claude": claude,
            "codex": codex,
            "anthropicKey": settings::get_secret("anthropic_key").is_some(),
            "openaiKey": settings::get_secret("openai_key").is_some(),
            "accessibility": paste::accessibility_ok(),
        })
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
fn setup_provider(provider: String, action: String) -> Result<String, String> {
    let cmd = cli::setup_command(&provider, &action).ok_or("Install it first.")?;
    cli::open_terminal(&cmd)?;
    Ok(cmd)
}

#[tauri::command]
fn set_api_key(name: String, value: String) -> Result<(), String> {
    if name != "anthropic_key" && name != "openai_key" {
        return Err("unknown key".into());
    }
    settings::set_secret(&name, value.trim())
}

#[derive(Serialize, Clone)]
struct Progress {
    id: String,
    done: u64,
    total: u64,
}

#[tauri::command]
async fn download_model(app: AppHandle, id: String) -> Result<(), String> {
    let m = stt::model(&id).ok_or("Unknown model")?;
    let dir = stt::models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let part = dir.join(format!("{}.part", m.file));
    let resp = reqwest::Client::new()
        .get(stt::url(m))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Download failed: {e}"))?;
    let total = resp.content_length().unwrap_or(m.size_mb as u64 * 1_000_000);
    let mut f = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut done = 0u64;
    let mut last = Instant::now();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download interrupted: {e}"))?;
        f.write_all(&chunk).map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        if last.elapsed() > Duration::from_millis(80) {
            last = Instant::now();
            let _ = app.emit("download", Progress { id: id.clone(), done, total });
        }
    }
    f.flush().map_err(|e| e.to_string())?;
    drop(f);
    if done < total / 2 {
        let _ = std::fs::remove_file(&part);
        return Err("Download was incomplete. Try again.".into());
    }
    std::fs::rename(&part, dir.join(m.file)).map_err(|e| e.to_string())?;
    preload(app.state::<Arc<Core>>().inner());
    let _ = app.emit("download", Progress { id, done: total, total });
    Ok(())
}

#[tauri::command]
fn delete_model(c: CoreRef, id: String) -> Result<(), String> {
    if c.settings.lock().model == id {
        c.stt.unload();
    }
    let p = stt::model_path(&id).ok_or("Unknown model")?;
    std::fs::remove_file(p).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct Rewrite {
    text: String,
    ms: u64,
}

/// "Try it" in settings: rewrite typed text with the current AI settings.
#[tauri::command]
async fn test_rewrite(c: CoreRef<'_>, text: String, settings: Option<Settings>) -> Result<Rewrite, String> {
    let s = settings.unwrap_or_else(|| c.settings.lock().clone());
    tauri::async_runtime::spawn_blocking(move || {
        let t = Instant::now();
        let p = llm::Pending::start(&s)?;
        let text = p.finish(&text)?;
        Ok(Rewrite { text, ms: t.elapsed().as_millis() as u64 })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn clear_history(c: CoreRef) {
    let mut h = c.history.lock();
    h.clear();
    settings::save(&history_path(), &*h);
}

#[tauri::command]
fn delete_history_item(c: CoreRef, ts: u64) {
    let mut h = c.history.lock();
    h.retain(|i| i.ts != ts);
    settings::save(&history_path(), &*h);
}

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_privacy(which: String) {
    if which == "accessibility" && paste::request_accessibility() {
        return;
    }
    paste::open_privacy_pane(&which);
}

#[tauri::command]
fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), String> {
    let a = app.autolaunch();
    if enabled { a.enable() } else { a.disable() }.map_err(|e| e.to_string())
}

#[tauri::command]
fn pause_shortcut(app: AppHandle, c: CoreRef, paused: bool) -> Result<(), String> {
    // lets the settings page record a new shortcut without triggering dictation
    if paused {
        if let Some(prev) = c.registered.lock().take() {
            let _ = app.global_shortcut().unregister(prev.as_str());
        }
        Ok(())
    } else {
        let s = c.settings.lock().shortcut.clone();
        register_shortcut(&app, &c, &s)
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

// ───────────────────────────── app ─────────────────────────────

pub fn run() {
    let settings: Settings = settings::load(&settings_path());
    let history: Vec<HistoryItem> = settings::load(&history_path());
    let state = Arc::new(Core {
        settings: Mutex::new(settings),
        mode: Mutex::new(Mode::Idle),
        session: Mutex::new(None),
        stt: stt::Stt::new(),
        history: Mutex::new(history),
        registered: Mutex::new(None),
        vibrancy: Mutex::new(false),
        test_audio: Mutex::new(None),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app, None)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--background"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, sc, ev| {
                    if sc.matches(Modifiers::empty(), Code::Escape) {
                        if ev.state() == ShortcutState::Pressed {
                            cancel(app);
                        }
                        return;
                    }
                    match ev.state() {
                        ShortcutState::Pressed => on_press(app),
                        ShortcutState::Released => on_release(app),
                    }
                })
                .build(),
        )
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_settings,
            provider_status,
            setup_provider,
            set_api_key,
            download_model,
            delete_model,
            test_rewrite,
            clear_history,
            delete_history_item,
            copy_text,
            open_privacy,
            set_launch_at_login,
            pause_shortcut,
            ui_report,
            quit
        ])
        .on_window_event(|w, ev| {
            if let WindowEvent::CloseRequested { api, .. } = ev {
                if w.label() == "main" {
                    api.prevent_close();
                    let _ = w.hide();
                    #[cfg(target_os = "macos")]
                    let _ = w.app_handle().set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            }
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();

            if let Some(main) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
                    *state.vibrancy.lock() = apply_vibrancy(&main, NSVisualEffectMaterial::Sidebar, Some(NSVisualEffectState::Active), None).is_ok();
                }
                #[cfg(windows)]
                {
                    *state.vibrancy.lock() = window_vibrancy::apply_mica(&main, None).is_ok();
                }
                let _ = &main;
            }
            if let Some(ov) = app.get_webview_window("overlay") {
                // stays up but invisible & click-through; content animates in on demand
                let _ = ov.set_ignore_cursor_events(true);
                #[cfg(target_os = "macos")]
                make_panel(&ov);
                let _ = ov.show();
                position_overlay(&handle);
            }

            let shortcut = state.settings.lock().shortcut.clone();
            let registered = register_shortcut(&handle, &state, &shortcut);
            if let Err(e) = &registered {
                eprintln!("murmur: {e}");
            }
            // ask macOS for paste permission up front (shows the system prompt once if missing)
            let trusted = paste::accessibility_ok() || paste::request_accessibility();
            if let Ok(meta) = std::fs::metadata(log_path()) {
                if meta.len() > 2_000_000 {
                    let _ = std::fs::remove_file(log_path());
                }
            }
            report("startup", serde_json::json!({
                "shortcut": shortcut, "shortcut_ok": registered.is_ok(), "shortcut_error": registered.err(),
                "accessibility": trusted, "exe": std::env::current_exe().ok(),
            }));

            let open = MenuItem::with_id(app, "open", "Open Murmur…", true, None::<&str>)?;
            let dictate = MenuItem::with_id(app, "dictate", "Start Dictation", true, None::<&str>)?;
            let history = MenuItem::with_id(app, "history", "History", true, None::<&str>)?;
            let prompt_on = state.settings.lock().style == "prompt";
            let prompt_mode = CheckMenuItem::with_id(app, "prompt", "Prompt Mode: turn speech into AI prompts", true, prompt_on, None::<&str>)?;
            let prompt_item = prompt_mode.clone();
            let quit = MenuItem::with_id(app, "quit", "Quit Murmur", true, Some("CmdOrCtrl+Q"))?;
            let menu = Menu::with_items(app, &[&dictate, &prompt_mode, &PredefinedMenuItem::separator(app)?, &open, &history, &PredefinedMenuItem::separator(app)?, &quit])?;
            TrayIconBuilder::with_id("murmur")
                // macOS: monochrome template that adapts to the menu bar; elsewhere: the color icon
                .icon(tauri::image::Image::from_bytes(if cfg!(target_os = "macos") {
                    include_bytes!("../icons/tray.png").as_slice()
                } else {
                    include_bytes!("../icons/32x32.png").as_slice()
                })?)
                .icon_as_template(cfg!(target_os = "macos"))
                .tooltip("Murmur")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, e| match e.id.as_ref() {
                    "prompt" => {
                        let c = core(app);
                        let mut s = c.settings.lock();
                        s.style = if s.style == "prompt" { "clean".into() } else { "prompt".into() };
                        let _ = prompt_item.set_checked(s.style == "prompt");
                        settings::save(&settings_path(), &*s);
                        drop(s);
                        let _ = app.emit("settings-changed", ());
                    }
                    "open" => show_main(app, None),
                    "history" => show_main(app, Some("history")),
                    "dictate" => {
                        let c = core(app);
                        let mut m = c.mode.lock();
                        if *m == Mode::Idle {
                            *m = Mode::HandsFree;
                            drop(m);
                            begin(app, &c, true);
                        } else {
                            drop(m);
                            finish(app, &c);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            {
                let (h, c) = (handle.clone(), state.clone());
                std::thread::Builder::new().name("murmur-wake".into()).spawn(move || wake_loop(h, c))?;
            }
            if let Ok(wav) = std::env::var("MURMUR_SELFTEST") {
                let (h, c) = (handle.clone(), state.clone());
                std::thread::spawn(move || run_selftest(h, c, wav));
            }

            let first_run = { !state.settings.lock().onboarded };
            let model = state.settings.lock().model.clone();
            // Opened by the user → show the window. Started at login (--background) or by a
            // self-test → stay quietly in the menu bar unless setup isn't finished.
            let background = std::env::args().any(|a| a == "--background") || std::env::var_os("MURMUR_SELFTEST").is_some();
            if first_run || !stt::installed(&model) || !background {
                show_main(&handle, None);
            }
            preload(&state);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Murmur")
        .run(|app, ev| {
            // clicking Murmur in Finder/Spotlight/Dock while it's already running
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = ev {
                show_main(app, None);
            }
            if let tauri::RunEvent::Exit = ev {
                // whisper.cpp's Metal backend asserts if a model is still loaded at process exit
                app.state::<Arc<Core>>().stt.unload();
            }
        });
}
