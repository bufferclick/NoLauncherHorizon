//! One Launch, from the page: the sign-in when it is needed, the build, its files, the game.
//!
//! The steps are the launcher's own, in the order its Play takes them: `account` signs in with
//! Microsoft (the device code, its code on the page and in the clipboard) and holds the owner
//! gate; `backend` installs the build the instance plays, prepares its resources from the
//! player's own Minecraft install, and writes the game's command; `instances` and `launch` put it
//! in the instance's folder and through the player's wrapper.
//!
//! What is added here is the order, and the state the page reads while it works: one job at a
//! time, behind a lock, so a page that reloads every second sees the step it is on and no second
//! game starts beside the first.

use crate::host;
use crate::ui::Stage;
use horizon_launcher::account::{self, Accounts, SignIn, SignInState};
use horizon_launcher::instances::{self as instances_api, Instance};
use horizon_launcher::{backend, launch, mojang, process};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// How often the sign-in is looked at.
const POLL: Duration = Duration::from_millis(200);

/// What the page shows of the job: the words of the step, how far it got, the sign-in's code.
#[derive(Clone)]
pub struct State {
    pub title: String,
    pub line: String,
    pub fraction: Option<f32>,
    /// The code to type at Microsoft's page, while one is waiting, and the page it is typed at.
    pub code: Option<(String, String)>,
    pub stage: Stage,
    pub error: Option<String>,
}

/// One instance being launched, as the page reads it.
pub struct Job {
    /// The instance's id, so a page's reload finds the same job.
    pub id: String,
    /// Its name, for the side pane.
    pub name: String,
    state: Mutex<State>,
}

impl Job {
    /// Starts `instance` on its own thread, and returns the job the page watches.
    pub fn start(instance: Instance, log: Arc<host::Log>) -> Arc<Self> {
        let job = Arc::new(Self {
            id: instance.id.clone(),
            name: instance.name.clone(),
            state: Mutex::new(State {
                title: format!("Starting {}", instance.name),
                line: "Reading your sign-in and the game's files…".to_owned(),
                fraction: None,
                code: None,
                stage: Stage::Working,
                error: None,
            }),
        });
        let mine = job.clone();
        std::thread::spawn(move || {
            let result = run(&mine, &instance, &log);
            mine.finish(result, &log);
        });
        job
    }

    /// The job as it is now.
    pub fn state(&self) -> State {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_else(|_| State {
                title: "Starting".to_owned(),
                line: String::new(),
                fraction: None,
                code: None,
                stage: Stage::Failed,
                error: Some("The job's state could not be read.".into()),
            })
    }

    fn step(&self, title: &str, line: &str, fraction: Option<f32>) {
        if let Ok(mut state) = self.state.lock() {
            state.title = title.to_owned();
            state.line = line.to_owned();
            state.fraction = fraction;
            state.code = None;
        }
    }

    fn progress(&self, fraction: f32) {
        if let Ok(mut state) = self.state.lock() {
            state.fraction = Some(fraction.clamp(0., 1.));
        }
    }

    fn sign_in_code(&self, code: &str, link: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.code = Some((code.to_owned(), link.to_owned()));
        }
    }

    fn started(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.stage = Stage::Started;
            state.code = None;
            state.fraction = None;
        }
    }

    fn finish(&self, result: Result<String, String>, log: &host::Log) {
        if let Ok(mut state) = self.state.lock() {
            state.code = None;
            state.fraction = None;
            match result {
                Ok(line) => {
                    state.stage = Stage::Done;
                    state.title = format!("{} ended", self.name);
                    state.line = line;
                }
                Err(error) => {
                    log.line(&format!("{}: {error}", self.name));
                    state.stage = Stage::Failed;
                    state.title = format!("{} did not start", self.name);
                    state.error = Some(error);
                }
            }
        }
    }
}

/// The whole launch. Ok: what to say when the game ended; Err: why it did not start.
fn run(job: &Job, instance: &Instance, log: &Arc<host::Log>) -> Result<String, String> {
    log.line(&format!("Launching {} ({}).", instance.name, instance.id));
    sign_in(job, log)?;
    let tag = version(job, instance, log)?;
    prepare(job, &tag, log)?;
    play(job, instance, &tag, log)
}

/// The Microsoft sign-in, unless this computer has an account that owns the game already: the
/// code goes on the page and in the clipboard, and Microsoft's own page opens once.
fn sign_in(job: &Job, log: &Arc<host::Log>) -> Result<(), String> {
    let accounts = Accounts::load();
    if !accounts.accounts.is_empty() && accounts.has_owner(account::store()) {
        log.line("Signed in already.");
        return Ok(());
    }
    log.line("Signing in with Microsoft.");
    job.step(
        "Signing in with Microsoft",
        "Asking Microsoft for a sign-in code…",
        None,
    );
    let sign_in = SignIn::start();
    let mut opened = false;
    loop {
        match sign_in.state() {
            SignInState::Starting => {}
            SignInState::Code(code) => {
                let (user_code, link) = (code.user_code.clone(), code.verification_uri.clone());
                job.sign_in_code(&user_code, &link);
                if !opened {
                    opened = true;
                    // Microsoft's page takes the code in its address, so the player lands on it
                    // with nothing to type; the page behind shows the code all the same.
                    let open = crate::ui::open_url(&link, &user_code);
                    log.line(&format!("Signing in at {open} with the code {user_code}."));
                    if !host::copy(&user_code) {
                        log.line("The code could not be put on the clipboard; it is on the page.");
                    }
                    if let Err(e) = host::open(&open) {
                        log.line(&format!("{e} Open {open} in a browser yourself."));
                    }
                }
            }
            SignInState::Working(phase) => {
                job.step("Signing in with Microsoft", phase.label(), None);
            }
            SignInState::Done(Ok(account)) => {
                log.line(&format!("Signed in as {}.", account.name));
                return Ok(());
            }
            SignInState::Done(Err(e)) => return Err(e),
        }
        std::thread::sleep(POLL);
    }
}

/// The build the instance plays: its own pinned build, installed when this computer has no copy,
/// else the launcher's choice from what is installed, and the newest published build when
/// nothing is installed at all.
fn version(job: &Job, instance: &Instance, log: &Arc<host::Log>) -> Result<String, String> {
    if let Some(pinned) = instance.version.clone() {
        if backend::installed().contains(&pinned) {
            log.line(&format!("{} plays the installed {pinned}.", instance.name));
            return Ok(pinned);
        }
        return install(job, &pinned, log);
    }
    match backend::resolve_version(None) {
        Ok(tag) => {
            log.line(&format!("Playing the installed {tag}."));
            Ok(tag)
        }
        Err(_) => {
            // Nothing installed: the newest build GitHub publishes, as a launcher's Play would.
            job.step("Getting Horizon", "Asking GitHub for the newest build…", None);
            let releases = backend::releases().map_err(|e| {
                format!("No Horizon build is installed on this computer yet, and GitHub could not be asked for one ({e}).")
            })?;
            let latest = backend::latest(&releases)
                .ok_or("GitHub lists no Horizon build for this computer.")?;
            install(job, &latest.tag, log)
        }
    }
}

fn install(job: &Job, tag: &str, log: &Arc<host::Log>) -> Result<String, String> {
    log.line(&format!("Installing {tag}."));
    job.step("Getting Horizon", &format!("Downloading {tag}…"), Some(0.0));
    let progress = |fraction: f32| job.progress(fraction);
    let tag = backend::install_tag(tag, false, progress)?;
    job.progress(1.0);
    log.line(&format!("Installed {tag}."));
    Ok(tag)
}

/// Prepares `tag`'s resources when it has none: from the player's own Minecraft install, as the
/// launcher's Play does on a build it has never prepared.
fn prepare(job: &Job, tag: &str, log: &Arc<host::Log>) -> Result<(), String> {
    if backend::check_prepared(tag) {
        log.line(&format!("{tag} is prepared."));
        return Ok(());
    }
    log.line(&format!("Preparing {tag}."));
    mojang::CANCEL.store(false, Ordering::Relaxed);
    job.step(
        "Getting the game's files",
        "Looking for your Minecraft install…",
        None,
    );
    let minecraft = backend::minecraft_dir(None)?;
    let lines = |line: String| log.line(&line);
    let (preparer, minecraft) =
        backend::preparer(tag, &minecraft, backend::MinecraftFrom::Install, &lines)?;
    // The sounds come beside the preparation (horizon-prepare sounds): a version without them
    // plays silently, which is a line in the log, never a failed start.
    backend::sounds(tag, &minecraft, &lines);
    backend::set_prepare_step(backend::PrepareStep::Prepare);
    job.step(
        "Preparing the game",
        "Building its tables from your Minecraft files…",
        None,
    );
    match preparer {
        backend::Preparer::InProcess(options) => backend::prepare_in_process(&options, &lines)?,
        backend::Preparer::Process(cmd) => {
            let (tx, rx) = std::sync::mpsc::channel();
            backend::spawn(backend::Job::Prepare, cmd, tx)?;
            loop {
                match rx.recv() {
                    Ok(backend::Msg::Line(backend::Job::Prepare, line)) => log.line(&line),
                    Ok(backend::Msg::Exit(backend::Job::Prepare, result)) => result?,
                    Ok(_) => {}
                    Err(_) => return Err("The preparation stopped without saying why.".into()),
                }
            }
        }
    }
    // The answer `check_prepared` kept is from before the preparation.
    backend::forget_prepared();
    if !backend::check_prepared(tag) {
        return Err(format!(
            "{tag} is still not prepared: the preparation stopped before it finished. The log \
             says where."
        ));
    }
    log.line(&format!("{tag} is prepared."));
    Ok(())
}

/// Starts the game in the instance's folder, and stays until it ends: the Windows runtime slot it
/// staged must outlive the game, the play time is saved, and a crash is worth the page's words.
fn play(job: &Job, instance: &Instance, tag: &str, log: &Arc<host::Log>) -> Result<String, String> {
    job.step(
        "Starting Horizon",
        "Reading your sign-in and the game's files…",
        None,
    );
    let mut cmd = backend::client_as(tag, None)?;
    instances_api::apply(&mut cmd, instance)?;
    // The instance's own variables first: `wrap` carries them into the wrapper it builds.
    let vars = instance.vars(cmd.get_program());
    cmd.envs(vars.iter().map(|(key, value)| (key, value)));
    let cmd = launch::wrap(cmd, &instance.launch);
    let (mut cmd, runtime) = backend::prepare_command(cmd)?;
    process::hide_console(&mut cmd);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(e) = account::take_unsaved() {
        log.line(&e);
    }
    log.line(&format!(
        "Starting {tag} in {}.",
        instance.home.display()
    ));
    let started = SystemTime::now();
    let clock = std::time::Instant::now();
    let status = cmd
        .spawn()
        .and_then(|mut child| {
            job.started();
            child.wait()
        })
        .map_err(|e| format!("Could not start {:?}: {e}", cmd.get_program()))?;
    drop(runtime);
    if let Err(e) = instances_api::record_play(&instance.id, clock.elapsed()) {
        log.line(&format!("Play time not saved: {e}"));
    }
    if let Some(mut after) = launch::after_exit(&instance.launch, &instance.home, &vars) {
        match after.status() {
            Ok(s) if s.success() => {}
            Ok(s) => log.line(&format!("The post-exit command failed ({s}).")),
            Err(e) => log.line(&format!("The post-exit command didn't run: {e}")),
        }
    }
    log.line(&format!("Horizon ended: {status}."));
    if status.success() {
        return Ok(format!(
            "Horizon ended after {}. This tab can be closed.",
            instances_api::play_time(clock.elapsed())
        ));
    }
    // A crash report says what went wrong; without one, the game's own log is the place.
    match backend::crash_report_since(&backend::game_home(), started) {
        Some(report) => {
            let summary = std::fs::read_to_string(&report)
                .ok()
                .and_then(|text| backend::crash_summary(&text));
            Err(match summary {
                Some(summary) => format!("Horizon crashed: {summary} — {}", report.display()),
                None => format!("Horizon crashed ({status}): {}", report.display()),
            })
        }
        None => Err(format!(
            "Horizon ended ({status}). Its log is in {}.",
            backend::logs_dir().display()
        )),
    }
}
