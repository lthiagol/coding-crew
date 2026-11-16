//! raul — the TUI of coding-lanes.
//!
//! raul reads a work folder and shows it. It writes nothing.

mod app;
mod data;
mod needs;
mod overview;
mod path;
mod tabs;
mod theme;

use std::io::{Stdout, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui::buffer::{Buffer, CellWidth};

use app::{Action, App};
use coding_lanes_core::store::{DATA_DIR, WORK_DIR};
use data::Work;
use theme::{Preset, Theme};

const USAGE: &str =
    "usage: raul [--theme dark|terminal] [--render --plain [--width N] [--height N]] [work-folder]";
/// The columns of a plain render when --width is not given.
const DEFAULT_WIDTH: u16 = 100;
/// The rows of a plain render when --height is not given.
const DEFAULT_HEIGHT: u16 = 40;
/// The largest width or height a plain render accepts. The buffer is
/// `width × height` cells, so an unbounded pair can ask for more memory than
/// the machine has and abort.
const MAX_DIMENSION: u16 = 1000;

fn main() -> Result<()> {
    let options = parse_args(std::env::args().skip(1))?;

    if options.render {
        return render_plain(options);
    }

    let mut terminal = setup()?;
    let result = run(&mut terminal, options);
    restore(&mut terminal)?;
    result
}

/// What the command line asks for.
#[derive(Debug)]
struct Options {
    /// The work folder to read.
    root: PathBuf,
    /// The colours of the window.
    theme: Theme,
    /// Print the screen as text and exit.
    render: bool,
    /// The columns of a plain render.
    width: u16,
    /// The rows of a plain render.
    height: u16,
}

/// The work folder to read and the colours to paint it with.
///
/// The folder is given on the command line, or discovered from the current
/// directory: the store is discovered, not configured. The theme is chosen
/// with a flag.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Options> {
    let mut root: Option<PathBuf> = None;
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
            other => root = Some(PathBuf::from(other)),
        }
    }
    // The two flags are one mode: one without the other would ask for a
    // render that is not plain, or a plain text that is not a render.
    if render != plain {
        bail!("--render and --plain go together\n{USAGE}");
    }
    Ok(Options {
        root: root.unwrap_or_else(discover),
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

/// The work folder of the project in the current directory.
///
/// The store is discovered, not configured: the nearest ancestor that holds
/// a `coding-lanes/work` folder is the one. A `coding-lanes` folder without
/// a `work` folder is not a store, so the search walks on. When there is no
/// store at all, the folder that was looked for is returned — raul then says
/// so instead of reading the wrong place.
fn discover() -> PathBuf {
    let Ok(current) = std::env::current_dir() else {
        return PathBuf::new();
    };
    discover_from(&current).unwrap_or_else(|| current.join(DATA_DIR).join(WORK_DIR))
}

/// The nearest ancestor of `start` that holds a work folder.
fn discover_from(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start.to_path_buf());
    while let Some(current) = dir {
        let candidate = current.join(DATA_DIR).join(WORK_DIR);
        if candidate.is_dir() {
            return Some(candidate);
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    None
}

/// The window loop. The folder is read again on every redraw.
fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, options: Options) -> Result<()> {
    let mut app = App::new(options.root, options.theme);
    loop {
        let work = Work::load(&app.root);
        terminal.draw(|frame| app.draw(frame, &work))?;

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
fn render_plain(options: Options) -> Result<()> {
    let app = App::new(options.root, options.theme);
    let work = Work::load(&app.root);
    let mut terminal = Terminal::new(TestBackend::new(options.width, options.height))?;
    terminal.draw(|frame| app.draw(frame, &work))?;
    let text = plain_text(terminal.backend().buffer());
    let mut stdout = std::io::stdout();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

/// The buffer as plain text: one row per line, no paint, and no trailing
/// spaces that carry no information.
///
/// A symbol that takes more than one column is emitted once, and the
/// continuation cells ratatui reset behind it are skipped: the terminal gives
/// the symbol its columns already.
///
/// Every row ends with a newline, so the output has a stable final newline.
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("raul-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        dir
    }

    #[test]
    fn the_discovery_needs_a_work_folder() {
        let dir = scratch_dir("discovery");
        // A coding-lanes folder without a work folder is not a store.
        fs::create_dir_all(dir.join(DATA_DIR)).expect("a data folder");
        assert_eq!(discover_from(&dir), None);
        fs::create_dir_all(dir.join(DATA_DIR).join(WORK_DIR)).expect("a work folder");
        assert_eq!(discover_from(&dir), Some(dir.join(DATA_DIR).join(WORK_DIR)));
    }

    #[test]
    fn the_discovery_walks_up() {
        let dir = scratch_dir("discovery-up");
        fs::create_dir_all(dir.join(DATA_DIR).join(WORK_DIR)).expect("a work folder");
        let deep = dir.join("crates").join("raul");
        fs::create_dir_all(&deep).expect("a nested folder");
        assert_eq!(
            discover_from(&deep),
            Some(dir.join(DATA_DIR).join(WORK_DIR))
        );
    }

    #[test]
    fn without_a_store_the_folder_that_was_looked_for_is_used() {
        let dir = scratch_dir("discovery-none");
        assert_eq!(discover_from(&dir), None, "there is no work folder here");
    }

    #[test]
    fn the_theme_is_the_dark_one_by_default() {
        let options = parse_args(std::iter::empty()).expect("the options");
        assert_eq!(options.theme, Theme::preset(Preset::Dark));
    }

    #[test]
    fn the_theme_is_chosen_with_a_flag() {
        for args in [
            vec!["--theme".to_string(), "terminal".to_string()],
            vec!["--theme=terminal".to_string()],
        ] {
            let options = parse_args(args.into_iter()).expect("the options");
            assert_eq!(options.theme, Theme::preset(Preset::Terminal));
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
        let height = parse_args(
            ["--render", "--plain", "--height", "1000"]
                .map(String::from)
                .into_iter(),
        )
        .expect("1000 rows");
        assert_eq!((height.width, height.height), (DEFAULT_WIDTH, 1000));
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
}
