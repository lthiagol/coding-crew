//! `raul --render --plain` through the real binary.
//!
//! The command runs from the repository root with the work folder as a
//! relative path, so the Overview's title line — which names the folder it
//! reads — is the same on every machine.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

use coding_crew_core::config::{CONFIG_FILE, Config, config_path, home_at};
use coding_crew_theme::chrome::SPINNER;
use coding_crew_theme::{Icon, IconSet};

mod support;

/// The work folder, relative to the repository root.
const WORK: &str = "mock-data/coding-crew/work";

/// The variable that names the icon set when no flag does.
const ICONS_ENV: &str = "CODING_CREW_ICONS";

/// A fresh project folder holding a `work` folder with the duplicate-id
/// files, written in `order`. The project folder is the working directory of
/// a render, so every case names the same relative work folder.
fn duplicate_id_project(tag: &str, order: &[usize]) -> PathBuf {
    let project = std::env::temp_dir().join(format!("raul-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&project);
    let dir = project.join("work");
    fs::create_dir_all(&dir).expect("a work folder");
    for index in order {
        let (name, contents) = support::DUPLICATE_ID_FILES[*index];
        fs::write(dir.join(name), contents).expect("writing a duplicate-id file");
    }
    project
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// The committed fixture file for a task id.
///
/// The title drives the file name, so a test that needs one task finds it by
/// the id prefix rather than by a slug the generator rewrites.
fn fixture_path(id: &str) -> PathBuf {
    let dir = repo_root().join(WORK);
    let prefix = format!("{id}-");
    let mut matches: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("the work folder")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".json"))
        })
        .collect();
    matches.sort();
    assert_eq!(matches.len(), 1, "{id} names {} fixtures", matches.len());
    matches.pop().expect("the fixture")
}

fn run(args: &[&str]) -> Output {
    run_in(&repo_root(), args)
}

fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_raul"))
        .current_dir(dir)
        // The variable names a set only when no flag does: a developer's own
        // choice must not change a snapshot that is recorded in nerd.
        .env_remove(ICONS_ENV)
        .env("HOME", render_home())
        .args(args)
        .output()
        .expect("running raul")
}

/// The home folder the renders read their configuration from. A developer's
/// own file must not change a snapshot, so the renders never read `$HOME`.
fn render_home() -> PathBuf {
    let home = std::env::temp_dir().join(format!("raul-render-home-{}", std::process::id()));
    fs::create_dir_all(&home).expect("a scratch home");
    home
}

/// Every ordering of the indices `0..count`.
fn permutations(count: usize) -> Vec<Vec<usize>> {
    fn walk(next: usize, current: &mut Vec<usize>, orders: &mut Vec<Vec<usize>>) {
        if next == current.len() {
            orders.push(current.clone());
            return;
        }
        for index in next..current.len() {
            current.swap(next, index);
            walk(next + 1, current, orders);
            current.swap(next, index);
        }
    }
    let mut orders = Vec::new();
    let mut current: Vec<usize> = (0..count).collect();
    walk(0, &mut current, &mut orders);
    orders
}

/// The Overview at the default size, as the snapshot records it.
fn overview() -> Output {
    run(&[
        "--render", "--plain", "--width", "100", "--height", "40", WORK,
    ])
}

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name)
}

/// The terminal columns a line takes, by the one table both measures read.
fn display_width(text: &str) -> usize {
    column_table().width_of(text)
}

/// The one table of terminal-column rules, read by the sweep and by this
/// check. `scripts/terminal-columns.txt` is the source of truth.
fn column_table() -> &'static ColumnTable {
    static TABLE: OnceLock<ColumnTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let path = repo_root().join("scripts/terminal-columns.txt");
        ColumnTable::load(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    })
}

/// The columns a cluster takes once the table's presentation selector follows
/// it: an emoji presentation is two columns wide.
const PRESENTATION_COLUMNS: usize = 2;

/// The columns a range of codepoints takes, the cell that advances to the
/// next tab stop, or a codepoint that joins the cluster before it.
#[derive(Clone, Copy)]
enum Columns {
    Fixed(usize),
    Tab,
    Join,
}

struct ColumnRange {
    start: u32,
    end: u32,
    width: Columns,
}

struct ColumnCase {
    name: String,
    text: String,
    expected: usize,
}

struct ColumnTable {
    tab_stop: usize,
    join_after: char,
    presentation: char,
    ranges: Vec<ColumnRange>,
    cases: Vec<ColumnCase>,
}

impl ColumnTable {
    fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        let mut tab_stop = None;
        let mut join_after = None;
        let mut presentation = None;
        let mut ranges: Vec<ColumnRange> = Vec::new();
        let mut cases = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let number = index + 1;
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split_whitespace().collect();
            let error = |message: String| format!("{}:{number}: {message}", path.display());
            match fields[0] {
                "rule" => {
                    if fields.len() < 3 {
                        return Err(error(
                            "want: rule tab-stop N, rule join-after CP or rule presentation-selector CP"
                                .to_string(),
                        ));
                    }
                    match fields[1] {
                        "tab-stop" => {
                            if fields.len() != 3 {
                                return Err(error("want: rule tab-stop N".to_string()));
                            }
                            let stop: usize = fields[2].parse().map_err(|_| {
                                error(format!("not a positive number: {}", fields[2]))
                            })?;
                            if stop == 0 {
                                return Err(error("the tab stop must be positive".to_string()));
                            }
                            tab_stop = Some(stop);
                        }
                        "join-after" => {
                            if fields.len() != 3 {
                                return Err(error("want: rule join-after CP".to_string()));
                            }
                            let value = u32::from_str_radix(fields[2], 16).map_err(|_| {
                                error(format!("not a hex codepoint: {}", fields[2]))
                            })?;
                            let character = char::from_u32(value)
                                .ok_or_else(|| error(format!("not a character: {}", fields[2])))?;
                            join_after = Some(character);
                        }
                        "presentation-selector" => {
                            if fields.len() != 3 {
                                return Err(error(
                                    "want: rule presentation-selector CP".to_string(),
                                ));
                            }
                            let value = u32::from_str_radix(fields[2], 16).map_err(|_| {
                                error(format!("not a hex codepoint: {}", fields[2]))
                            })?;
                            let character = char::from_u32(value)
                                .ok_or_else(|| error(format!("not a character: {}", fields[2])))?;
                            presentation = Some(character);
                        }
                        // The other rule lines derive the ranges; the
                        // generator reads them, so this reader only needs
                        // the three above.
                        _ => {}
                    }
                }
                "case" => {
                    if fields.len() != 4 {
                        return Err(error("want: case NAME CODEPOINTS COLUMNS".to_string()));
                    }
                    let mut text = String::new();
                    for codepoint in fields[2].split(',') {
                        let value = u32::from_str_radix(codepoint, 16)
                            .map_err(|_| error(format!("not a hex codepoint: {codepoint}")))?;
                        let character = char::from_u32(value)
                            .ok_or_else(|| error(format!("not a character: {codepoint}")))?;
                        text.push(character);
                    }
                    let expected = fields[3]
                        .parse()
                        .map_err(|_| error(format!("not a column count: {}", fields[3])))?;
                    cases.push(ColumnCase {
                        name: fields[1].to_string(),
                        text,
                        expected,
                    });
                }
                range => {
                    if fields.len() != 2 {
                        return Err(error("want: START-END WIDTH".to_string()));
                    }
                    let (start, end) = range
                        .split_once('-')
                        .ok_or_else(|| error("want: START-END WIDTH".to_string()))?;
                    let start = u32::from_str_radix(start, 16)
                        .map_err(|_| error(format!("not a hex codepoint: {start}")))?;
                    let end = u32::from_str_radix(end, 16)
                        .map_err(|_| error(format!("not a hex codepoint: {end}")))?;
                    let width = match fields[1] {
                        "tab" => Columns::Tab,
                        "join" => Columns::Join,
                        other => match other.parse::<usize>() {
                            Ok(0) => Columns::Fixed(0),
                            Ok(1) => Columns::Fixed(1),
                            Ok(2) => Columns::Fixed(2),
                            _ => {
                                return Err(error(format!(
                                    "a range takes 0, 1, 2, tab or join, not {other}"
                                )));
                            }
                        },
                    };
                    if start > end {
                        return Err(error(format!("range {range} runs backwards")));
                    }
                    if let Some(last) = ranges.last()
                        && start <= last.end
                    {
                        return Err(error("ranges are not in order".to_string()));
                    }
                    ranges.push(ColumnRange { start, end, width });
                }
            }
        }
        let tab_stop = tab_stop.ok_or_else(|| format!("{}: no tab-stop rule", path.display()))?;
        let join_after =
            join_after.ok_or_else(|| format!("{}: no join-after rule", path.display()))?;
        let presentation = presentation
            .ok_or_else(|| format!("{}: no presentation-selector rule", path.display()))?;
        if ranges.is_empty() {
            return Err(format!("{}: no ranges", path.display()));
        }
        if cases.is_empty() {
            return Err(format!("{}: no cases", path.display()));
        }
        Ok(Self {
            tab_stop,
            join_after,
            presentation,
            ranges,
            cases,
        })
    }

    fn range_at(&self, codepoint: u32) -> Option<&ColumnRange> {
        let index = self
            .ranges
            .partition_point(|range| range.start <= codepoint);
        if index == 0 {
            return None;
        }
        let range = &self.ranges[index - 1];
        (codepoint <= range.end).then_some(range)
    }

    fn advance(&self, character: char, column: usize) -> usize {
        match self.range_at(character as u32) {
            Some(ColumnRange {
                width: Columns::Fixed(width),
                ..
            }) => *width,
            Some(ColumnRange {
                width: Columns::Join,
                ..
            }) => 0,
            Some(_) => self.tab_stop - column % self.tab_stop,
            None => 1,
        }
    }

    /// Whether the table says `character` joins the cluster before it.
    fn joins(&self, character: char) -> bool {
        matches!(
            self.range_at(character as u32),
            Some(ColumnRange {
                width: Columns::Join,
                ..
            })
        )
    }

    /// The columns a line takes, by the table's grapheme-cluster rule.
    ///
    /// A cluster takes the columns of its first codepoint, and the codepoints
    /// that join it take none. A codepoint joins when the table says `join`,
    /// or when the one before it is the table's joiner. The table's
    /// presentation selector gives the cluster before it two columns, when
    /// that is more than the cluster already takes.
    fn width_of(&self, text: &str) -> usize {
        let mut column = 0;
        let mut cluster_start = 0;
        let mut open_cluster = false;
        let mut previous_was_joiner = false;
        for character in text.chars() {
            if open_cluster && character == self.presentation {
                column = column.max(cluster_start + PRESENTATION_COLUMNS);
            } else if !(self.joins(character) || (open_cluster && previous_was_joiner)) {
                cluster_start = column;
                column += self.advance(character, column);
                open_cluster = true;
            }
            previous_was_joiner = character == self.join_after;
        }
        column
    }

    fn case(&self, name: &str) -> Option<&ColumnCase> {
        self.cases.iter().find(|case| case.name == name)
    }
}

/// A line measured by the sweep's own reader: the other side of the compare.
fn sweep_width(line: &str) -> usize {
    let mut child = Command::new("python3")
        .arg(repo_root().join("scripts/pty-sweep.py"))
        .arg("--measure")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("running the sweep's measure");
    {
        let stdin = child.stdin.as_mut().expect("the sweep's stdin");
        stdin.write_all(line.as_bytes()).expect("writing the line");
        stdin.write_all(b"\n").expect("ending the line");
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("the sweep's measure");
    assert!(
        output.status.success(),
        "the sweep refused the line: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("the measurement is UTF-8")
        .trim()
        .parse()
        .expect("a measured width")
}

/// A work folder holding one approved task whose title is a wide grapheme.
fn wide_task_work() -> PathBuf {
    let source = fixture_path("TSK29");
    let text = fs::read_to_string(&source).expect("the task the wide title is built from");
    let mut task: serde_json::Value = serde_json::from_str(&text).expect("the fixture parses");
    task["id"] = serde_json::json!("TSK01");
    task["title"] = serde_json::json!("界");
    task["slug"] = serde_json::json!("wide");
    task["depends_on"] = serde_json::json!([]);
    let dir = std::env::temp_dir().join(format!("raul-wide-grapheme-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a work folder");
    fs::write(
        dir.join("TSK01-wide.json"),
        serde_json::to_string_pretty(&task).expect("a task") + "\n",
    )
    .expect("writing the wide task");
    dir
}

#[test]
fn the_overview_render_matches_the_snapshot() {
    assert_overview_at(100, "overview.txt");
}

/// The height the detail snapshot renders at: tall enough for the whole TSK13
/// document, so the snapshot carries every section — the fenced code block in
/// its note included — and not a window onto the top of it.
const DETAIL_SNAPSHOT_HEIGHT: &str = "220";

/// The detail pane of a real mock-data task, as the snapshot records it.
fn detail_snapshot() -> Output {
    run(&[
        "--render",
        "--plain",
        "--detail",
        DETAIL_TASK,
        "--width",
        "100",
        "--height",
        DETAIL_SNAPSHOT_HEIGHT,
        WORK,
    ])
}

fn detail_snapshot_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/detail.txt")
}

#[test]
fn the_detail_render_matches_the_snapshot() {
    // The snapshot is a real mock-data task, not a synthetic one: the
    // document the command prints is asserted byte for byte against the
    // committed file, so any change to the projection, the engine or the
    // pane shows up here first.
    let output = detail_snapshot();
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let snapshot = fs::read_to_string(detail_snapshot_path()).expect("the detail snapshot");
    assert_eq!(
        text, snapshot,
        "the detail render differs from tests/snapshots/detail.txt"
    );
    assert!(!text.contains('\u{1b}'), "the render carries ANSI escapes");
    assert!(text.ends_with('\n'), "the render has no final newline");
}

/// A line's words, with a heading's lead glyph and its rule taken off.
fn words(content: &str) -> &str {
    content
        .trim_start_matches(['▍', '▸'])
        .trim()
        .trim_end_matches('─')
        .trim()
}

/// The lines of the Review section, borders and padding removed.
fn rendered_review(text: &str) -> Vec<String> {
    let mut inside = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let content = words(line.trim_matches('│').trim());
        if content == "Review" {
            inside = true;
            continue;
        }
        if inside && content == "Questions" {
            break;
        }
        if inside {
            lines.push(content.to_string());
        }
    }
    assert!(
        !lines.is_empty(),
        "the render has no Review section:\n{text}"
    );
    lines
}

#[test]
fn the_review_reads_as_rounds_and_findings_at_forty_and_one_hundred_columns() {
    // At 40 and at 100 the review is a history: the round's heading, its
    // fields, and the findings under it, grouped by severity. The table the
    // unit removed rendered `self` as `se`/`lf` at 100 and one character per
    // line at 40; neither width may split a finding from its round. The
    // heading carries four parts, so at 40 it may run over two lines.
    for (width, height) in [("40", "400"), ("100", "220")] {
        let output = run(&[
            "--render",
            "--plain",
            "--detail",
            DETAIL_TASK,
            "--width",
            width,
            "--height",
            height,
            WORK,
        ]);
        assert!(
            output.status.success(),
            "raul failed at {width}: {output:?}"
        );
        let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
        let review = rendered_review(&text);
        let starts = |prefix: &str| {
            review
                .iter()
                .position(|row| row.starts_with(prefix))
                .unwrap_or_else(|| panic!("no line starts with {prefix} at {width}:\n{review:#?}"))
        };
        let label = |text: &str| {
            review
                .iter()
                .position(|row| row == text)
                .unwrap_or_else(|| panic!("no {text} group at {width}:\n{review:#?}"))
        };

        let round = starts("Round  0  —  self");
        let medium = label("Medium");
        let first = starts("•  FIND-01");
        let low = label("Low");
        let second = starts("•  FIND-02");
        assert!(
            round < medium && medium < first && first < low && low < second,
            "a finding is not under its round and its group at {width}:\n{review:#?}"
        );

        // The heading names the verdict and the reviewer, on one line or two.
        let heading = review[round..medium].join(" ");
        for part in ["changes-needed", "runner-01"] {
            assert!(
                heading.contains(part),
                "{part} is missing from the heading at {width}:\n{review:#?}"
            );
        }

        // The round's fields are bullets under the heading.
        for bullet in ["• focus:  spec-compliance", "• at:  2026-11-10T16:00Z"] {
            let row = review
                .iter()
                .position(|line| line == bullet)
                .unwrap_or_else(|| panic!("{bullet} is missing at {width}:\n{review:#?}"));
            assert!(round < row, "{bullet} is not under the round at {width}");
        }

        // Nothing is squeezed into a column one character wide: no word from
        // the old table is left as a fragment on its own line.
        for fragment in ["s", "e", "l", "f", "ex", "te", "rn", "al"] {
            assert!(
                !review.iter().any(|line| line == fragment),
                "'{fragment}' is a line of the review at {width}:\n{review:#?}"
            );
        }
        assert!(
            review.iter().any(|line| line.contains("external")),
            "the word `external` is broken at {width}:\n{review:#?}"
        );
    }
}

/// The Overview at a canonical width, asserted against its snapshot. The
/// files are what `scripts/pty-sweep.py` writes at that width: one plain
/// render, byte for byte.
fn assert_overview_at(width: u16, name: &str) {
    let width_arg = width.to_string();
    let output = run(&[
        "--render", "--plain", "--width", &width_arg, "--height", "40", WORK,
    ]);
    assert!(
        output.status.success(),
        "raul failed at width {width}: {output:?}"
    );
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let path = snapshot_path(name);
    let snapshot =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(
        text, snapshot,
        "the Overview at width {width} differs from {name}"
    );
    assert!(!text.contains('\u{1b}'), "the render carries ANSI escapes");
    assert!(text.ends_with('\n'), "the render has no final newline");
    for (number, line) in text.lines().enumerate() {
        assert_eq!(
            line.trim_end_matches(' '),
            line,
            "row {number} ends in spaces that carry no information"
        );
        let measured = display_width(line);
        assert!(
            measured <= width as usize,
            "row {number} is {measured} columns wide, not at most {width}: {line:?}"
        );
    }
}

#[test]
fn the_file_names_the_icon_set_when_no_flag_or_variable_does() {
    // The file's `[raul] icons` draws the set a flag or a variable does not
    // name, and a flag still beats it.
    let home = std::env::temp_dir().join(format!("raul-file-home-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(home_at(&home)).expect("the home");
    let text = Config::default()
        .to_toml()
        .replace("icons = \"nerd\"", "icons = \"unicode\"");
    fs::write(config_path(&home), text).expect("the file");
    let render = |flags: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_raul"))
            .current_dir(repo_root())
            .env_remove(ICONS_ENV)
            .env("HOME", &home)
            .args(["--render", "--plain", "--width", "100", "--height", "40"])
            .args(flags)
            .arg(WORK)
            .output()
            .expect("running raul");
        assert!(output.status.success(), "raul failed: {output:?}");
        String::from_utf8(output.stdout).expect("the plain render is UTF-8")
    };
    let unicode = render(&["--icons", "unicode"]);
    assert_eq!(render(&[]), unicode, "the file names unicode");
    let nerd = fs::read_to_string(snapshot_path("overview.txt")).expect("the overview snapshot");
    assert_eq!(
        render(&["--icons", "nerd"]),
        nerd,
        "the flag beats the file"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn a_broken_file_renders_the_default_screen_and_names_the_problem() {
    // A file raul cannot read does not stop it: the screen is the one the
    // defaults draw, and the file and the error are named on stderr.
    let home = std::env::temp_dir().join(format!("raul-broken-home-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(home_at(&home)).expect("the home");
    fs::write(config_path(&home), "round_limit = \n").expect("a broken file");
    let output = Command::new(env!("CARGO_BIN_EXE_raul"))
        .current_dir(repo_root())
        .env_remove(ICONS_ENV)
        .env("HOME", &home)
        .args([
            "--render", "--plain", "--width", "100", "--height", "40", WORK,
        ])
        .output()
        .expect("running raul");
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let snapshot =
        fs::read_to_string(snapshot_path("overview.txt")).expect("the overview snapshot");
    assert_eq!(text, snapshot, "a broken file draws the defaults");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains(CONFIG_FILE), "{stderr}");
    assert!(stderr.contains("syntax error"), "{stderr}");
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn a_flag_names_the_icon_set_over_the_environment() {
    // A developer who exports the unicode set still gets the nerd snapshot
    // when the command names nerd: the flag wins over the variable.
    let output = Command::new(env!("CARGO_BIN_EXE_raul"))
        .current_dir(repo_root())
        .env(ICONS_ENV, "unicode")
        .env("HOME", render_home())
        .args([
            "--render", "--plain", "--icons", "nerd", "--width", "100", "--height", "40", WORK,
        ])
        .output()
        .expect("running raul");
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let snapshot =
        fs::read_to_string(snapshot_path("overview.txt")).expect("the overview snapshot");
    assert_eq!(
        text, snapshot,
        "a render that names nerd differs from overview.txt under {ICONS_ENV}=unicode"
    );
}

/// The Settings tab at a width, asserted against its snapshot. The home is the
/// render's own, so the configuration is the defaults the tab writes.
fn assert_settings_at(width: u16) {
    let width_arg = width.to_string();
    let output = run(&[
        "--render", "--plain", "--tab", "settings", "--width", &width_arg, "--height", "40", WORK,
    ]);
    assert!(
        output.status.success(),
        "raul failed for settings at width {width}: {output:?}"
    );
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let name = format!("settings-{width}.txt");
    let path = snapshot_path(&name);
    let snapshot =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(
        text, snapshot,
        "the Settings tab at width {width} differs from {name}"
    );
}

#[test]
fn the_settings_tab_matches_the_snapshot_at_40_and_100_columns() {
    // The file as the tab draws it: the comments, the headers and the values
    // of the defaults, the cursor on the first value, and the count of what
    // the pane does not show.
    assert_settings_at(40);
    assert_settings_at(100);
}

#[test]
fn the_overview_matches_the_snapshot_at_every_canonical_width() {
    // Widths the sweep and the reviewers read: the narrow design width, the
    // two-column layout, and the default. A rendering regression that the
    // data unit tests miss changes one of these files.
    for (width, name) in [
        (40, "overview-40.txt"),
        (60, "overview-60.txt"),
        (100, "overview.txt"),
    ] {
        assert_overview_at(width, name);
    }
}

/// A tab, named on the command line, rendered at a width.
fn render_tab(tab: &str, width: u16) -> String {
    render_tab_with("dark", tab, width)
}

/// A tab, rendered at a width in a named preset.
fn render_tab_with(preset: &str, tab: &str, width: u16) -> String {
    render_tab_at(preset, tab, width, 40)
}

/// A tab, rendered at a width and a height in a named preset.
fn render_tab_at(preset: &str, tab: &str, width: u16, height: u16) -> String {
    let width_arg = width.to_string();
    let height_arg = height.to_string();
    let output = run(&[
        "--render",
        "--plain",
        "--theme",
        preset,
        "--tab",
        tab,
        "--width",
        &width_arg,
        "--height",
        &height_arg,
        WORK,
    ]);
    assert!(
        output.status.success(),
        "raul failed for {preset}/{tab} at width {width}: {output:?}"
    );
    String::from_utf8(output.stdout).expect("the plain render is UTF-8")
}

#[test]
fn the_settings_tab_is_reachable_in_render_mode_at_every_canonical_width() {
    // The tab draws the configuration file: its first value is on screen at
    // the default height, and the title names the file.
    for width in [40, 60, 100] {
        let text = render_tab_at("dark", "settings", width, 60);
        assert_eq!(
            text.lines().count(),
            60,
            "the render did not take --height at width {width}"
        );
        // The tab bar names it, so the screen is the Settings tab.
        assert!(text.contains("Settings"), "width {width}: {text}");
        // It draws the file: its first value, and its path in the title.
        assert!(text.contains("round_limit = 5"), "width {width}: {text}");
        assert!(text.contains("config.toml"), "width {width}: {text}");
        assert!(
            !text.contains("nothing to set yet"),
            "width {width}: {text}"
        );
        // The footer names what changes the file: it quits, saves and reloads.
        // The keys that edit, change and move are the help's.
        let footer = text.lines().last().expect("a footer row").trim_end();
        assert!(footer.contains("quit"), "width {width}: {footer}");
        if width == 100 {
            for words in ["save", "undo", "reload", "help"] {
                assert!(footer.contains(words), "width {width}: {footer}");
            }
        }
        for (number, line) in text.lines().enumerate() {
            let measured = display_width(line);
            assert!(
                measured <= width as usize,
                "row {number} is {measured} columns wide, not at most {width}: {line:?}"
            );
        }
    }
}

#[test]
fn a_short_row_tab_render_shows_the_window_and_announces_what_it_hides() {
    // The plain render has no keys, so it shows the window at the top of a
    // short tab: the rows that fit are on screen, and the tab's own
    // announcement names the rows that are not. The window the keys move is
    // the same one: the app tests press the keys.
    for tab in ["needs-you", "path", "tasks", "backlog"] {
        let output = run(&[
            "--render", "--plain", "--tab", tab, "--width", "100", "--height", "12", WORK,
        ]);
        assert!(output.status.success(), "{tab}: raul failed: {output:?}");
        let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
        let rows: Vec<&str> = text.lines().collect();
        assert_eq!(rows.len(), 12, "{tab}: the render did not take --height");
        // The pane holds rows, not a blank screen.
        assert!(
            rows.iter().skip(3).any(|row| {
                row.contains("TSK") || row.contains("ITM") || row.contains("EXECUTION")
            }),
            "{tab}: no row is in the window: {text}"
        );
        // The rows the window leaves out are counted.
        assert!(
            text.contains("not shown") || text.contains("hidden") || text.contains("more task"),
            "{tab}: a short window hides its count: {text}"
        );
        for (number, line) in text.lines().enumerate() {
            let measured = display_width(line);
            assert!(
                measured <= 100,
                "{tab}: row {number} is {measured} columns wide: {line:?}"
            );
        }
    }
}

#[test]
fn the_row_tabs_fit_their_width_at_every_canonical_width() {
    // The selection bar is paint only: the plain render carries the same
    // words, and no line grows past the width it is drawn in.
    for tab in ["needs-you", "path", "tasks", "backlog"] {
        for width in [40u16, 60, 100] {
            let text = render_tab(tab, width);
            assert_eq!(
                text.lines().count(),
                40,
                "{tab} at {width}: the render did not take --height"
            );
            for (number, line) in text.lines().enumerate() {
                assert_eq!(
                    line.trim_end_matches(' '),
                    line,
                    "{tab} at {width}: row {number} ends in spaces that carry no information"
                );
                let measured = display_width(line);
                assert!(
                    measured <= width as usize,
                    "{tab} at {width}: row {number} is {measured} columns wide: {line:?}"
                );
            }
        }
    }
}

/// The ids of every task file in the mock work folder, in id order.
fn mock_task_ids() -> Vec<String> {
    let mut ids: Vec<String> = fs::read_dir(repo_root().join(WORK))
        .expect("the mock work folder")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with("TSK") && name.ends_with(".json"))
        .map(|name| name[..5].to_string())
        .collect();
    ids.sort();
    ids
}

/// The task id a line of the plain render names first, when it names one.
fn first_task_id(line: &str) -> Option<String> {
    let at = line.find("TSK")?;
    let digits: String = line[at + 3..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    (!digits.is_empty()).then(|| format!("TSK{digits}"))
}

#[test]
fn the_tasks_tab_lists_every_task_in_id_order_at_forty_and_one_hundred_columns() {
    // The index of the work: every task file is a row, drafts and finished
    // tasks included, in the id order a fresh window opens in. The mock
    // folder's thirty tasks fit the default height, so every one is shown.
    let expected = mock_task_ids();
    assert_eq!(expected.len(), 30, "the mock work folder changed");
    for width in [40u16, 100] {
        let text = render_tab("tasks", width);
        let shown: Vec<String> = text.lines().filter_map(first_task_id).collect();
        assert_eq!(shown, expected, "the tasks tab at {width} columns: {text}");
    }
}

#[test]
fn a_subtask_row_names_its_parent_at_one_hundred_columns() {
    // A subtask is a flat row, with its parent in the suffix, the parent icon
    // first. The mock folder has subtasks, so one row carries the suffix.
    let text = render_tab("tasks", 100);
    let suffix = format!("{} TSK", Icon::Parent.glyph(IconSet::Nerd));
    assert!(
        text.lines().any(|line| line.contains(&suffix)),
        "no subtask row names its parent: {text}"
    );
}

#[test]
fn the_row_tabs_carry_the_review_state() {
    // Needs you at 100: the escalated task with the budget spent and five
    // findings open says both facts, severities and all.
    let needs = render_tab("needs-you", 100);
    assert!(
        needs.contains("5 open findings (2 high, 2 medium, 1 low) · 5/5 rounds"),
        "{needs}"
    );
    // Path at 100: a review row says the findings open and their severity.
    let path = render_tab("path", 100);
    assert!(path.contains("1 open finding (1 medium)"), "{path}");
    // A row with nothing to report carries no state in either tab.
    for (tab, text) in [("needs-you", &needs), ("path", &path)] {
        let row = text
            .lines()
            .find(|row| row.contains("TSK04"))
            .unwrap_or_else(|| panic!("{tab}: no row for TSK04: {text}"));
        for word in ["rounds", "finding", "open"] {
            assert!(!row.contains(word), "{tab}: TSK04 carries {word}: {row}");
        }
    }
}

#[test]
fn the_backlog_row_carries_its_body_on_the_second_line() {
    // The real binary draws the item's own body under the row's columns,
    // indented to where the row's title starts. A draft carries none, so its
    // row is followed by the next row, not by a second line.
    let text = render_tab("backlog", 100);
    let rows: Vec<&str> = text.lines().collect();
    let at = rows
        .iter()
        .position(|row| row.contains("ITM01"))
        .unwrap_or_else(|| panic!("no ITM01 row: {text}"));
    let body = "The activity block says what happened; it does not say when the run began.";
    assert!(rows[at + 1].contains(body), "{:#?}", &rows[at..at + 2]);
    // The columns are counted on the screen: the selection bar is three bytes
    // and one column.
    let column = |row: &str, text: &str| {
        let start = row.find(text).expect("the text is on the row");
        display_width(&row[..start])
    };
    assert_eq!(
        column(rows[at + 1], body),
        column(rows[at], "The Overview says"),
        "the body is not indented to the row's own text"
    );
    let draft = rows
        .iter()
        .position(|row| row.contains("TSK01"))
        .unwrap_or_else(|| panic!("no TSK01 row: {text}"));
    assert!(
        rows[draft + 1].contains("TSK02"),
        "a draft row grew a second line: {:#?}",
        &rows[draft..draft + 2]
    );
}

/// A render's text with the borders and the padding of each row taken off,
/// and the rows joined by spaces: a sentence that wraps reads as one line.
fn flowed(text: &str) -> String {
    text.lines()
        .map(|row| row.trim_matches(|ch: char| ch == '│' || ch.is_whitespace()))
        .filter(|row| !row.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
}

#[test]
fn an_item_opens_its_own_document_at_forty_and_one_hundred_columns() {
    // A smaller task has a document like a task's: the header card, the body
    // and the history. Neither width says the item has no document, and the
    // body reads whole at both, wrapped only where the pane wraps it.
    let body = "The activity block says what happened; it does not say when the run began.";
    for (width, height) in [("40", "400"), ("100", "220")] {
        let output = run(&[
            "--render", "--plain", "--detail", "ITM01", "--width", width, "--height", height, WORK,
        ]);
        assert!(
            output.status.success(),
            "raul failed at {width}: {output:?}"
        );
        let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
        let limit: usize = width.parse().expect("a width");
        assert!(
            text.lines().all(|row| display_width(row) <= limit),
            "a row is wider than {width} columns:\n{text}"
        );
        assert!(!text.contains("no document yet"), "{text}");
        assert!(!text.contains("smaller task"), "{text}");

        let flat = flowed(&text);
        assert!(flat.contains("ITM01"), "{text}");
        for section in ["Header", "Body", "History"] {
            assert!(
                text.lines()
                    .any(|row| words(row.trim_matches('│').trim()) == section),
                "the {section} section is missing at {width}:\n{text}"
            );
        }
        for field in ["• id:", "• kind:", "• status:", "• scope:"] {
            assert!(
                flat.contains(field),
                "{field} is missing at {width}:\n{text}"
            );
        }
        assert!(
            flat.contains(body),
            "the body does not read whole at {width}:\n{text}"
        );
    }
}

/// The most urgent severity among a task's open findings.
fn most_urgent(task: &coding_crew_core::Task) -> Option<&'static str> {
    let mut found = [false; 3];
    for finding in task
        .review
        .findings
        .iter()
        .filter(|finding| finding.status == coding_crew_core::FindingStatus::Open)
    {
        match finding.severity {
            coding_crew_core::Severity::High => found[0] = true,
            coding_crew_core::Severity::Medium => found[1] = true,
            coding_crew_core::Severity::Low => found[2] = true,
        }
    }
    if found[0] {
        Some("high")
    } else if found[1] {
        Some("medium")
    } else if found[2] {
        Some("low")
    } else {
        None
    }
}

#[test]
fn a_rendered_row_that_names_the_open_findings_names_the_most_urgent_severity() {
    // The pin is on the rows the binary draws at 40, 60 and 100 columns. At
    // 100, every row that names the open findings names the most urgent
    // severity among them — TSK19, TSK20 and TSK21 included — on both tabs.
    let tasks = coding_crew_core::Store::new(repo_root().join(WORK))
        .list()
        .expect("the fixtures");
    let mut named = Vec::new();
    for tab in ["needs-you", "path"] {
        for width in [40u16, 60, 100] {
            let text = render_tab(tab, width);
            let rows: Vec<&str> = text.lines().collect();
            for line in &rows {
                assert!(
                    display_width(line) <= width as usize,
                    "{tab} at {width}: a row is wider than the pane: {line:?}"
                );
            }
            for task in &tasks {
                let Some(row) = rows.iter().find(|row| row.contains(&task.id)) else {
                    continue;
                };
                if !row.contains("open finding") {
                    continue;
                }
                // The count is labelled, at every width.
                assert!(
                    row.contains(" open finding"),
                    "{tab} at {width}: {} shows a bare count: {row}",
                    task.id
                );
                if width < 100 {
                    continue;
                }
                let urgent =
                    most_urgent(task).unwrap_or_else(|| panic!("{} has no open finding", task.id));
                assert!(
                    row.contains(urgent),
                    "{tab} at {width}: {} names no {urgent}: {row}",
                    task.id
                );
                named.push((tab, task.id.clone()));
            }
        }
    }
    // The pin is not vacuous: the unit's motivating rows are named on both
    // tabs, not skipped because the row was absent or the state dropped.
    for tab in ["needs-you", "path"] {
        for id in ["TSK19", "TSK20", "TSK21"] {
            let checked = named
                .iter()
                .any(|(checked_tab, checked_id)| *checked_tab == tab && checked_id == id);
            assert!(checked, "{tab} does not name the severity of {id} at 100");
        }
    }
}

/// Every tab the window has, by its command-line name.
const TAB_NAMES: [&str; 6] = [
    "overview",
    "needs-you",
    "path",
    "tasks",
    "backlog",
    "settings",
];

#[test]
fn every_tab_renders_alike_in_every_preset_at_every_canonical_width() {
    // The plain render carries no colour, so the words are the whole
    // contract a preset keeps on screen. Nord, Catppuccin and Dracula are
    // dark presets: they change the paint, and the same screen says the same
    // thing in every preset. Terminal is the one that names no colour; it
    // says the same words too. The Settings tab is the exception by design:
    // it marks the theme in use, so the mark is normalised and the rest of
    // the words still have to agree.
    let mark = Icon::InUse.glyph(IconSet::Nerd);
    let without_mark = |text: &str| text.replace(mark, " ");
    for tab in TAB_NAMES {
        for width in [40u16, 60, 100] {
            let dark = render_tab_with("dark", tab, width);
            for preset in ["nord", "catppuccin", "dracula", "terminal"] {
                let other = render_tab_with(preset, tab, width);
                if tab == "settings" {
                    assert_eq!(
                        without_mark(&other),
                        without_mark(&dark),
                        "{preset} changes the facts of {tab} at width {width}"
                    );
                } else {
                    assert_eq!(
                        other, dark,
                        "{preset} changes the words of {tab} at width {width}"
                    );
                }
                assert_eq!(
                    other.lines().count(),
                    40,
                    "{preset}/{tab} at {width}: the render did not take --height"
                );
            }
        }
    }
}

#[test]
fn the_tab_flag_needs_the_render_mode() {
    let output = run(&["--tab", "settings", WORK]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "--tab without the render mode exited 0: {output:?}"
    );
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--tab"), "{stderr}");
    assert!(stderr.contains("--render"), "{stderr}");
}

#[test]
fn an_unknown_tab_is_refused_before_rendering() {
    let output = run(&["--render", "--plain", "--tab", "settingss", WORK]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unknown tab did not exit 1: {output:?}"
    );
    assert!(output.stdout.is_empty(), "a screen was printed: {output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("settingss"), "{stderr}");
    assert!(stderr.contains("settings"), "{stderr}");
}

#[test]
fn the_tab_and_detail_flags_are_refused_together() {
    // The pane covers the whole window: the tab under it is a state the keys
    // cannot reach, so the pair is refused before anything is printed.
    let output = run(&[
        "--render",
        "--plain",
        "--tab",
        "path",
        "--detail",
        DETAIL_TASK,
        WORK,
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "--tab with --detail did not exit 1: {output:?}"
    );
    assert!(output.stdout.is_empty(), "a screen was printed: {output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--tab"), "{stderr}");
    assert!(stderr.contains("--detail"), "{stderr}");
}

#[test]
fn two_runs_print_the_same_bytes() {
    let first = overview();
    let second = run(&["--plain", "--height=40", "--render", "--width=100", WORK]);
    assert!(first.status.success(), "the first run failed: {first:?}");
    assert!(second.status.success(), "the second run failed: {second:?}");
    assert_eq!(first.stdout, second.stdout, "two runs printed differently");
}

#[test]
fn the_default_size_is_100_by_40() {
    let explicit = overview();
    let default = run(&["--render", "--plain", WORK]);
    assert!(
        explicit.status.success(),
        "the sized run failed: {explicit:?}"
    );
    assert!(
        default.status.success(),
        "the default run failed: {default:?}"
    );
    assert_eq!(
        default.stdout, explicit.stdout,
        "the default size is not 100 by 40"
    );
}

#[test]
fn the_render_takes_the_requested_dimensions() {
    let width = 73;
    let height = 23;
    let output = run(&[
        "--render", "--plain", "--width", "73", "--height", "23", WORK,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), height, "the render did not take --height");
    for (number, row) in rows.iter().enumerate() {
        let measured = display_width(row);
        assert!(
            measured <= width,
            "row {number} is {measured} columns wide, not at most {width}: {row:?}"
        );
    }
}

#[test]
fn the_terminal_column_table_pins_the_cases_the_measures_used_to_disagree_about() {
    // The cases live in the table, so the two measures cannot drift on the
    // characters that differ: a combining mark, a zero-width joiner, a soft
    // hyphen, a tab, a wide character, a fullwidth character, an emoji, a
    // joined emoji and the conjoining Hangul jamo.
    let table = column_table();
    for name in [
        "combining-mark",
        "zero-width-joiner",
        "soft-hyphen",
        "tab",
        "east-asian-wide",
        "fullwidth",
        "emoji",
        "joined-emoji",
        "joined-emoji-chain",
        "emoji-with-modifier",
        "conjoining-hangul-leading",
        "conjoining-hangul-medial",
        "conjoining-hangul-final",
        "conjoining-hangul-syllable",
        "emoji-presentation",
        "emoji-keycap",
    ] {
        let case = table
            .case(name)
            .unwrap_or_else(|| panic!("the table has no case {name}"));
        assert_eq!(
            table.width_of(&case.text),
            case.expected,
            "the table's {name} case"
        );
    }
    assert_eq!(table.tab_stop, 8, "the tab stop is the table's");
    assert_eq!(table.width_of("a"), 1, "an ordinary letter");
    assert_eq!(table.width_of("\u{0301}"), 0, "a combining mark");
    assert_eq!(table.width_of("e\u{0301}"), 1, "a letter and its mark");
    assert_eq!(table.width_of("\t"), 8, "a tab from the first column");
    assert_eq!(table.width_of("a\tb"), 9, "a tab advances in place");
}

#[test]
fn the_icon_and_spinner_cases_are_the_theme_glyphs_one_column_each() {
    // The sweep's icon and spinner cases are a hand copy of the theme's
    // glyphs. Each case must spell them in the theme's order, and each glyph
    // must take one column on its own: a total alone would pass one glyph at
    // two beside one at zero.
    let table = column_table();
    let sets: [(&str, Vec<&str>); 3] = [
        (
            "nerd-icons",
            Icon::ALL
                .iter()
                .map(|icon| icon.glyph(IconSet::Nerd))
                .collect(),
        ),
        (
            "unicode-icons",
            Icon::ALL
                .iter()
                .map(|icon| icon.glyph(IconSet::Unicode))
                .collect(),
        ),
        ("spinner-frames", SPINNER.to_vec()),
    ];
    for (name, glyphs) in sets {
        let case = table
            .case(name)
            .unwrap_or_else(|| panic!("the table has no case {name}"));
        assert_eq!(
            case.text,
            glyphs.concat(),
            "the table's {name} case spells the theme's glyphs"
        );
        for glyph in glyphs {
            assert_eq!(table.width_of(glyph), 1, "{glyph:?} in the {name} case");
        }
    }
}

#[test]
fn a_joined_cluster_counts_its_first_codepoint_once() {
    // The rule the old per-codepoint measure broke: a joiner sequence is one
    // cluster, and only its first codepoint's columns count. Summing the
    // codepoints would count the emoji after each joiner again.
    let table = column_table();
    let joined = "\u{1f469}\u{200d}\u{1f4bb}";
    assert_eq!(
        table.width_of(joined),
        2,
        "a joined emoji sequence draws one grapheme"
    );
    let scalar: usize = joined.chars().fold(0, |column, character| {
        column + table.advance(character, column)
    });
    assert_eq!(
        scalar, 4,
        "the cluster case is not vacuous: a scalar measure counts four"
    );
}

#[test]
fn the_conjoining_jamo_take_the_columns_they_draw() {
    // A leading consonant is East Asian wide and draws two columns; a vowel
    // or trailing jamo draws nothing alone and joins the syllable before it.
    let table = column_table();
    for leading in ['\u{1100}', '\u{a960}'] {
        assert_eq!(
            table.width_of(&leading.to_string()),
            2,
            "the leading jamo {leading:?} is two columns"
        );
    }
    for vowel_or_trailing in ['\u{1161}', '\u{11a8}', '\u{d7b0}'] {
        assert_eq!(
            table.width_of(&vowel_or_trailing.to_string()),
            0,
            "the jamo {vowel_or_trailing:?} joins and takes no column"
        );
    }
}

#[test]
fn the_table_measures_what_ratatui_draws() {
    // The table is the rule both measures share; it is right only if it says
    // what the terminal paints. ratatui is the paint: its buffer takes each
    // grapheme's cells, so the columns it fills are the columns drawn.
    for text in [
        "\u{26a0}\u{fe0f}",
        "\u{2714}\u{fe0f}",
        "\u{2764}\u{fe0f}",
        "1\u{fe0f}\u{20e3}",
        "\u{1100}",
        "\u{1100}\u{1161}\u{11a8}",
        "\u{1161}",
        "a\u{1161}",
    ] {
        assert_eq!(display_width(text), drawn_width(text), "{text:?}");
    }
}

/// The columns ratatui paints a text in, from column zero.
fn drawn_width(text: &str) -> usize {
    let mut buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 20, 1));
    let (column, _) = buffer.set_stringn(0, 0, text, 20, ratatui::style::Style::default());
    usize::from(column)
}

#[test]
fn the_terminal_column_table_is_regenerated_from_its_rules() {
    // The table's header claims its ranges are the rules unrolled. The
    // generator is what makes that a claim a test reads: a range edited by
    // hand, or a rule changed without regenerating, fails here.
    let output = Command::new("python3")
        .arg(repo_root().join("scripts/generate-terminal-columns.py"))
        .arg("--check")
        .output()
        .expect("running the table's generator");
    assert!(
        output.status.success(),
        "the committed table is not what its rules generate: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_sweep_and_the_rust_check_measure_the_same_table_cases() {
    // The sweep reads the same file through its own reader. Its check prints
    // one measured/expected pair per case; every one must equal the number
    // this side measures, so the two implementations agree on the rules.
    let output = Command::new("python3")
        .arg(repo_root().join("scripts/pty-sweep.py"))
        .arg("--check-table")
        .output()
        .expect("running the sweep's table check");
    assert!(
        output.status.success(),
        "the sweep refused the table: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("the table check is UTF-8");
    let table = column_table();
    let mut seen = 0;
    for line in stdout.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 3 {
            continue;
        }
        let case = table.case(fields[0]).unwrap_or_else(|| {
            panic!(
                "the sweep measures a case the table does not: {}",
                fields[0]
            )
        });
        assert_eq!(
            fields[1].parse::<usize>().expect("the sweep's measurement"),
            table.width_of(&case.text),
            "the sweep and the Rust check disagree on {}",
            case.name
        );
        assert_eq!(
            fields[1], fields[2],
            "the sweep's {} case drifted from its own table",
            case.name
        );
        seen += 1;
    }
    assert_eq!(
        seen,
        table.cases.len(),
        "the sweep checked {seen} of the table's {} cases",
        table.cases.len()
    );
}

#[test]
fn a_wide_grapheme_costs_two_columns_not_three() {
    let dir = wide_task_work();
    let work = dir.to_str().expect("the work folder is UTF-8");
    let output = run(&[
        "--render", "--plain", "--width", "100", "--height", "40", work,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 40, "the render did not take --height");
    for (number, row) in rows.iter().enumerate() {
        let measured = display_width(row);
        assert!(
            measured <= 100,
            "row {number} is {measured} columns wide, not at most 100: {row:?}"
        );
    }
    let row = rows
        .iter()
        .find(|row| row.contains('界'))
        .expect("the wide title is not on screen");
    assert!(
        row.contains("TSK01"),
        "the wide title is not named: {row:?}"
    );
    assert_eq!(
        display_width(row),
        100,
        "the row of the wide title is not aligned: {row:?}"
    );
    assert!(
        row.ends_with('│'),
        "the right border is not on the row of the wide title: {row:?}"
    );
}

#[test]
fn unreadable_files_are_rendered_in_name_order() {
    // The malformed files are created in a different order from their names,
    // so the render can only be stable if the load orders them.
    let dir = support::malformed_work("render-unreadable-order");
    let work = dir.to_str().expect("the work folder is UTF-8");
    let output = run(&[
        "--render", "--plain", "--width", "100", "--height", "40", work,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let mut previous = 0;
    for name in support::NAME_ORDER {
        let at = text
            .find(name)
            .unwrap_or_else(|| panic!("{name} is not named: {text}"));
        assert!(at >= previous, "{name} is out of order: {text}");
        previous = at;
    }
}

#[test]
fn duplicate_ids_render_the_same_in_every_creation_order() {
    // Two folders with the same files, created in different orders, must
    // render the same bytes. Each file name is unique, so it breaks the id
    // tie the same way in every order; nothing here assumes the order the
    // filesystem hands back.
    let orders = permutations(support::DUPLICATE_ID_FILES.len());
    let mut first: Option<Vec<u8>> = None;
    for (index, order) in orders.iter().enumerate() {
        let project = duplicate_id_project(&format!("render-duplicate-order-{index}"), order);
        let output = run_in(
            &project,
            &[
                "--render", "--plain", "--width", "100", "--height", "40", "work",
            ],
        );
        assert!(
            output.status.success(),
            "raul failed for creation order {order:?}: {output:?}"
        );
        let text = String::from_utf8(output.stdout.clone()).expect("the plain render is UTF-8");
        // The Overview names only the first claimable task, so the duplicate
        // that wins the tie is the one on screen. Asserting the winner is
        // Alpha catches a load that hands Beta back first; a check for either
        // title would not.
        assert!(
            text.contains("Alpha task"),
            "Alpha task is not the rendered duplicate for creation order {order:?}: {text}"
        );
        assert!(
            !text.contains("Beta task"),
            "Beta task renders before its file name for creation order {order:?}: {text}"
        );
        match &first {
            None => first = Some(output.stdout),
            Some(expected) => assert_eq!(
                expected, &output.stdout,
                "creation order {order:?} rendered different bytes"
            ),
        }
        let _ = fs::remove_dir_all(&project);
    }
    assert!(first.is_some(), "there is no creation order to check");
}

#[test]
fn either_flag_alone_is_refused() {
    for flag in ["--render", "--plain"] {
        let output = run(&[flag, WORK]);
        assert!(!output.status.success(), "{flag} alone exited 0");
        assert!(output.stdout.is_empty(), "{flag} alone printed a screen");
    }
}

/// The task id whose title the detail tests show.
const DETAIL_TASK: &str = "TSK13";

/// The title of the detail task.
fn detail_title() -> String {
    let source = fixture_path(DETAIL_TASK);
    let json = fs::read_to_string(&source).expect("the detail task fixture");
    let task: serde_json::Value = serde_json::from_str(&json).expect("the fixture parses");
    task["title"].as_str().expect("a title").to_string()
}

#[test]
fn the_detail_renders_the_record_close_up() {
    let output = run(&[
        "--render",
        "--plain",
        "--detail",
        DETAIL_TASK,
        "--width",
        "100",
        "--height",
        "40",
        WORK,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 40, "the render did not take --height");
    // The header: the id and the title, both on screen.
    assert!(text.contains(DETAIL_TASK), "{text}");
    assert!(text.contains(&detail_title()), "{text}");
    // The body is the document: the first sections are on display.
    assert!(text.contains("Header"), "{text}");
    assert!(text.contains("Intent"), "{text}");
    // The footer says what the pane does.
    assert!(text.contains("esc") && text.contains("close"), "{text}");
    // The pane is full screen: the window under it is not on display.
    assert!(!text.contains("Settings"), "{text}");
    assert!(!text.contains("Overview"), "{text}");
}

/// The detail pane of a fixture, tall enough for its whole note.
fn detail_render(id: &str) -> String {
    let output = run(&[
        "--render", "--plain", "--detail", id, "--width", "100", "--height", "400", WORK,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    String::from_utf8(output.stdout).expect("the plain render is UTF-8")
}

#[test]
fn the_note_constructs_render_in_the_detail_of_their_fixtures() {
    let diagram = detail_render("TSK20");
    assert!(diagram.contains("╭─ mermaid"), "{diagram}");
    // The alerts draw the icon set in force: nerd by default.
    let alerts = detail_render("TSK19");
    let note = format!("{} Note", Icon::AlertNote.glyph(IconSet::Nerd));
    assert!(alerts.contains(&note), "{alerts}");
    let warning = format!("{} Warning", Icon::AlertWarning.glyph(IconSet::Nerd));
    assert!(alerts.contains(&warning), "{alerts}");
}

#[test]
fn the_detail_is_bounded_by_the_dimensions() {
    let title = detail_title();
    let output = run(&[
        "--render",
        "--plain",
        "--detail",
        DETAIL_TASK,
        "--width",
        "30",
        "--height",
        "12",
        WORK,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 12, "the render did not take --height");
    for (number, row) in rows.iter().enumerate() {
        let measured = display_width(row);
        assert!(
            measured <= 30,
            "row {number} is {measured} columns wide, not at most 30: {row:?}"
        );
    }
    // The title is cut, and the cut is marked: a part of it is never
    // shown as if it were the whole one.
    assert!(
        rows.iter().any(|row| row.contains('…')),
        "no cut is marked: {text}"
    );
    assert!(
        !text.contains(&title),
        "the whole title is shown where it does not fit: {text}"
    );
    assert!(text.contains(DETAIL_TASK), "{text}");
}

/// A work folder holding one task whose title is `count` joined `é`
/// graphemes: two scalars each, one column each.
fn combining_title_work() -> PathBuf {
    let title = "e\u{0301}".repeat(30);
    let source = fixture_path("TSK29");
    let text = fs::read_to_string(&source).expect("the task the title is built from");
    let mut task: serde_json::Value = serde_json::from_str(&text).expect("the fixture parses");
    task["id"] = serde_json::json!("TSK01");
    task["title"] = serde_json::json!(title);
    task["slug"] = serde_json::json!("combining");
    task["depends_on"] = serde_json::json!([]);
    let dir = std::env::temp_dir().join(format!("raul-combining-title-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a work folder");
    fs::write(
        dir.join("TSK01-combining.json"),
        serde_json::to_string_pretty(&task).expect("a task") + "\n",
    )
    .expect("writing the task");
    dir
}

#[test]
fn a_combining_title_is_cut_on_grapheme_boundaries() {
    // The header holds more graphemes than the pane has columns for: the
    // cut keeps whole graphemes. A scalar cut would leave a dangling
    // combining mark behind, and the check can fail on one.
    let dir = combining_title_work();
    let work = dir.to_str().expect("the work folder is UTF-8");
    let output = run(&[
        "--render", "--plain", "--detail", "TSK01", "--width", "30", "--height", "12", work,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    for (number, row) in text.lines().enumerate() {
        for grapheme in ratatui::text::Span::raw(row.to_string())
            .styled_graphemes(ratatui::style::Style::default())
        {
            assert_ne!(
                grapheme.symbol, "\u{0301}",
                "row {number} cut a grapheme: {row:?}"
            );
        }
    }
    assert!(text.contains('…'), "no cut is marked: {text}");
    // The case the two measures used to disagree about: a rendered line with
    // a combining mark. The sweep and this check must now call it one column.
    let row = text
        .lines()
        .find(|row| row.contains('\u{0301}'))
        .expect("the combining title is not on screen");
    assert_eq!(
        display_width(row),
        sweep_width(row),
        "the two measures disagree on a rendered combining mark: {row:?}"
    );
    assert_eq!(
        display_width(row),
        30,
        "the rendered row is not the pane width: {row:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_unknown_detail_id_is_refused_before_rendering() {
    let output = run(&[
        "--render", "--plain", "--detail", "TSK99", "--width", "100", "--height", "40", WORK,
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unknown id did not exit 1: {output:?}"
    );
    assert!(output.stdout.is_empty(), "a screen was printed: {output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("TSK99"), "{stderr}");
}

#[test]
fn the_detail_flag_needs_the_render_mode() {
    let output = run(&["--detail", DETAIL_TASK, WORK]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "--detail without the render mode exited 0: {output:?}"
    );
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--detail"), "{stderr}");
}

#[test]
fn a_dimension_over_1000_is_refused_before_rendering() {
    for (flag, value) in [
        ("--width", "1001"),
        ("--height", "1001"),
        ("--width", "65535"),
        ("--height", "65535"),
    ] {
        let output = run(&["--render", "--plain", flag, value, WORK]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{flag} {value} did not exit 1: {output:?}"
        );
        assert!(
            output.stdout.is_empty(),
            "{flag} {value} printed a screen before refusing"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(flag), "{flag} {value}: {stderr}");
        assert!(stderr.contains("1000"), "{flag} {value}: {stderr}");
        assert!(
            !stderr.contains("panicked") && !stderr.contains("memory allocation"),
            "{flag} {value} aborted: {stderr}"
        );
    }
}
