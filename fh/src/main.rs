//! Fabric n Horizon: JLac's window over your Horizon instances. Double-click it and the browser
//! opens on a page that lists the instances this computer already has and starts the one you
//! pick — nothing else. There is no launcher to click through, no settings screen, no Azure app of
//! your own: the instances, the sign-in, the builds, the files and the game's command are all
//! `horizon-launcher`'s own, the same ones a launcher installed beside this one uses.
//!
//! The window is three parts: `ui.rs` draws the pages (HTML with JLac's palette and shape),
//! `server.rs` answers the browser on loopback with one per-run word every address carries, and
//! `job.rs` does the Launch the page asks for. `host.rs` is the system around them (the browser,
//! the clipboard, the folder, the log) and `instances.rs` turns the launcher's list into rows.
//!
//! Versioning (fh/VERSION, the tag the release workflow publishes): v1.2 first, then one tenth
//! per update — v1.3, …, v1.9, v1.10 (never v2.0 for an update) — and Minecraft's own numbers
//! later where a build is worth naming after them.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod host;
mod instances;
mod job;
mod qr;
mod server;
mod ui;

use horizon_launcher::account::Accounts;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The version this build is: `fh/VERSION` through `build.rs`, the same string the release is
/// tagged with.
pub const VERSION: &str = env!("FH_VERSION");

const HELP: &str = "\
Fabric n Horizon: your Horizon instances, one page.

Double-click it and your browser opens on a page that lists the Horizon instances this computer
has. Pick one and press Launch: it signs you in with Microsoft if it has to (the code is on the
page and in the clipboard), installs the build the instance plays when it is missing, takes the
game's files from your own Minecraft the way the launcher does, and starts the game.

Usage: FH [options]

Options:
  --version, -V    Print the version and stop
  --help, -h       Print this and stop

The data folder is the launcher's own (HORIZON_LAUNCHER_HOME overrides it), so the instances, the
builds, the worlds and the sign-in are shared with a launcher installed beside this one. The log
of the last starts is fh/fh.log in it. FH closes by itself when the page has not been asked for
anything for half an hour and no launch is waiting on a game.
";

/// How long the page may go unasked before FH closes: long enough to read a page or sign in
/// slowly, short enough that a closed tab does not leave a process behind.
const IDLE: Duration = Duration::from_secs(30 * 60);

/// The window: what every request is answered from.
struct App {
    /// The word every address carries, made once per run (`server::token`).
    token: String,
    log: Arc<host::Log>,
    /// The launch the page is watching, if any.
    job: Mutex<Option<Arc<job::Job>>>,
    /// What every page's foot says.
    footer: String,
    /// When a request was last answered, in seconds since the epoch.
    asked: AtomicU64,
}

fn main() {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                say(HELP);
                return;
            }
            "--version" | "-V" => {
                say(&format!("Fabric n Horizon v{VERSION}"));
                return;
            }
            other => {
                say(&format!("{other} is not an option of this program."));
                say("See FH --help.");
                std::process::exit(2);
            }
        }
    }
    std::process::exit(run());
}

/// Writes where a console may be reading; a double-click has none, and that is not a failure.
fn say(text: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stdout(), "{text}");
}

/// Opens the window and answers it until it is not asked for anything any more. 0 when the window
/// opened, 1 when it could not.
fn run() -> i32 {
    // The launcher's data folder, which a "Move to another drive" may have pointed elsewhere: the
    // same instances, builds and sign-in a launcher installed beside this one uses.
    let problem = horizon_launcher::data_location::startup();
    let dir = horizon_launcher::backend::data_dir().join("fh");
    let log = Arc::new(host::Log::open(dir.join("fh.log")));
    if let Some(problem) = problem {
        log.line(&problem);
    }
    log.line(&format!("Fabric n Horizon v{VERSION}."));
    let app = Arc::new(App {
        token: server::token(),
        footer: format!(
            "Fabric n Horizon v{VERSION} · instances from the launcher's folder · log: {}",
            log.path().display()
        ),
        log: log.clone(),
        job: Mutex::new(None),
        asked: AtomicU64::new(0),
    });
    let token = app.token.clone();
    let handler = app.clone();
    let server = match server::Server::start(token, move |request| handle(&handler, request)) {
        Ok(server) => server,
        Err(e) => {
            log.line(&e);
            return 1;
        }
    };
    app.moment();
    let url = server.url();
    log.line(&format!("The window is {url}."));
    if let Err(e) = host::open(&url) {
        log.line(&format!("{e} Open {url} in a browser yourself."));
    }
    while !app.tired() {
        std::thread::sleep(Duration::from_secs(30));
    }
    log.line("Nothing has asked for the page for half an hour: closing.");
    0
}

impl App {
    /// Remembers that a request was answered just now.
    fn moment(&self) {
        self.asked.store(now(), Ordering::Relaxed);
    }

    /// Whether the page has gone unasked for long enough to close, with no launch still running
    /// (a game in play keeps FH open: it holds the runtime slot the game starts from).
    fn tired(&self) -> bool {
        if now().saturating_sub(self.asked.load(Ordering::Relaxed)) < IDLE.as_secs() {
            return false;
        }
        match self.job.lock() {
            Ok(job) => !matches!(
                job.as_ref().map(|job| job.state().stage),
                Some(ui::Stage::Working) | Some(ui::Stage::Started)
            ),
            Err(_) => true,
        }
    }

    /// This instance's launch: the running one when it is this instance's and has not finished,
    /// a new one otherwise (a second Launch after it ended starts the game again).
    fn launch_of(&self, instance: &horizon_launcher::instances::Instance) -> Arc<job::Job> {
        let mut slot = match self.job.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(running) = slot.as_ref() {
            let stage = running.state().stage;
            if running.id == instance.id
                && matches!(stage, ui::Stage::Working | ui::Stage::Started)
            {
                return running.clone();
            }
        }
        let started = job::Job::start(instance.clone(), self.log.clone());
        *slot = Some(started.clone());
        started
    }
}

/// Now, in seconds since the epoch.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

/// Every request: the key first, then the page it asks for.
fn handle(app: &App, request: &server::Request) -> server::Response {
    app.moment();
    if request.method != "GET" && request.method != "HEAD" {
        return server::Response::text(405, "This window only reads: GET, and no forms.");
    }
    if request.get("k") != Some(app.token.as_str()) {
        return server::Response::text(
            403,
            "That address is missing the word FH opened your browser with. Open the page from \
             the address FH opened, or start FH again.",
        );
    }
    match request.path.as_str() {
        "/" => home(app, request),
        "/icon" => icon(request),
        "/launch" => launch(app, request),
        "/folder" => folder(app, request),
        other => server::Response::page(404, ui::missing(other, &app.footer)),
    }
}

/// The front page: the list on the left, the chosen instance on the right.
fn home(app: &App, request: &server::Request) -> server::Response {
    let list = horizon_launcher::instances::list();
    let chosen = request
        .get("id")
        .and_then(|id| list.iter().find(|instance| instance.id == id))
        .or_else(|| {
            // No row named: the one the launcher's own Play would start, then the first row.
            if list.is_empty() {
                return None;
            }
            let playing = horizon_launcher::instances::selected();
            list.iter()
                .find(|instance| instance.id == playing.id)
                .or_else(|| list.first())
        });
    let rows: Vec<ui::Row> = list.iter().map(instances::row).collect();
    let chosen_row = chosen.map(instances::row);
    let played = chosen.and_then(instances::played);
    let home_path = chosen.map(|instance| instance.home.display().to_string());
    let account = Accounts::load().selected().map(|account| account.name.clone());
    let note = match request.get("note") {
        Some(note) if !note.is_empty() => Some(note.to_owned()),
        _ => running_note(app),
    };
    server::Response::html(ui::home(&ui::Home {
        token: &app.token,
        account: account.as_deref(),
        rows: &rows,
        selected: chosen_row.as_ref(),
        home: home_path.as_deref(),
        played: played.as_deref(),
        note: note.as_deref(),
        footer: &app.footer,
    }))
}

/// A line under the buttons: the launch that failed, or the one that is waiting on a game.
fn running_note(app: &App) -> Option<String> {
    let job = app.job.lock().ok()?.as_ref().cloned()?;
    let state = job.state();
    match state.stage {
        ui::Stage::Failed => state.error,
        ui::Stage::Working => Some(format!("{} is starting…", job.name)),
        ui::Stage::Started => Some(format!("{} is running.", job.name)),
        ui::Stage::Done => Some(format!("{} is not running.", job.name)),
    }
}

/// A row's picture, as the SVG it asks for.
fn icon(request: &server::Request) -> server::Response {
    match request.get("id").and_then(instances::find) {
        Some(instance) => server::Response::svg(&instances::tile(&instance)),
        None => server::Response::text(404, "No instance has that name."),
    }
}

/// Starts the instance, or shows the launch that is already running; a page that reloads every
/// second asks for the same address and gets the same job.
fn launch(app: &App, request: &server::Request) -> server::Response {
    let Some(instance) = request.get("id").and_then(instances::find) else {
        return server::Response::text(404, "No instance has that name.");
    };
    if instance.java.is_some() {
        // Not started here: the row's button says so already, and this address keeps it that way.
        return server::Response::redirect(format!(
            "/?k={}&id={}&note={}",
            app.token,
            ui::encode_uri(&instance.id),
            ui::encode_uri(&format!(
                "{} plays Minecraft: Java Edition. FH starts Horizon instances; Minecraft ones \
                 are next.",
                instance.name
            )),
        ));
    }
    let job = app.launch_of(&instance);
    let state = job.state();
    let account = Accounts::load().selected().map(|account| account.name.clone());
    server::Response::html(ui::job(&ui::Job {
        account: account.as_deref(),
        instance: &job.name,
        title: &state.title,
        line: &state.line,
        fraction: state.fraction,
        code: state
            .code
            .as_ref()
            .map(|(code, link)| (code.as_str(), link.as_str())),
        stage: state.stage,
        error: state.error.as_deref(),
        footer: &app.footer,
    }))
}

/// Opens the instance's game folder in Explorer, and goes back to its row.
fn folder(app: &App, request: &server::Request) -> server::Response {
    let Some(instance) = request.get("id").and_then(instances::find) else {
        return server::Response::text(404, "No instance has that name.");
    };
    let back = format!("/?k={}&id={}", app.token, ui::encode_uri(&instance.id));
    match host::open_folder(&instance.home) {
        Ok(()) => server::Response::redirect(back),
        Err(e) => {
            app.log.line(&e);
            server::Response::redirect(format!("{back}&note={}", ui::encode_uri(&e)))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::VERSION;

    /// `fh/VERSION` and `fh/Cargo.toml` are bumped together: the tag the release workflow creates
    /// (`fh-v1.2`) comes from the file, the .exe's own version from the manifest, and a build
    /// where they disagree is a build nobody can name.
    #[test]
    fn the_version_is_the_one_the_release_is_tagged_with() {
        assert_eq!(VERSION, VERSION.trim(), "VERSION has no padding");
        assert!(
            env!("CARGO_PKG_VERSION").starts_with(&format!("{VERSION}.")),
            "Cargo.toml's {} and VERSION's {} are the same release",
            env!("CARGO_PKG_VERSION"),
            VERSION
        );
    }
}
