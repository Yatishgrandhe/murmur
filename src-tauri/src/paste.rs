//! Typing the result into whatever app has focus: clipboard + simulated ⌘V / Ctrl+V,
//! then the user's previous clipboard is put back.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::time::Duration;

/// Retries a clipboard call while another app holds the clipboard (up to 5 tries, 40 ms apart).
fn with_retry<T, E: std::fmt::Display>(mut f: impl FnMut() -> Result<T, E>) -> Result<T, String> {
    let mut last = String::new();
    for attempt in 0..5 {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => last = e.to_string(),
        }
        if attempt < 4 {
            std::thread::sleep(Duration::from_millis(40));
        }
    }
    Err(last)
}

pub fn paste(text: &str, restore: bool) -> Result<(), String> {
    let mut cb = with_retry(arboard::Clipboard::new)?;
    let previous = if restore { cb.get_text().ok() } else { None };
    with_retry(|| cb.set_text(text.to_string()))?;
    std::thread::sleep(Duration::from_millis(30));
    press_paste()?;
    if let Some(prev) = previous {
        // Chrome/Electron read the pasteboard asynchronously; give them time before restoring
        std::thread::sleep(Duration::from_millis(500));
        let _ = with_retry(|| cb.set_text(prev.clone()));
    }
    Ok(())
}

fn press_paste() -> Result<(), String> {
    let mut e = Enigo::new(&Settings::default()).map_err(|e| format!("Keyboard access unavailable: {e}"))?;
    #[cfg(target_os = "macos")]
    let (modifier, v) = (Key::Meta, Key::Other(9)); // kVK_ANSI_V: layout-independent, no main-thread lookups
    // VK_V as a raw virtual-key code (enigo passes Key::Other straight through on Windows), so
    // Ctrl+V still works on non-Latin keyboard layouts where 'v' is a different key.
    #[cfg(windows)]
    let (modifier, v) = (Key::Control, Key::Other(0x56));
    #[cfg(all(not(target_os = "macos"), not(windows)))]
    let (modifier, v) = (Key::Control, Key::Unicode('v'));
    #[cfg(windows)]
    {
        // The Ctrl+Shift+Space shortcut may still be physically held; release leftover modifiers
        // so they don't turn the paste into Ctrl+Shift+V or Win+V. Errors are ignored on purpose.
        for m in [Key::Shift, Key::Alt, Key::Meta] {
            let _ = e.key(m, Direction::Release);
        }
    }
    e.key(modifier, Direction::Press).map_err(|e| e.to_string())?;
    let r = e.key(v, Direction::Click).map_err(|e| e.to_string());
    e.key(modifier, Direction::Release).map_err(|e| e.to_string())?;
    r
}

/// macOS needs permission to send the paste keystroke. Posting key events is its own check
/// (PostEvent, granted through the Accessibility list); on some macOS versions it and
/// AXIsProcessTrusted disagree, so either one is enough.
pub fn accessibility_ok() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGPreflightPostEventAccess() -> bool;
        }
        unsafe { CGPreflightPostEventAccess() || AXIsProcessTrusted() }
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
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGRequestPostEventAccess() -> bool;
        }
        unsafe {
            if CGRequestPostEventAccess() {
                return true;
            }
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
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let _ = std::process::Command::new("explorer")
                .arg("ms-settings:privacy-microphone")
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
    }
    let _ = which;
}

/// Is there somewhere to type? Checked right before pasting.
/// Some(true): a text field has focus. Some(false): clearly nowhere to type (nothing focused, the
/// desktop, a button/list...). None: can't tell. Errs toward pasting, and the dictation stays on
/// the clipboard either way, so a wrong guess never loses text.
#[cfg(target_os = "macos")]
pub fn focused_accepts_text() -> Option<bool> {
    use core_foundation::base::{CFType, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::{CFString, CFStringRef};

    type AXError = i32;
    const AX_OK: AXError = 0;
    const AX_NO_VALUE: AXError = -25212; // nothing focused in that app

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> CFTypeRef;
        fn AXUIElementCopyAttributeValue(el: CFTypeRef, attr: CFStringRef, out: *mut CFTypeRef) -> AXError;
        fn AXUIElementSetAttributeValue(el: CFTypeRef, attr: CFStringRef, value: CFTypeRef) -> AXError;
        fn AXUIElementIsAttributeSettable(el: CFTypeRef, attr: CFStringRef, settable: *mut u8) -> AXError;
        fn AXUIElementSetMessagingTimeout(el: CFTypeRef, seconds: f32) -> AXError;
        fn AXUIElementGetPid(el: CFTypeRef, pid: *mut i32) -> AXError;
    }
    unsafe fn copy_attr(el: CFTypeRef, name: &str) -> Result<CFType, AXError> {
        let attr = CFString::new(name);
        let mut out: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(el, attr.as_concrete_TypeRef(), &mut out);
        if err == AX_OK && !out.is_null() {
            Ok(CFType::wrap_under_create_rule(out))
        } else {
            Err(err)
        }
    }
    unsafe fn string_attr(el: CFTypeRef, name: &str) -> String {
        copy_attr(el, name).ok().and_then(|v| v.downcast::<CFString>()).map(|s| s.to_string()).unwrap_or_default()
    }

    unsafe {
        let sys_ptr = AXUIElementCreateSystemWide();
        if sys_ptr.is_null() {
            return None;
        }
        let sys = CFType::wrap_under_create_rule(sys_ptr);
        AXUIElementSetMessagingTimeout(sys.as_CFTypeRef(), 0.25);
        let app = copy_attr(sys.as_CFTypeRef(), "AXFocusedApplication").ok()?;
        let app_r = app.as_CFTypeRef();
        AXUIElementSetMessagingTimeout(app_r, 0.25);
        let mut pid = 0i32;
        if AXUIElementGetPid(app_r, &mut pid) != AX_OK || pid == std::process::id() as i32 {
            return None;
        }
        // Electron apps (Slack, VS Code…) only build their accessibility tree when asked
        let flag = CFString::new("AXManualAccessibility");
        let _ = AXUIElementSetAttributeValue(app_r, flag.as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef());
        let app_name = string_attr(app_r, "AXTitle");

        let el = match copy_attr(app_r, "AXFocusedUIElement") {
            Ok(e) => e,
            Err(AX_NO_VALUE) => return Some(false),
            Err(_) => return None,
        };
        let r = el.as_CFTypeRef();
        AXUIElementSetMessagingTimeout(r, 0.25);
        let role = string_attr(r, "AXRole");
        let editable = copy_attr(r, "AXEditable").ok().and_then(|v| v.downcast::<CFBoolean>()).map(bool::from);
        if editable == Some(true) {
            return Some(true);
        }
        if matches!(role.as_str(), "AXTextField" | "AXTextArea" | "AXComboBox" | "AXSearchField") {
            return Some(editable != Some(false));
        }
        let range = CFString::new("AXSelectedTextRange");
        let mut settable: u8 = 0;
        if AXUIElementIsAttributeSettable(r, range.as_concrete_TypeRef(), &mut settable) == AX_OK && settable != 0 {
            return Some(true);
        }
        const NOT_TEXT: [&str; 14] = [
            "AXButton", "AXCheckBox", "AXRadioButton", "AXPopUpButton", "AXMenuButton", "AXImage", "AXList",
            "AXOutline", "AXTable", "AXRow", "AXCell", "AXMenuItem", "AXLink", "AXSlider",
        ];
        if NOT_TEXT.contains(&role.as_str()) || (app_name == "Finder" && role != "AXTextField") {
            return Some(false);
        }
        None
    }
}

/// Windows: a classic caret means a text field; the desktop/taskbar means nowhere to type.
/// Browsers, Electron and UWP draw their own caret, so those stay "unknown" (paste as usual).
#[cfg(windows)]
pub fn focused_accepts_text() -> Option<bool> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
    };
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_null() {
            return Some(false);
        }
        let mut buf = [0u16; 128];
        let n = GetClassNameW(fg, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
        let class = String::from_utf16_lossy(&buf[..n]);
        if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return Some(false);
        }
        let tid = GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let mut gi: GUITHREADINFO = std::mem::zeroed();
        gi.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if GetGUIThreadInfo(tid, &mut gi) != 0 && !gi.hwndCaret.is_null() {
            return Some(true);
        }
        None
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn focused_accepts_text() -> Option<bool> {
    None
}
