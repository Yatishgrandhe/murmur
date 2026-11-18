//! Finding and talking to the official `claude` (Claude Pro/Max plan) and `codex`
//! (ChatGPT Plus/Pro plan) command-line tools. Murmur never sees the user's login:
//! it simply runs the CLI the user installed and signed in to.

use serde::Serialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// GUI apps on macOS start with a bare PATH (no Homebrew, no ~/.local/bin), so build
/// one that includes the user's login-shell PATH and the usual install locations.
pub fn path_var() -> &'static OsString {
    static P: OnceLock<OsString> = OnceLock::new();
    P.get_or_init(|| {
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Some(p) = std::env::var_os("PATH") {
            dirs.extend(std::env::split_paths(&p));
        }
        #[cfg(unix)]
        {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
            if let Ok(o) = Command::new(shell).args(["-lc", "printf %s \"$PATH\""]).stdin(Stdio::null()).output() {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                dirs.extend(std::env::split_paths(&OsString::from(s)));
            }
        }
        dirs.extend(extra_dirs());
        let mut seen = std::collections::HashSet::new();
        dirs.retain(|d| !d.as_os_str().is_empty() && seen.insert(d.clone()));
        match std::env::join_paths(&dirs) {
            Ok(p) => p,
            Err(_) => {
                // One bad entry (a quote or ';' in a Windows path) must not wipe out the whole PATH.
                dirs.retain(|d| std::env::join_paths([d]).is_ok());
                std::env::join_paths(dirs).unwrap_or_default()
            }
        }
    })
}

fn extra_dirs() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut v = vec![
        home.join(".local/bin"),
        home.join(".claude/local"),
        home.join(".npm-global/bin"),
        home.join(".bun/bin"),
        home.join(".volta/bin"),
        home.join(".cargo/bin"),
    ];
    if cfg!(target_os = "macos") {
        v.push("/opt/homebrew/bin".into());
        v.push("/usr/local/bin".into());
    }
    #[cfg(windows)]
    {
        if let Some(a) = std::env::var_os("APPDATA") {
            v.push(PathBuf::from(a).join("npm"));
        }
        if let Some(u) = std::env::var_os("USERPROFILE") {
            v.push(PathBuf::from(u).join("scoop").join("shims"));
        }
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            v.push(PathBuf::from(&l).join("Yarn").join("bin"));
            v.push(PathBuf::from(&l).join("Programs").join("claude"));
            v.push(PathBuf::from(l).join("pnpm"));
        }
        if let Some(p) = std::env::var_os("ProgramFiles") {
            v.push(PathBuf::from(p).join("nodejs"));
        }
    }
    v
}

pub fn find(name: &str) -> Option<PathBuf> {
    which::which_in(name, Some(path_var()), std::env::temp_dir()).ok().or_else(|| {
        // Cached PATH is stale if the CLI was installed while Murmur runs: re-check the install dirs.
        extra_dirs().iter().find_map(|d| which::which_in(name, Some(d), std::env::temp_dir()).ok())
    })
}

/// Like `find`, but on Windows a native `<name>.exe` wins over an npm `.cmd` shim in the same dirs.
pub fn find_prefer_exe(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let exe = format!("{name}.exe");
        if let Some(p) = std::env::split_paths(path_var()).map(|d| d.join(&exe)).find(|p| p.is_file()) {
            return Some(p);
        }
    }
    find(name)
}

/// Kills a CLI child. On Windows the npm `.cmd` shim runs node.exe as a grandchild that
/// `Child::kill` would leave behind, so the whole tree is killed with taskkill first.
pub fn kill_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
    let _ = child.kill();
}

/// `output()` with a deadline: a hung CLI is killed (with its process tree) instead of blocking.
fn output_within(c: &mut Command, limit: Duration) -> std::io::Result<std::process::Output> {
    let mut child = c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let deadline = Instant::now() + limit;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            kill_tree(&mut child);
            let _ = child.wait();
            return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Timed out checking sign-in"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.wait_with_output()
}

/// A `Command` for a CLI with the augmented PATH and no console window on Windows.
pub fn command(bin: &Path) -> Command {
    let mut c = Command::new(bin);
    c.env("PATH", path_var()).current_dir(std::env::temp_dir());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

#[derive(Serialize, Clone, Default)]
pub struct CliStatus {
    pub installed: bool,
    pub logged_in: bool,
    pub account: Option<String>,
    pub detail: String,
}

pub fn status(provider: &str) -> CliStatus {
    let name = match provider {
        "claude" => "claude",
        "codex" => "codex",
        _ => return CliStatus::default(),
    };
    let Some(bin) = find(name) else {
        return CliStatus { detail: "Not installed".into(), ..Default::default() };
    };
    let mut st = CliStatus { installed: true, ..Default::default() };
    let out = match provider {
        "claude" => output_within(command(&bin).args(["auth", "status"]), Duration::from_secs(6)),
        _ => output_within(command(&bin).args(["login", "status"]), Duration::from_secs(6)),
    };
    match out {
        Ok(o) => {
            let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if provider == "claude" {
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                    st.logged_in = v["loggedIn"].as_bool().unwrap_or(false);
                    let method = v["authMethod"].as_str().unwrap_or("");
                    st.account = v["email"].as_str().map(String::from);
                    st.detail = if !st.logged_in {
                        "Not signed in".into()
                    } else if method == "claude.ai" {
                        "Signed in with your Claude plan".into()
                    } else {
                        "Signed in (API billing)".into()
                    };
                } else {
                    st.detail = text.lines().next().unwrap_or("").to_string();
                }
            } else {
                st.logged_in = o.status.success() && !text.to_lowercase().contains("not logged in");
                st.detail = if st.logged_in {
                    if text.to_lowercase().contains("chatgpt") { "Signed in with ChatGPT".into() } else { text.lines().next().unwrap_or("Signed in").trim().to_string() }
                } else {
                    "Not signed in".into()
                };
            }
        }
        Err(e) => st.detail = e.to_string(),
    }
    st
}

/// Command line the user should run to install / sign in, shown in a terminal window.
pub fn setup_command(provider: &str, action: &str) -> Option<String> {
    let win = cfg!(windows);
    // PowerShell needs `&` to run a quoted path; POSIX shells take the quoted path as-is.
    let run = |bin: PathBuf, args: &str| {
        let p = bin.display().to_string();
        if win {
            format!("& '{}' {args}", p.replace('\'', "''"))
        } else {
            format!("'{p}' {args}")
        }
    };
    Some(match (provider, action) {
        ("claude", "install") if win => "irm https://claude.ai/install.ps1 | iex".into(),
        ("claude", "install") => "curl -fsSL https://claude.ai/install.sh | bash".into(),
        ("claude", "login") => run(find("claude")?, "auth login"),
        ("codex", "install") if !win && find("brew").is_some() => "brew install codex".into(),
        ("codex", "install") => "npm install -g @openai/codex".into(),
        ("codex", "login") => run(find("codex")?, "login"),
        _ => return None,
    })
}

/// Opens a visible terminal running `cmd` (Terminal.app on macOS, PowerShell on Windows).
pub fn open_terminal(cmd: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
            cmd.replace('\\', "\\\\").replace('"', "\\\"")
        );
        Command::new("osascript").args(["-e", &script]).spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x10;
        Command::new("powershell")
            .args(["-NoExit", "-ExecutionPolicy", "Bypass", "-Command", cmd])
            .env("PATH", path_var())
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        for term in ["x-terminal-emulator", "gnome-terminal", "konsole", "xterm"] {
            if Command::new(term).args(["-e", "sh", "-c", &format!("{cmd}; exec $SHELL")]).spawn().is_ok() {
                return Ok(());
            }
        }
        Err(format!("Run this in a terminal: {cmd}"))
    }
}
