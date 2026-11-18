//! Typing the result into whatever app has focus: clipboard + simulated ⌘V / Ctrl+V,
//! then the user's previous clipboard is put back.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::time::Duration;

pub fn paste(text: &str, restore: bool) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let previous = if restore { cb.get_text().ok() } else { None };
    cb.set_text(text.to_string()).map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_millis(30));
    press_paste()?;
    if let Some(prev) = previous {
        std::thread::sleep(Duration::from_millis(350));
        let _ = cb.set_text(prev);
    }
    Ok(())
}

fn press_paste() -> Result<(), String> {
    let mut e = Enigo::new(&Settings::default()).map_err(|e| format!("Keyboard access unavailable: {e}"))?;
    #[cfg(target_os = "macos")]
    let (modifier, v) = (Key::Meta, Key::Other(9)); // kVK_ANSI_V: layout-independent, no main-thread lookups
    #[cfg(not(target_os = "macos"))]
    let (modifier, v) = (Key::Control, Key::Unicode('v'));
    e.key(modifier, Direction::Press).map_err(|e| e.to_string())?;
    let r = e.key(v, Direction::Click).map_err(|e| e.to_string());
    e.key(modifier, Direction::Release).map_err(|e| e.to_string())?;
    r
}

/// macOS needs Accessibility permission to send the paste keystroke.
pub fn accessibility_ok() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        unsafe { AXIsProcessTrusted() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Shows macOS's own "Murmur would like to control this computer" prompt, which also adds
/// Murmur to the Accessibility list with the right identity. Returns the current trust state.
pub fn request_accessibility() -> bool {
    #[cfg(target_os = "macos")]
    {
        use core_foundation::base::TCFType;
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
        use core_foundation::string::{CFString, CFStringRef};
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            static kAXTrustedCheckOptionPrompt: CFStringRef;
            fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
        }
        unsafe {
            let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
            let opts = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
            AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub fn open_privacy_pane(which: &str) {
    #[cfg(target_os = "macos")]
    {
        let anchor = if which == "mic" { "Privacy_Microphone" } else { "Privacy_Accessibility" };
        let _ = std::process::Command::new("open")
            .arg(format!("x-apple.systempreferences:com.apple.preference.security?{anchor}"))
            .spawn();
    }
    #[cfg(windows)]
    {
        if which == "mic" {
            let _ = std::process::Command::new("cmd").args(["/C", "start", "ms-settings:privacy-microphone"]).spawn();
        }
    }
    let _ = which;
}
