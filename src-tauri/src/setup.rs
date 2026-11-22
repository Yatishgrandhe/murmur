//! Setup state that only Murmur itself writes (the settings window never overwrites it):
//! which AI connections have been *verified* with a real request, and which installer files the
//! user has already been asked about. Lives next to settings.json, so a reinstall keeps it.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

/// A verification stays valid this long; every successful rewrite renews it.
const FRESH_MS: u64 = 14 * 24 * 3600 * 1000;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Verified {
    pub at_ms: u64,
    /// The account it was verified for (Claude reports an email); a different account needs a new check.
    pub account: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    verified: BTreeMap<String, Verified>,
    /// Installer file names the user already answered "delete?" for.
    installers_handled: Vec<String>,
}

fn path() -> PathBuf {
    crate::settings::config_dir().join("setup.json")
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(crate::settings::load(&path())))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn update(f: impl FnOnce(&mut State)) {
    let mut s = state().lock();
    f(&mut s);
    crate::settings::save(&path(), &*s);
}

pub fn mark_verified(provider: &str, account: Option<String>) {
    update(|s| {
        s.verified.insert(provider.to_string(), Verified { at_ms: now_ms(), account });
    });
}

pub fn clear_verified(provider: &str) {
    if state().lock().verified.contains_key(provider) {
        update(|s| {
            s.verified.remove(provider);
        });
    }
}

/// True only after a real request through this provider succeeded recently, for this account.
pub fn is_verified(provider: &str, account: Option<&str>) -> bool {
    let s = state().lock();
    let Some(v) = s.verified.get(provider) else { return false };
    let account_ok = match (&v.account, account) {
        (Some(a), Some(b)) => a == b,
        (Some(_), None) => false,
        (None, _) => true,
    };
    account_ok && now_ms().saturating_sub(v.at_ms) < FRESH_MS
}

/// Errors that mean the sign-in itself is broken (not a network blip).
pub fn is_auth_error(e: &str) -> bool {
    let e = e.to_lowercase();
    ["not logged in", "not signed in", "log in", "login", "authenticat", "unauthorized", "401", "invalid api key", "invalid x-api-key", "credential"]
        .iter()
        .any(|k| e.contains(k))
}

// ───────────────────────────── installer files ─────────────────────────────

#[derive(Serialize, Clone)]
pub struct InstallerFile {
    pub path: String,
    pub name: String,
    pub size_mb: f32,
}

fn looks_like_installer(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("murmur")
        && (n.ends_with(".dmg") || n.ends_with("-setup.exe") || n.ends_with(".msi") || n.ends_with(".app.tar.gz"))
}

fn downloads() -> Option<PathBuf> {
    dirs::download_dir()
}

/// Murmur installer files in Downloads that the user hasn't been asked about yet.
pub fn find_installers() -> Vec<InstallerFile> {
    let Some(dir) = downloads() else { return vec![] };
    let handled = state().lock().installers_handled.clone();
    let Ok(rd) = std::fs::read_dir(&dir) else { return vec![] };
    let mut out: Vec<InstallerFile> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let meta = e.metadata().ok()?;
            (meta.is_file() && looks_like_installer(&name) && !handled.contains(&name)).then(|| InstallerFile {
                path: e.path().to_string_lossy().to_string(),
                name,
                size_mb: meta.len() as f32 / 1_048_576.0,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The user's answer to "delete the installer?". Only files `find_installers` reported (in
/// Downloads, named like a Murmur installer) are ever touched, and they go to the Trash /
/// Recycle Bin, not deleted outright.
pub fn handle_installers(names: Vec<String>, delete: bool) -> Result<usize, String> {
    let current = find_installers();
    let mut trashed = 0;
    let mut err = None;
    for f in current.iter().filter(|f| names.contains(&f.name)) {
        if delete {
            match trash::delete(&f.path) {
                Ok(()) => trashed += 1,
                Err(e) => err = Some(format!("Couldn't move {} to the Trash: {e}", f.name)),
            }
        }
    }
    update(|s| {
        for n in names {
            if !s.installers_handled.contains(&n) {
                s.installers_handled.push(n);
            }
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(trashed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_names() {
        assert!(looks_like_installer("Murmur_0.4.0_aarch64.dmg"));
        assert!(looks_like_installer("Murmur_0.4.0_x64-setup.exe"));
        assert!(!looks_like_installer("Murmur notes.txt"));
        assert!(!looks_like_installer("Other_0.4.0.dmg"));
    }

    #[test]
    fn auth_errors() {
        assert!(is_auth_error("Claude: Not logged in · Please run /login"));
        assert!(!is_auth_error("Timed out after 45 s"));
    }
}
