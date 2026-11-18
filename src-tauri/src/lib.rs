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

use futures_util::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use settings::{HistoryItem, Settings};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
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
    position_overlay(app);
    if s.sounds {
        audio::blip(audio::Blip::Start);
    }
    let a2 = app.clone();
    let rec = audio::Recording::start(move |level| {
        let _ = a2.emit_to("overlay", "level", level);
    });
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
    let c = core(app);
    {
        let mut m = c.mode.lock();
        if !matches!(*m, Mode::Holding | Mode::HandsFree) {
            return;
        }
        *m = Mode::Idle;
    }
    let _ = app.global_shortcut().unregister("Escape");
    if let Some(sess) = c.session.lock().take() {
        drop(sess.rec);
        if let Ok(p) = sess.pending {
            std::thread::spawn(move || p.cancel());
        }
    }
    emit_phase(app, "idle", None, Some("Cancelled".into()), false);
}

fn pipeline(app: &AppHandle, c: &Arc<Core>, sess: Session) {
    let s = c.settings.lock().clone();
    if s.sounds {
        audio::blip(audio::Blip::Stop);
    }
    let t0 = Instant::now();
    let Session { rec, pending, .. } = sess;
    let fail = |msg: String, pending: Result<llm::Pending, String>| {
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

    let mut warning = None;
    let mut used = s.provider.clone();
    let text = match pending {
        Ok(p) if p.is_off() => {
            used = "off".into();
            raw.clone()
        }
        Ok(p) => match llm::trivial(&raw) {
            Some(t) => {
                p.cancel();
                t
            }
            None => {
                emit_phase(app, "rewriting", Some(raw.clone()), Some(llm::provider_label(&s.provider).into()), false);
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

    if paste::accessibility_ok() {
        if let Err(e) = paste::paste(&text, s.restore_clipboard) {
            let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.clone()));
            warning = Some(format!("Couldn't paste ({e}). It's on your clipboard."));
        }
    } else {
        let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.clone()));
        warning = Some("Copied. Allow Murmur under Accessibility to paste automatically.".into());
    }
    emit_phase(app, "done", Some(text.clone()), warning, false);

    let item = HistoryItem { ts: now_ms(), raw, text, provider: used, seconds, ms: t0.elapsed().as_millis() as u64 };
    let mut h = c.history.lock();
    h.insert(0, item);
    h.truncate(500);
    settings::save(&history_path(), &*h);
    drop(h);
    let _ = app.emit("history-changed", ());
}

// ───────────────────────────── windows ─────────────────────────────

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
}

fn show_main(app: &AppHandle, section: Option<&str>) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        if let Some(s) = section {
            let _ = w.emit("navigate", s);
        }
    }
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
    if new.model != old.model {
        c.stt.unload();
    }
    settings::save(&settings_path(), &new);
    *c.settings.lock() = new;
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
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app, None)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
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
                let _ = ov.show();
                position_overlay(&handle);
            }

            let shortcut = state.settings.lock().shortcut.clone();
            if let Err(e) = register_shortcut(&handle, &state, &shortcut) {
                eprintln!("murmur: {e}");
            }

            let open = MenuItem::with_id(app, "open", "Open Murmur…", true, None::<&str>)?;
            let dictate = MenuItem::with_id(app, "dictate", "Start Dictation", true, None::<&str>)?;
            let history = MenuItem::with_id(app, "history", "History", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Murmur", true, Some("CmdOrCtrl+Q"))?;
            let menu = Menu::with_items(app, &[&dictate, &PredefinedMenuItem::separator(app)?, &open, &history, &PredefinedMenuItem::separator(app)?, &quit])?;
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
                .on_menu_event(|app, e| match e.id.as_ref() {
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

            let first_run = { !state.settings.lock().onboarded };
            let model = state.settings.lock().model.clone();
            if first_run || !stt::installed(&model) {
                show_main(&handle, None);
            } else {
                let c2 = state.clone();
                std::thread::spawn(move || {
                    let _ = c2.stt.ensure_loaded(&model);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Murmur");
}
