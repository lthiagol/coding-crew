//! hugo — the TUI for the flow.
//!
//! hugo reads the runtime and shows it. It does not drive the loop, and it
//! writes nothing but its own configuration home.

mod app;
mod config;
mod sidebar;

use std::io::{Stdout, Write};
use std::path::PathBuf;

use anyhow::{Result, bail};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui::buffer::{Buffer, CellWidth};

use app::{Action, App, RENDER_NOW, ReadError, RuntimeRead};
use coding_crew_core::runtime::{Runtime, RuntimeError};
use coding_crew_core::store::RUNTIME_FILE;
use coding_crew_core::{Store, StoreError};
use coding_crew_theme::{Preset, Theme};
use config::ConfigRead;

const USAGE: &str = "usage: hugo [--theme dark|nord|catppuccin|dracula|terminal] [--render --plain [--width N] [--height N]]";
/// The columns of a plain render when --width is not given.
const DEFAULT_WIDTH: u16 = 100;
/// The rows of a plain render when --height is not given.
const DEFAULT_HEIGHT: u16 = 40;
/// The largest width or height a plain render accepts, as in raul: the
/// buffer is `width × height` cells, and an unbounded pair can ask for more
/// memory than the machine has.
const MAX_DIMENSION: u16 = 1000;

fn main() -> Result<()> {
    let options = parse_args(std::env::args().skip(1))?;
    // The configuration is the tool's, so it is read — and its defaults
    // written when it is absent — before the window is drawn, in render mode
    // too. A configuration that cannot be read is a state of the screen, not
    // a reason to fail.
    let config = config::load_home();

    if options.render {
        return render_plain(options, &config);
    }

    let mut terminal = setup()?;
    let result = run(&mut terminal, options, &config);
    restore(&mut terminal)?;
    result
}

/// What the command line asks for.
#[derive(Debug)]
struct Options {
    /// The colours of the window.
    theme: Theme,
    /// Print the screen as text and exit.
    render: bool,
    /// The columns of a plain render.
    width: u16,
    /// The rows of a plain render.
    height: u16,
}

/// The flags of the command line.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Options> {
    let mut preset = Preset::Dark;
    let mut render = false;
    let mut plain = false;
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
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
            other if other.starts_with('-') => bail!("unknown option {other}\n{USAGE}"),
            other => {
                bail!("hugo takes no folder: it reads the runtime, not the work\n{other}\n{USAGE}")
            }
        }
    }
    // The two flags are one mode: one without the other would ask for a
    // render that is not plain, or a plain text that is not a render.
    if render != plain {
        bail!("--render and --plain go together\n{USAGE}");
    }
    Ok(Options {
        theme: Theme::preset(preset),
        render,
        width,
        height,
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

/// The window loop. The runtime is read again on every redraw.
fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    options: Options,
    config: &ConfigRead,
) -> Result<()> {
    let app = App::new(options.theme);
    let located = locate_runtime();
    loop {
        let runtime = read_runtime(&located);
        // One clock read per frame, as in raul: the crews' holds and every
        // key that answers the draw see the instant the screen was measured
        // at. No other module under `src/` reads a clock.
        let now = coding_crew_core::now_rfc3339();
        terminal.draw(|frame| app.draw(frame, &runtime, config, &now))?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && app.on_key(key) == Action::Quit
        {
            break;
        }
    }
    Ok(())
}

/// Print the screen as text and exit.
///
/// No terminal is set up: the screen is drawn into a `TestBackend` buffer,
/// which is converted to plain UTF-8 instead of being painted on a terminal.
/// The contract is raul's, so hugo's screens can be pinned the same way.
///
/// A render draws at a fixed instant, not the wall clock: a pinned screen
/// must be the same bytes on every run.
fn render_plain(options: Options, config: &ConfigRead) -> Result<()> {
    let app = App::new(options.theme);
    let located = locate_runtime();
    let runtime = read_runtime(&located);
    let mut terminal = Terminal::new(TestBackend::new(options.width, options.height))?;
    terminal.draw(|frame| app.draw(frame, &runtime, config, RENDER_NOW))?;
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

/// The buffer as plain text: one row per line, no paint, and no trailing
/// spaces that carry no information.
///
/// A symbol that takes more than one column is emitted once, and the
/// continuation cells ratatui reset behind it are skipped: the terminal gives
/// the symbol its columns already. Every row ends with a newline, so the
/// output has a stable final newline.
///
/// This is raul's conversion, kept here as the same contract: the two
/// binaries share the screen format, not their code.
fn plain_text(buffer: &Buffer) -> String {
    let area = *buffer.area();
    let mut text = String::new();
    for y in 0..area.height {
        let mut row = String::new();
        let mut skip = 0u16;
        for x in 0..area.width {
            let Some(cell) = buffer.cell((x, y)) else {
                continue;
            };
            if skip == 0 {
                row.push_str(cell.symbol());
            }
            skip = skip.max(cell.cell_width()).saturating_sub(1);
        }
        text.push_str(row.trim_end_matches(' '));
        text.push('\n');
    }
    text
}

fn setup() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// The configuration home's path, for the tests that hand hugo a home.
#[cfg(test)]
fn config_home() -> Option<std::path::PathBuf> {
    config::user_home().map(|home| config::home_at(&home))
}

#[cfg(test)]
mod tests {
    use super::*;

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
