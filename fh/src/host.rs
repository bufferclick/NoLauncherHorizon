//! The system around the page: the player's browser, the clipboard a sign-in code goes to, the
//! Explorer window a folder button opens, and the log file that says what happened when the page
//! can't.
//!
//! Nothing here opens a window of its own. A double-clicked .exe has no console to print to, so
//! every line that matters is kept in `fh.log` beside the page's data, and the only things this
//! program ever starts directly are the browser, the folder and the game.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Instant;

/// Opens `url` in the player's default browser (the shell's handler on Windows, the desktop's
/// elsewhere).
#[cfg(any(windows, unix))]
pub fn open(url: &str) -> Result<(), String> {
    let status = opener(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Could not open {url}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Could not open {url}: {status}"))
    }
}

#[cfg(not(any(windows, unix)))]
pub fn open(_url: &str) -> Result<(), String> {
    Err("This system has no browser to open.".into())
}

#[cfg(windows)]
fn opener(url: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut command = horizon_launcher::process::command("cmd");
    // `start` hands the address to the shell's handler for it; the empty title keeps the quotes
    // of the address itself straight, and no console window opens (process::command).
    command.raw_arg(format!("/D /S /C start \"\" \"{url}\""));
    command
}

#[cfg(target_os = "macos")]
fn opener(url: &str) -> std::process::Command {
    let mut command = horizon_launcher::process::command("open");
    command.arg(url);
    command
}

#[cfg(all(unix, not(target_os = "macos")))]
fn opener(url: &str) -> std::process::Command {
    let mut command = horizon_launcher::process::command("xdg-open");
    command.arg(url);
    command
}

/// Opens `path` in the system's file manager: Explorer on Windows, the desktop's elsewhere.
pub fn open_folder(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!("{} is not a folder on this computer.", path.display()));
    }
    #[cfg(windows)]
    let mut command = {
        let mut command = horizon_launcher::process::command("explorer.exe");
        command.arg(path);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = horizon_launcher::process::command("open");
        command.arg(path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = horizon_launcher::process::command("xdg-open");
        command.arg(path);
        command
    };
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    // Explorer answers 1 even when the window opened: only a failed start is a failure here.
    if status.success() || cfg!(windows) {
        Ok(())
    } else {
        Err(format!("Could not open {}: {status}", path.display()))
    }
}

/// Puts `text` on the clipboard, so the sign-in code can be pasted on Microsoft's page: Windows'
/// own `clip.exe`, which keeps the clipboard after this program exits. False where it can't.
#[cfg(windows)]
pub fn copy(text: &str) -> bool {
    let mut child = match horizon_launcher::process::command("clip.exe")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let written = child
        .stdin
        .take()
        .map(|mut stdin| stdin.write_all(text.as_bytes()).is_ok())
        .unwrap_or(false);
    written && child.wait().map(|status| status.success()).unwrap_or(false)
}

#[cfg(not(windows))]
pub fn copy(_text: &str) -> bool {
    false
}

/// The log of the starts: every line the work reported, with the second it came in at. It is the
/// only place a failure is explained in full — a double-clicked .exe has no terminal.
pub struct Log {
    path: PathBuf,
    started: Instant,
    file: Mutex<Option<std::fs::File>>,
}

impl Log {
    /// The log at `path`, made fresh when the last one grew past a megabyte.
    pub fn open(path: PathBuf) -> Self {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > 1 << 20) {
            let _ = std::fs::remove_file(&path);
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();
        Self {
            path,
            started: Instant::now(),
            file: Mutex::new(file),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// One line, with the seconds since the start. Also on a terminal's stderr, where there is
    /// one (a start from a command prompt with its output redirected, a test).
    pub fn line(&self, text: &str) {
        let line = format!("[{:>6.2}s] {}\n", self.started.elapsed().as_secs_f32(), text);
        // A terminal's stderr, where there is one. A double-clicked .exe has no console, and
        // `eprint!` panics where writing fails, so the handle is written to and ignored.
        let _ = std::io::stderr().write_all(line.as_bytes());
        if let Ok(mut file) = self.file.lock() {
            if let Some(file) = file.as_mut() {
                let _ = file.write_all(line.as_bytes());
            }
        }
    }
}
