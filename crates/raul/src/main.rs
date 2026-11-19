//! raul — the TUI of coding-lanes.
//!
//! raul reads a work folder and shows it. It writes nothing.

mod app;
mod backlog;
mod data;
mod detail;
mod needs;
mod overview;
mod path;
mod settings;
mod tabs;
mod text;
mod theme;
mod view;

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
use coding_lanes_core::{Store, StoreError};
use data::{Missing, Work};
use detail::Detail;
use tabs::Tab;
use theme::{Preset, Theme};

const USAGE: &str = "usage: raul [--theme dark|nord|catppuccin|dracula|terminal] [--render --plain [--width N] [--height N] [--tab NAME] [--detail TASK-ID]] [work-folder]";
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
    /// Why the folder is not there, when it is not.
    reason: Missing,
    /// The colours of the window.
    theme: Theme,
    /// Print the screen as text and exit.
    render: bool,
    /// The columns of a plain render.
    width: u16,
    /// The rows of a plain render.
    height: u16,
    /// The tab a plain render shows.
    tab: Tab,
    /// The record the detail pane shows, in render mode.
    detail: Option<String>,
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
    let mut tab: Option<Tab> = None;
    let mut detail: Option<String> = None;
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
            "--tab" => {
                let Some(name) = args.next() else {
                    bail!("--tab needs a name\n{USAGE}");
                };
                tab = Some(tab_of(&name)?);
            }
            other if other.starts_with("--tab=") => {
                tab = Some(tab_of(&other["--tab=".len()..])?);
            }
            "--detail" => {
                let Some(id) = args.next() else {
                    bail!("--detail needs a task id\n{USAGE}");
                };
                detail = Some(id);
            }
            other if other.starts_with("--detail=") => {
                detail = Some(other["--detail=".len()..].to_string());
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
    // The detail pane is reachable deterministically through the plain
    // render: the flag has nothing to change in the interactive window.
    if detail.is_some() && !render {
        bail!("--detail works with --render --plain\n{USAGE}");
    }
    if let Some(id) = &detail
        && id.trim().is_empty()
    {
        bail!("--detail needs a task id\n{USAGE}");
    }
    // The tab is reachable deterministically through the plain render: the
    // flag has nothing to change in the interactive window, where the keys
    // switch tabs.
    if tab.is_some() && !render {
        bail!("--tab works with --render --plain\n{USAGE}");
    }
    // The detail pane covers the whole window, so a tab under it is a state
    // the keys cannot reach: the pair is refused, never silently rendered.
    if tab.is_some() && detail.is_some() {
        bail!("--tab and --detail cannot be combined: the detail pane covers the tab\n{USAGE}");
    }
    let (root, reason) = match root {
        Some(root) => {
            let reason = reason_at(&root);
            (root, reason)
        }
        None => locate_current()?,
    };
    Ok(Options {
        root,
        reason,
        theme: Theme::preset(preset),
        render,
        width,
        height,
        tab: tab.unwrap_or(Tab::Overview),
        detail,
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

/// The tab of a name.
fn tab_of(name: &str) -> Result<Tab> {
    Tab::from_name(name).ok_or_else(|| {
        let names: Vec<&str> = Tab::ALL.iter().map(|tab| tab.name()).collect();
        anyhow::anyhow!("unknown tab {name}: the tabs are {}", names.join(", "))
    })
}

/// The work folder of the project in the current directory, and why it is
/// absent when it is.
fn locate_current() -> Result<(PathBuf, Missing)> {
    let current = std::env::current_dir()
        .map_err(|error| anyhow::anyhow!("reading the current directory: {error}"))?;
    locate(&current)
}

/// The work folder of the nearest project at or above `start`, and why it is
/// absent when it is.
///
/// The rule is the store's. When the nearest project has no `work/`, its own
/// path is the one shown. With no `coding-lanes/` anywhere, the path looked
/// for is `start/coding-lanes/work`.
fn locate(start: &Path) -> Result<(PathBuf, Missing)> {
    located(start, Store::discover_work(start))
}

/// The folder and the reason, from the result of the store's discovery.
fn located(start: &Path, found: Result<Store, StoreError>) -> Result<(PathBuf, Missing)> {
    match found {
        Ok(store) => Ok((store.root().to_path_buf(), Missing::NoWork)),
        Err(StoreError::NoWorkFolder(data)) => Ok((data.join(WORK_DIR), Missing::NoWork)),
        Err(StoreError::NoDataFolder) => {
            Ok((start.join(DATA_DIR).join(WORK_DIR), Missing::NoFolder))
        }
        // Discovery returns only the two errors above. Any other error is
        // not a missing folder, and is not swallowed as one.
        Err(other) => Err(anyhow::anyhow!("finding the work folder: {other}")),
    }
}

/// Why a work folder given on the command line is not there.
fn reason_at(root: &Path) -> Missing {
    match root.parent() {
        Some(data)
            if data.file_name().and_then(|name| name.to_str()) == Some(DATA_DIR)
                && data.is_dir() =>
        {
            Missing::NoWork
        }
        _ => Missing::NoFolder,
    }
}

/// The window loop. The folder is read again on every redraw.
fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, options: Options) -> Result<()> {
    let reason = options.reason;
    let mut app = App::new(options.root, options.theme);
    loop {
        let work = Work::load_missing(&app.root, reason);
        // One clock read per frame: the draw and the key that answers it
        // both see the instant the rows on display were ordered by.
        let now = coding_lanes_core::now_rfc3339();
        terminal.draw(|frame| app.draw(frame, &work, &now))?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && app.on_key(key, &work, &now) == Action::Quit
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
///
/// With `--detail`, the screen is the detail pane over the whole window,
/// bounded by the same dimensions.
fn render_plain(options: Options) -> Result<()> {
    let mut app = App::new(options.root, options.theme);
    app.tab = options.tab;
    let work = Work::load_missing(&app.root, options.reason);
    if let Some(id) = &options.detail {
        if work.record_title(id).is_none() {
            bail!("--detail: no record {id} in {}", app.root.display());
        }
        app.detail = Some(Detail::opened(id.clone(), app.tab, None));
    }
    // The plain render is one frame too: one instant, read once.
    let now = coding_lanes_core::now_rfc3339();
    let mut terminal = Terminal::new(TestBackend::new(options.width, options.height))?;
    terminal.draw(|frame| app.draw(frame, &work, &now))?;
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
        // A coding-lanes folder without a work folder is the project, and its
        // own work folder is the one reported missing.
        fs::create_dir_all(dir.join(DATA_DIR)).expect("a data folder");
        assert_eq!(
            locate(&dir).expect("a location"),
            (dir.join(DATA_DIR).join(WORK_DIR), Missing::NoWork)
        );
        fs::create_dir_all(dir.join(DATA_DIR).join(WORK_DIR)).expect("a work folder");
        assert_eq!(
            locate(&dir).expect("a location"),
            (dir.join(DATA_DIR).join(WORK_DIR), Missing::NoWork)
        );
    }

    #[test]
    fn the_discovery_walks_up() {
        let dir = scratch_dir("discovery-up");
        fs::create_dir_all(dir.join(DATA_DIR).join(WORK_DIR)).expect("a work folder");
        let deep = dir.join("crates").join("raul");
        fs::create_dir_all(&deep).expect("a nested folder");
        assert_eq!(
            locate(&deep).expect("a location"),
            (dir.join(DATA_DIR).join(WORK_DIR), Missing::NoWork)
        );
    }

    #[test]
    fn a_project_without_work_does_not_borrow_an_ancestor() {
        let ancestor = scratch_dir("discovery-ancestor");
        fs::create_dir_all(ancestor.join(DATA_DIR).join(WORK_DIR)).expect("the ancestor's work");
        let project = ancestor.join("project");
        fs::create_dir_all(project.join(DATA_DIR)).expect("a data folder without work");
        let deep = project.join("src");
        fs::create_dir_all(&deep).expect("a nested folder");
        assert_eq!(
            locate(&deep).expect("a location"),
            (project.join(DATA_DIR).join(WORK_DIR), Missing::NoWork),
            "the nearest project's work folder is the one shown as missing"
        );
    }

    #[test]
    fn without_a_project_the_folder_looked_for_is_named() {
        // The folder is built from the start, without a walk: a coding-lanes
        // folder above the temporary directory cannot change it.
        let start = Path::new("/nowhere/project/src");
        assert_eq!(
            located(start, Err(StoreError::NoDataFolder)).expect("a location"),
            (start.join(DATA_DIR).join(WORK_DIR), Missing::NoFolder)
        );
    }

    #[test]
    fn a_missing_project_folder_is_named_by_the_discovery() {
        let data = PathBuf::from("/nowhere/project/coding-lanes");
        assert_eq!(
            located(
                Path::new("/nowhere/project/src"),
                Err(StoreError::NoWorkFolder(data.clone()))
            )
            .expect("a location"),
            (data.join(WORK_DIR), Missing::NoWork)
        );
    }

    #[test]
    fn any_other_discovery_error_is_not_a_missing_folder() {
        let error = located(
            Path::new("/nowhere/project/src"),
            Err(StoreError::Item("a broken store".into())),
        )
        .expect_err("not a missing folder");
        assert!(error.to_string().contains("a broken store"), "{error}");
    }

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

    #[test]
    fn the_detail_flag_names_the_record() {
        for args in [
            vec!["--render", "--plain", "--detail", "TSK13"],
            vec!["--plain", "--detail=TSK13", "--render"],
        ] {
            let options = parse_args(args.into_iter().map(String::from)).expect("the options");
            assert_eq!(options.detail.as_deref(), Some("TSK13"));
        }
    }

    #[test]
    fn the_detail_flag_needs_the_render_mode() {
        let error = parse_args(["--detail", "TSK13"].map(String::from).into_iter())
            .expect_err("the flag is a render-mode flag");
        assert!(error.to_string().contains("--detail"), "{error}");
        assert!(error.to_string().contains("--render"), "{error}");
    }

    #[test]
    fn the_detail_flag_needs_an_id() {
        for args in [
            vec!["--render", "--plain", "--detail"],
            vec!["--render", "--plain", "--detail="],
            vec!["--render", "--plain", "--detail", " "],
        ] {
            let error = parse_args(args.into_iter().map(String::from))
                .expect_err("the flag needs a task id");
            assert!(error.to_string().contains("--detail"), "{error}");
        }
    }

    #[test]
    fn the_default_render_has_no_detail() {
        let options =
            parse_args(["--render", "--plain"].map(String::from).into_iter()).expect("the options");
        assert!(options.detail.is_none());
    }

    #[test]
    fn the_tab_flag_names_the_tab() {
        let settings = parse_args(
            ["--render", "--plain", "--tab", "settings"]
                .map(String::from)
                .into_iter(),
        )
        .expect("the settings tab");
        assert_eq!(settings.tab, Tab::Settings);
        let needs_you = parse_args(
            ["--plain", "--tab=needs-you", "--render"]
                .map(String::from)
                .into_iter(),
        )
        .expect("the needs-you tab");
        assert_eq!(needs_you.tab, Tab::NeedsYou);
    }

    #[test]
    fn the_render_starts_on_the_overview() {
        let options =
            parse_args(["--render", "--plain"].map(String::from).into_iter()).expect("the options");
        assert_eq!(options.tab, Tab::Overview);
    }

    #[test]
    fn an_unknown_tab_is_refused() {
        let error = parse_args(
            ["--render", "--plain", "--tab", "settingss"]
                .map(String::from)
                .into_iter(),
        )
        .expect_err("settingss is not a tab");
        assert!(error.to_string().contains("settingss"), "{error}");
        assert!(error.to_string().contains("settings"), "{error}");
        let error = parse_args(
            ["--render", "--plain", "--tab"]
                .map(String::from)
                .into_iter(),
        )
        .expect_err("a tab needs a name");
        assert!(error.to_string().contains("--tab"), "{error}");
    }

    #[test]
    fn the_tab_flag_needs_the_render_mode() {
        let error = parse_args(["--tab", "settings"].map(String::from).into_iter())
            .expect_err("the flag is a render-mode flag");
        assert!(error.to_string().contains("--tab"), "{error}");
        assert!(error.to_string().contains("--render"), "{error}");
    }

    #[test]
    fn the_tab_and_detail_flags_are_refused_together() {
        // The pane covers the whole window: the tab under it is a state the
        // keys cannot reach, so the pair is refused rather than rendered.
        for args in [
            vec!["--render", "--plain", "--tab", "path", "--detail", "TSK13"],
            vec!["--render", "--plain", "--tab=settings", "--detail=TSK13"],
        ] {
            let error = parse_args(args.into_iter().map(String::from))
                .expect_err("the pair has no reachable state");
            let text = error.to_string();
            assert!(text.contains("--tab"), "{text}");
            assert!(text.contains("--detail"), "{text}");
            assert!(text.contains("combined"), "{text}");
        }
    }

    #[test]
    fn only_the_frame_reads_the_clock() {
        // One instant per frame, and no other clock read in the crate: a tab
        // that reads its own clock can disagree with the frame it is drawn
        // in. The two frames — the interactive loop and the plain render —
        // are the only readers; every other module takes the instant it is
        // given.
        let clock = ["now", "_rfc3339"].concat();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut readers: Vec<String> = Vec::new();
        for entry in fs::read_dir(&dir).expect("the source folder") {
            let path = entry.expect("a source entry").path();
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let source = fs::read_to_string(&path).expect("a source file");
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
        let main = fs::read_to_string(dir.join("main.rs")).expect("main.rs");
        assert_eq!(
            main.matches(&clock).count(),
            2,
            "the clock is read once per frame: the loop and the plain render"
        );
    }
}
