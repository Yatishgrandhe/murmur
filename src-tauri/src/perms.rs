//! Every permission Murmur needs, checked without prompting, so setup can skip what is
//! already done and only ask (after explaining why) when the user clicks a button.
//!
//! macOS: Microphone, Speech Recognition, Accessibility (to paste with ⌘V).
//! Windows: Microphone access for desktop apps, and "Online speech recognition".

use serde::Serialize;

/// "granted" | "denied" | "not_asked" | "unknown"
pub type State = &'static str;

#[derive(Serialize, Clone)]
pub struct Perms {
    pub mic: State,
    pub speech: State,
    /// macOS only; "granted" elsewhere.
    pub accessibility: State,
    /// All three are granted: setup has nothing left to ask for.
    pub all_granted: bool,
    /// Running from the mounted DMG / App Translocation instead of /Applications (macOS).
    pub not_installed: bool,
}

pub fn check() -> Perms {
    let mic = mic_state();
    let speech = speech_state();
    let accessibility = if crate::paste::accessibility_ok() { "granted" } else { ax_state_hint() };
    // "unknown" means Windows doesn't tell us; recording still works unless the user blocked it
    let ok = |s: State| s == "granted" || s == "unknown";
    Perms { mic, speech, accessibility, all_granted: ok(mic) && ok(speech) && ok(accessibility), not_installed: running_from_dmg() }
}

fn ax_state_hint() -> State {
    if cfg!(target_os = "macos") {
        "not_asked"
    } else {
        "granted"
    }
}

/// Asks for one permission. macOS shows its own prompt the first time; after a denial (or on
/// Windows, which has no prompt) the matching Settings page is opened instead.
pub fn request(which: &str) -> Perms {
    let before = check();
    match which {
        "mic" => {
            if before.mic == "not_asked" {
                request_mic();
            } else if before.mic != "granted" {
                crate::paste::open_privacy_pane("mic");
            }
        }
        "speech" => {
            if before.speech == "not_asked" || cfg!(windows) {
                crate::stt::request_access();
            } else if before.speech != "granted" {
                crate::paste::open_privacy_pane("speech");
            }
        }
        "accessibility" if !crate::paste::request_accessibility() => {
            crate::paste::open_privacy_pane("accessibility");
        }
        _ => {}
    }
    check()
}

/// macOS keeps a permission tied to the app's code signature. A build signed differently (an
/// update built without a stable certificate) leaves a stale Accessibility entry that shows as
/// "on" but doesn't work. Removing Murmur's entry lets the next request add it cleanly.
pub fn reset_accessibility() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("tccutil")
            .args(["reset", "Accessibility", "app.murmur.dictation"])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        crate::paste::request_accessibility();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

/// First launch after an update (macOS). Permissions belong to a code signature; an update with
/// a different one leaves entries that look "on" in System Settings but don't apply to this build
/// (Accessibility), or a refusal from a build that couldn't ask (Microphone). Murmur clears only
/// its own entries in those states, so the setup buttons show the macOS prompt again and Murmur
/// is listed in Settings. Working permissions are never touched.
pub fn heal_after_update() -> Vec<&'static str> {
    let mut cleared = Vec::new();
    #[cfg(target_os = "macos")]
    {
        let reset = |service: &str| {
            std::process::Command::new("tccutil")
                .args(["reset", service, "app.murmur.dictation"])
                .output()
                .is_ok_and(|o| o.status.success())
        };
        if !crate::paste::accessibility_ok() && reset("Accessibility") {
            cleared.push("accessibility");
        }
        if mic_state() == "denied" && reset("Microphone") {
            cleared.push("mic");
        }
    }
    cleared
}

fn running_from_dmg() -> bool {
    #[cfg(target_os = "macos")]
    {
        let p = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        p.starts_with("/Volumes/") || p.contains("/AppTranslocation/")
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

fn speech_state() -> State {
    let st = crate::stt::status();
    if st.ready {
        "granted"
    } else if st.can_request {
        "not_asked"
    } else {
        "denied"
    }
}

// ───────────────────────────── macOS microphone ─────────────────────────────

#[cfg(target_os = "macos")]
fn mic_state() -> State {
    use objc2::{class, msg_send};
    use objc2_foundation::NSString;
    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {}
    // AVMediaTypeAudio == @"soun"
    let media = NSString::from_str("soun");
    let status: isize = unsafe { msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: &*media] };
    match status {
        3 => "granted",
        0 => "not_asked",
        _ => "denied",
    }
}

#[cfg(target_os = "macos")]
fn request_mic() {
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2::{class, msg_send};
    use objc2_foundation::NSString;
    let media = NSString::from_str("soun");
    let (tx, rx) = std::sync::mpsc::channel();
    let block = RcBlock::new(move |granted: Bool| {
        let _ = tx.send(granted.as_bool());
    });
    unsafe {
        let _: () = msg_send![class!(AVCaptureDevice), requestAccessForMediaType: &*media, completionHandler: &*block];
    }
    let _ = rx.recv_timeout(std::time::Duration::from_secs(120));
}

// ───────────────────────────── Windows microphone ─────────────────────────────

#[cfg(windows)]
fn mic_state() -> State {
    // Settings › Privacy & security › Microphone: the global switch and "Let desktop apps access
    // your microphone". Windows only reports these via the consent store.
    let base = r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";
    let global = win_reg::read_string(base, "Value");
    let desktop = win_reg::read_string(&format!(r"{base}\NonPackaged"), "Value");
    if global.as_deref() == Some("Deny") || desktop.as_deref() == Some("Deny") {
        "denied"
    } else if global.as_deref() == Some("Allow") {
        "granted"
    } else {
        "unknown"
    }
}

#[cfg(windows)]
fn request_mic() {
    crate::paste::open_privacy_pane("mic");
}

#[cfg(windows)]
pub mod win_reg {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RRF_RT_REG_SZ};

    pub fn read_string(key: &str, value: &str) -> Option<String> {
        let (k, v) = (HSTRING::from(key), HSTRING::from(value));
        let mut buf = vec![0u16; 64];
        let mut len = (buf.len() * 2) as u32;
        let r = unsafe {
            RegGetValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(v.as_ptr()), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut len))
        };
        if r.is_err() {
            return None;
        }
        let n = (len as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
    }

    pub fn read_dword(key: &str, value: &str) -> Option<u32> {
        let (k, v) = (HSTRING::from(key), HSTRING::from(value));
        let mut out = 0u32;
        let mut len = 4u32;
        let r = unsafe {
            RegGetValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(v.as_ptr()), RRF_RT_REG_DWORD, None, Some((&mut out as *mut u32).cast()), Some(&mut len))
        };
        r.is_ok().then_some(out)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
fn mic_state() -> State {
    "unknown"
}

#[cfg(not(any(target_os = "macos", windows)))]
fn request_mic() {}
