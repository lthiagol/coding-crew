//! hugo — the TUI for the flow.
//!
//! hugo reads the runtime and shows it. It starts the orchestrator, which runs
//! the loop; it does not run the loop itself. It writes nothing but its own
//! configuration home.

mod app;
mod catalog;
mod config;
mod launch;
mod pick;
mod sidebar;
mod skill;

use std::io::{Stdout, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::{CrosstermBackend, TestBackend};

use app::{Action, App, RENDER_NOW, ReadError, RuntimeRead};
use coding_crew_core::runtime::{Runtime, RuntimeError};
use coding_crew_core::screen::{plain_text, restore, setup};
use coding_crew_core::store::RUNTIME_FILE;
use coding_crew_core::{Store, StoreError};
use coding_crew_theme::{IconSet, Preset, Theme};
use skill::SkillRequest;

const USAGE: &str = "usage: hugo [--theme dark|nord|catppuccin|dracula|terminal] [--icons nerd|unicode] [--render --plain [--width N] [--height N]] [--skill [orchestrator|builder|reviewer]]";
/// The columns of a plain render when --width is not given.
const DEFAULT_WIDTH: u16 = 100;
/// The rows of a plain render when --height is not given.
const DEFAULT_HEIGHT: u16 = 40;
/// The largest width or height a plain render accepts, as in raul: the
/// buffer is `width × height` cells, and an unbounded pair can ask for more
/// memory than the machine has.
const MAX_DIMENSION: u16 = 1000;

/// How often the loop looks for a catalogue while one is being read: the
/// window draws and takes keys between looks, so a slow tool costs the screen
/// a few frames, not its keys.
const CATALOG_POLL: Duration = Duration::from_millis(50);

/// How long the loop waits for a key when nothing is being read. The runtime
/// record and the hold lengths change without a key, so the window redraws at
/// this tick rather than only when a key or a resize arrives.
const TICK: Duration = Duration::from_secs(1);

/// The spinner's pace while an agent works: a frame every 120 milliseconds.
const WORKING_TICK: Duration = Duration::from_millis(120);

/// How long the loop waits for an event before it redraws: a short look while
/// a catalogue is being read, the spinner's pace while an agent works, and
/// otherwise the tick.
fn next_wait(catalog_pending: bool, working: bool) -> Duration {
    if catalog_pending {
        CATALOG_POLL
    } else if working {
        WORKING_TICK
    } else {
        TICK
    }
}

/// Whether an agent of the crew is executing or reviewing, so the spinner turns.
fn someone_works(runtime: &RuntimeRead) -> bool {
    matches!(runtime, RuntimeRead::Record(record) if record.crew.iter().any(|member| member.state.is_working()))
}

fn main() -> Result<()> {
    let options = parse_args(std::env::args().skip(1))?;
    let home = config::user_home();
    if let Some(request) = options.skill {
        return print_skill(request, home.as_deref());
    }
    // The configuration is the tool's, so it is read — and its defaults
    // written when it is absent — before the window is drawn, in render mode
    // too. A configuration that cannot be read is a state of the screen, not
    // a reason to fail. The home is kept: a pick is saved under it.
    let config = config::load(home.as_deref());
    let app = App::new(options.theme, options.icons, config, home);

    if options.render {
        return render_plain(options, app);
    }
    // The window starts the orchestrator through herdr when it runs inside
    // herdr. A render never does: a pinned screen is the same in every pane.
    let app = app.with_launcher(launch::from_env());

    let mut terminal = setup()?;
    let result = run(&mut terminal, app);
    restore(&mut terminal)?;
    result
}

/// What the command line asks for.
#[derive(Debug)]
struct Options {
    /// The colours of the window.
    theme: Theme,
    /// The icon set the window draws with.
    icons: IconSet,
    /// Print the screen as text and exit.
    render: bool,
    /// The columns of a plain render.
    width: u16,
    /// The rows of a plain render.
    height: u16,
    /// Print a skill as text and exit, instead of opening the window.
    skill: Option<SkillRequest>,
}

/// Print a skill as text and exit: the list of roles, or one role's skill
/// under the crew in force — the session's, when the orchestrator's tab runs
/// it, else the configuration's. No window is set up.
fn print_skill(request: SkillRequest, home: Option<&Path>) -> Result<()> {
    let text = match request {
        SkillRequest::List => skill::list(),
        SkillRequest::Role(role) => skill::for_role(role, config::session_or_load(home)),
    };
    let mut stdout = std::io::stdout();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

/// The flags of the command line.
fn parse_args(args: impl Iterator<Item = String>) -> Result<Options> {
    let mut args = args.peekable();
    let mut preset = Preset::Dark;
    let mut icons: Option<String> = None;
    let mut render = false;
    let mut plain = false;
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
    let mut skill = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--theme" => {
                let Some(name) = args.next() else {
                    bail!("--theme needs a name\n{USAGE}");
                };
                preset = preset_of(&name)?;
            }
            other if other.starts_with("--theme=") => {
                preset = preset_of(&other["--theme=".len()..])?;
            }
            "--icons" => {
                let Some(name) = args.next() else {
                    bail!("--icons needs a name\n{USAGE}");
                };
                icons = Some(name);
            }
            other if other.starts_with("--icons=") => {
                icons = Some(other["--icons=".len()..].to_string());
            }
            "--render" => render = true,
            "--plain" => plain = true,
            "--width" => {
                let Some(value) = args.next() else {
                    bail!("--width needs a number\n{USAGE}");
                };
                width = dimension("--width", &value)?;
            }
            other if other.starts_with("--width=") => {
                width = dimension("--width", &other["--width=".len()..])?;
            }
            "--height" => {
                let Some(value) = args.next() else {
                    bail!("--height needs a number\n{USAGE}");
                };
                height = dimension("--height", &value)?;
            }
            other if other.starts_with("--height=") => {
                height = dimension("--height", &other["--height=".len()..])?;
            }
            // The role is the next word when it is not a flag: a bare
            // `--skill` lists the roles.
            "--skill" => {
                skill = Some(match args.next_if(|next| !next.starts_with('-')) {
                    Some(name) => SkillRequest::Role(skill::role_of(&name)?),
                    None => SkillRequest::List,
                });
            }
            other if other.starts_with("--skill=") => {
                skill = Some(SkillRequest::Role(skill::role_of(
                    &other["--skill=".len()..],
                )?));
            }
            other if other.starts_with('-') => bail!("unknown option {other}\n{USAGE}"),
            other => {
                bail!("hugo takes no folder: it reads the runtime, not the work\n{other}\n{USAGE}")
            }
        }
    }
    // A skill is text for an agent, not a screen: it is not a render.
    if skill.is_some() && (render || plain) {
        bail!("--skill prints a skill and does not render\n{USAGE}");
    }
    // The two flags are one mode: one without the other would ask for a
    // render that is not plain, or a plain text that is not a render.
    if render != plain {
        bail!("--render and --plain go together\n{USAGE}");
    }
    let icons = IconSet::resolve(icons.as_deref(), None)
        .map_err(|error| anyhow::anyhow!("{error}\n{USAGE}"))?;
    Ok(Options {
        theme: Theme::preset(preset),
        icons,
        render,
        width,
        height,
        skill,
    })
}

/// A positive dimension of a plain render, up to `MAX_DIMENSION`.
fn dimension(flag: &str, value: &str) -> Result<u16> {
    match value.parse::<u16>() {
        Ok(number) if number > 0 && number <= MAX_DIMENSION => Ok(number),
        _ => bail!("{flag} needs a positive number up to {MAX_DIMENSION}, not {value}\n{USAGE}"),
    }
}

/// The preset of a name.
fn preset_of(name: &str) -> Result<Preset> {
    Preset::parse(name).ok_or_else(|| {
        let names: Vec<&str> = Preset::ALL.iter().map(|preset| preset.name()).collect();
        anyhow::anyhow!("unknown theme {name}: the themes are {}", names.join(", "))
    })
}

/// The window loop. The runtime is read again on every redraw, and every
/// redraw happens at least once a tick, so the screen keeps up without a key.
fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut app: App) -> Result<()> {
    let located = locate_runtime();
    app.project = located.label(std::env::current_dir().ok().as_deref());
    app.root = located.root();
    // The frame counter drives the spinner: every redraw is the next frame.
    let mut tick: usize = 0;
    loop {
        let runtime = read_runtime(&located);
        // One clock read per frame, as in raul: the crews' holds and every
        // key that answers the draw see the instant the screen was measured
        // at. No other module under `src/` reads a clock.
        let now = coding_crew_core::now_rfc3339();
        terminal.draw(|frame| app.draw(frame, &runtime, &now, tick))?;
        tick = tick.wrapping_add(1);
        // The catalogue is a harness's, so it is read beside the window: while
        // a read is in flight the loop polls for the answer as well as for a
        // key, so a slow tool never leaves raw mode without a reader. The
        // orchestrator's start is herdr's, and it is polled the same way.
        if app.catalog_pending() && app.poll_choice() {
            continue;
        }
        if app.start_pending() && app.poll_start() {
            continue;
        }
        // A wait that ends with no event redraws: the record is read again.
        let busy = app.catalog_pending() || app.start_pending();
        let deadline = Instant::now() + next_wait(busy, someone_works(&runtime));
        // The wait ends at the tick or at an event that changes the screen. An
        // event that changes nothing, as a pointer moving over the window does,
        // is read and the wait goes on to the same tick.
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || !event::poll(left)? {
                break;
            }
            // The event that woke the loop and every event queued behind it are
            // answered before the next draw: a burst of wheel notches costs one
            // redraw, not one per notch.
            let mut events = vec![event::read()?];
            while event::poll(Duration::ZERO)? {
                events.push(event::read()?);
            }
            let mut changed = false;
            for event in events {
                match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if app.on_key(key) == Action::Quit {
                            return Ok(());
                        }
                        changed = true;
                    }
                    Event::Mouse(mouse) => {
                        changed |= app.on_mouse(mouse) != Action::Idle;
                    }
                    Event::Key(_) => {}
                    // A resize, focus change or paste: the screen is drawn again.
                    _ => changed = true,
                }
            }
            if changed {
                break;
            }
        }
    }
}

/// Print the screen as text and exit.
///
/// No terminal is set up: the screen is drawn into a `TestBackend` buffer,
/// which is converted to plain UTF-8 instead of being painted on a terminal.
/// The contract is raul's, so hugo's screens can be pinned the same way.
///
/// A render draws at a fixed instant, not the wall clock: a pinned screen
/// must be the same bytes on every run.
fn render_plain(options: Options, mut app: App) -> Result<()> {
    let located = locate_runtime();
    app.project = located.label(std::env::current_dir().ok().as_deref());
    let runtime = read_runtime(&located);
    let mut terminal = Terminal::new(TestBackend::new(options.width, options.height))?;
    // A render is the first frame of the spinner, so a pinned screen is the
    // same bytes on every run.
    terminal.draw(|frame| app.draw(frame, &runtime, RENDER_NOW, 0))?;
    let text = plain_text(terminal.backend().buffer());
    let mut stdout = std::io::stdout();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

/// Where the runtime record was looked for.
enum Located {
    /// The record's path, in the nearest project.
    Found(PathBuf),
    /// There is no project: no record to read, not a broken one.
    Absent,
    /// The project could not be determined: that is not the empty state.
    Unreadable(ReadError),
}

impl Located {
    /// The project's root: the folder that holds the record's `coding-crew/`
    /// folder, when a record is found.
    fn root(&self) -> Option<PathBuf> {
        let Located::Found(runtime) = self else {
            return None;
        };
        Some(runtime.parent()?.parent()?.to_path_buf())
    }

    /// The project's folder as the title line names it, when one is found.
    ///
    /// The folder is named as raul names its folder: relative to the working
    /// directory, so a screen does not carry the machine's own paths. The
    /// working directory itself is named by its folder's name, a folder
    /// below it as `./folder`, and anything else by its whole path.
    fn label(&self, cwd: Option<&Path>) -> Option<String> {
        let project = self.root()?;
        let relative = cwd.and_then(|cwd| project.strip_prefix(cwd).ok());
        Some(match relative {
            Some(rest) if rest.as_os_str().is_empty() => {
                project.file_name()?.to_string_lossy().into_owned()
            }
            Some(rest) => format!("./{}", rest.display()),
            None => project.display().to_string(),
        })
    }
}

/// The nearest project's runtime record, discovered from the current
/// directory.
///
/// The project boundary is the store's: the nearest `coding-crew/` folder is
/// the project, and the record sits in it beside the journal. The work folder
/// may be absent; the record is still the project's. `Absent` when there is
/// no project. A discovery failure is not the empty state: it names the
/// record and the error. The record itself may not exist yet: [`read_runtime`]
/// says so, and the loop reads it again on every redraw.
fn locate_runtime() -> Located {
    let current = match std::env::current_dir() {
        Ok(current) => current,
        Err(error) => {
            return Located::Unreadable(ReadError {
                file: RUNTIME_FILE.to_string(),
                error: error.to_string(),
            });
        }
    };
    match Store::discover(&current) {
        Ok(store) => Located::Found(store.runtime_path()),
        Err(StoreError::NoDataFolder | StoreError::NoWorkFolder(_)) => Located::Absent,
        Err(error) => Located::Unreadable(ReadError {
            file: RUNTIME_FILE.to_string(),
            error: error.to_string(),
        }),
    }
}

/// Read the record, when there is one.
///
/// An absent record is the empty state. A record that cannot be read — a
/// file that does not parse, a role the model does not know, a stamp that is
/// missing — is not absent: it names the file and the error, as raul names a
/// file it cannot read. The flow's writer owns the file's shape.
fn read_runtime(located: &Located) -> RuntimeRead {
    let path = match located {
        Located::Found(path) => path,
        Located::Absent => return RuntimeRead::Absent,
        Located::Unreadable(error) => return RuntimeRead::Unreadable(error.clone()),
    };
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(RUNTIME_FILE)
        .to_string();
    match Runtime::read(path) {
        Ok(runtime) => RuntimeRead::Record(runtime),
        Err(RuntimeError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            RuntimeRead::Absent
        }
        Err(error) => RuntimeRead::Unreadable(ReadError {
            file,
            error: error.to_string(),
        }),
    }
}

/// The configuration home's path, for the tests that hand hugo a home.
#[cfg(test)]
fn config_home() -> Option<std::path::PathBuf> {
    config::user_home().map(|home| config::home_at(&home))
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_crew_core::FlowRole;
    use coding_crew_core::runtime::{AgentState, CrewMember};

    #[test]
    fn the_theme_is_the_dark_one_by_default() {
        let options = parse_args(std::iter::empty()).expect("the options");
        assert_eq!(options.theme, Theme::preset(Preset::Dark));
    }

    #[test]
    fn the_theme_is_chosen_with_a_flag() {
        for (name, preset) in [
            ("nord", Preset::Nord),
            ("catppuccin", Preset::Catppuccin),
            ("dracula", Preset::Dracula),
            ("terminal", Preset::Terminal),
        ] {
            for args in [
                vec!["--theme".to_string(), name.to_string()],
                vec![format!("--theme={name}")],
            ] {
                let options = parse_args(args.into_iter()).expect("the options");
                assert_eq!(options.theme, Theme::preset(preset), "{name}");
            }
        }
    }

    #[test]
    fn an_unknown_theme_is_refused() {
        let error = parse_args(["--theme".to_string(), "light".to_string()].into_iter())
            .expect_err("light is not a theme yet");
        assert!(error.to_string().contains("light"), "{error}");
        let error =
            parse_args(["--theme".to_string()].into_iter()).expect_err("a theme needs a name");
        assert!(error.to_string().contains("--theme"), "{error}");
    }

    #[test]
    fn the_icon_set_is_chosen_with_a_flag() {
        for (name, set) in [("nerd", IconSet::Nerd), ("unicode", IconSet::Unicode)] {
            for args in [
                vec!["--icons".to_string(), name.to_string()],
                vec![format!("--icons={name}")],
            ] {
                let options = parse_args(args.into_iter()).expect("the options");
                assert_eq!(options.icons, set, "{name}");
            }
        }
    }

    #[test]
    fn an_unknown_icon_set_is_refused() {
        let error = parse_args(["--icons".to_string(), "emoji".to_string()].into_iter())
            .expect_err("emoji is not an icon set");
        assert!(error.to_string().contains("emoji"), "{error}");
        assert!(error.to_string().contains("--icons"), "{error}");
        let error =
            parse_args(["--icons".to_string()].into_iter()).expect_err("an icon set needs a name");
        assert!(error.to_string().contains("--icons"), "{error}");
    }

    #[test]
    fn the_render_mode_is_two_flags_in_either_order() {
        for args in [["--render", "--plain"], ["--plain", "--render"]] {
            let options = parse_args(args.map(String::from).into_iter())
                .unwrap_or_else(|error| panic!("{args:?}: {error}"));
            assert!(options.render, "{args:?}");
        }
    }

    #[test]
    fn either_render_flag_alone_is_refused() {
        for flag in ["--render", "--plain"] {
            let error = parse_args([flag.to_string()].into_iter())
                .expect_err("a render flag alone is not a mode");
            assert!(error.to_string().contains("go together"), "{error}");
        }
    }

    #[test]
    fn the_plain_render_is_100_by_40_by_default() {
        let options =
            parse_args(["--render", "--plain"].map(String::from).into_iter()).expect("the options");
        assert_eq!((options.width, options.height), (100, 40));
    }

    #[test]
    fn the_plain_render_takes_a_width_and_a_height() {
        for args in [
            vec!["--render", "--plain", "--width", "60", "--height", "20"],
            vec!["--plain", "--width=60", "--render", "--height=20"],
        ] {
            let options = parse_args(args.into_iter().map(String::from)).expect("the options");
            assert_eq!((options.width, options.height), (60, 20));
        }
    }

    #[test]
    fn a_dimension_is_capped_at_1000() {
        let width = parse_args(
            ["--render", "--plain", "--width", "1000"]
                .map(String::from)
                .into_iter(),
        )
        .expect("1000 columns");
        assert_eq!((width.width, width.height), (1000, DEFAULT_HEIGHT));
        for flag in ["--width", "--height"] {
            for value in ["1001", "65535"] {
                let error = parse_args(
                    ["--render", "--plain", flag, value]
                        .map(String::from)
                        .into_iter(),
                )
                .expect_err("a dimension over the cap is refused");
                assert!(error.to_string().contains(flag), "{error}");
                assert!(error.to_string().contains("1000"), "{error}");
            }
        }
    }

    #[test]
    fn a_dimension_must_be_a_positive_number() {
        for (flag, value) in [("--width", "0"), ("--height", "0"), ("--width", "wide")] {
            let error = parse_args(
                ["--render", "--plain", flag, value]
                    .map(String::from)
                    .into_iter(),
            )
            .expect_err("a dimension must be positive");
            assert!(error.to_string().contains(flag), "{error}");
            assert!(error.to_string().contains("positive"), "{error}");
        }
        let error = parse_args(
            ["--render", "--plain", "--width"]
                .map(String::from)
                .into_iter(),
        )
        .expect_err("a dimension needs a value");
        assert!(error.to_string().contains("--width"), "{error}");
    }

    #[test]
    fn hugo_takes_no_folder() {
        // The seam: hugo reads the runtime, not the project's work folder.
        let error = parse_args(["mock-data/coding-crew/work"].map(String::from).into_iter())
            .expect_err("hugo has no folder argument");
        assert!(error.to_string().contains("runtime"), "{error}");
    }

    #[test]
    fn the_config_home_is_the_users_own() {
        // A machine without a home has no configuration home to name, which
        // `ensure_home` already allows.
        let Some(home) = config::user_home() else {
            return;
        };
        assert_eq!(
            config_home(),
            Some(home.join(".config").join("coding-crew"))
        );
    }

    /// A scratch folder for the reader's tests.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hugo-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        dir
    }

    #[test]
    fn an_absent_record_is_not_an_unreadable_one() {
        let dir = scratch("runtime-absent");
        let path = dir.join("runtime.json");
        // No file: the empty state.
        assert_eq!(
            read_runtime(&Located::Found(path.clone())),
            RuntimeRead::Absent
        );
        // A file that does not parse: reported, with its name and error.
        std::fs::write(&path, "{ not json").expect("a broken record");
        match read_runtime(&Located::Found(path)) {
            RuntimeRead::Unreadable(error) => {
                assert_eq!(error.file, "runtime.json");
                assert!(error.error.contains("parse"), "{}", error.error);
            }
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_stamp_is_unreadable_not_no_run() {
        // The stamp is required: a record without one is not a run that is
        // silent, it is a record that cannot be read.
        let dir = scratch("runtime-no-stamp");
        let path = dir.join("runtime.json");
        std::fs::write(&path, r#"{"started_at": "2026-11-19T09:00:00Z"}"#)
            .expect("a record without its stamp");
        match read_runtime(&Located::Found(path)) {
            RuntimeRead::Unreadable(error) => {
                assert_eq!(error.file, "runtime.json");
                assert!(error.error.contains("parse"), "{}", error.error);
            }
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_loop_redraws_on_a_tick_when_no_catalogue_is_read() {
        // Without a tick the window waits on a key, and the record and the
        // holds it shows go stale. A catalogue read is polled faster.
        assert_eq!(next_wait(false, false), TICK);
        assert!(next_wait(true, false) < next_wait(false, false));
    }

    #[test]
    fn the_spinner_turns_faster_while_an_agent_works() {
        // An agent that works turns the spinner at its own pace; an idle crew
        // keeps the slow tick, so the loop costs little CPU when nothing runs.
        assert_eq!(next_wait(false, true), WORKING_TICK);
        assert!(next_wait(false, true) < next_wait(false, false));
        assert_eq!(next_wait(true, true), CATALOG_POLL);
    }

    #[test]
    fn only_a_working_agent_turns_the_spinner() {
        let member = |state| CrewMember {
            role: FlowRole::Builder,
            model: "m".to_string(),
            actor: None,
            holds: None,
            since: None,
            state,
        };
        let record = |crew: Vec<CrewMember>| {
            RuntimeRead::Record(Runtime {
                started_at: "2026-11-09T08:30Z".to_string(),
                crew,
                queue: Vec::new(),
                round: None,
                budget: Default::default(),
                findings: 0,
                severities: None,
                stamp: Default::default(),
            })
        };
        assert!(!someone_works(&record(vec![member(AgentState::Idle)])));
        assert!(someone_works(&record(vec![
            member(AgentState::Idle),
            member(AgentState::Reviewing),
        ])));
        assert!(!someone_works(&RuntimeRead::Absent));
    }

    #[test]
    fn a_discovery_failure_is_not_the_empty_state() {
        let read = read_runtime(&Located::Unreadable(ReadError {
            file: "runtime.json".to_string(),
            error: "the project could not be found".to_string(),
        }));
        assert!(matches!(&read, RuntimeRead::Unreadable(_)), "{read:?}");
    }

    #[test]
    fn only_the_frame_reads_the_clock_and_a_render_is_fixed() {
        // One instant per frame, and no other clock read under `src/`: a
        // module that reads its own clock can disagree with the frame it is
        // drawn in. The render mode draws at the fixed instant the doc names.
        let clock = ["now", "_rfc3339"].concat();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut readers: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("the source folder") {
            let path = entry.expect("a source entry").path();
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a source file");
            if source.contains(&clock) {
                readers.push(
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .expect("a file name")
                        .to_string(),
                );
            }
        }
        readers.sort();
        assert_eq!(
            readers,
            vec!["main.rs".to_string()],
            "a module outside the frame reads the clock: {readers:?}"
        );
        let main = std::fs::read_to_string(dir.join("main.rs")).expect("main.rs");
        assert_eq!(
            main.matches(&clock).count(),
            1,
            "the loop is the one clock read; a render is fixed"
        );
        let doc = include_str!("../../../docs/hugo/screens.md");
        assert!(
            doc.contains(RENDER_NOW),
            "the doc does not name the instant"
        );
    }
}
