//! `hugo --render --plain` through the real binary, swept over the widths the
//! sweep covers.
//!
//! Every line is measured in terminal columns by the sweep's own reader, so a
//! line that ratatui would clip or that a terminal draws wider than its width
//! fails here. The render runs in the mock project, whose runtime record is
//! the one hugo's fixtures read.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The widths the sweep covers, as raul's sweep does.
const FIRST_WIDTH: u16 = 40;
const LAST_WIDTH: u16 = 120;

/// A short window, where the footer has one row, and the render default.
const HEIGHTS: [u16; 2] = [12, 40];

/// The icon sets hugo draws, each swept on its own.
const ICON_SETS: [&str; 2] = ["nerd", "unicode"];

/// The variable that names an icon set when no flag does.
const ICONS_ENV: &str = "CODING_CREW_ICONS";

/// The closing chips the footer keeps at every width: they are the keys that
/// answer wherever the help opens, so the footer never drops them.
const CLOSING_CHIPS: [&str; 2] = ["? help", "q quit"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// The configuration home renders read. A developer's own file must not
/// change what is measured, so the renders never read `$HOME`.
fn render_home() -> PathBuf {
    let home = std::env::temp_dir().join(format!("hugo-render-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).expect("a scratch home");
    home
}

/// One plain render of the window at `width` by `height` in `icons`.
fn render(width: u16, height: u16, icons: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_hugo"))
        .current_dir(repo_root().join("mock-data"))
        .env_remove(ICONS_ENV)
        .env("HOME", render_home())
        .args([
            "--render",
            "--plain",
            "--icons",
            icons,
            "--width",
            &width.to_string(),
            "--height",
            &height.to_string(),
        ])
        .output()
        .expect("running hugo");
    assert!(
        output.status.success(),
        "{icons} {width}x{height}: hugo exited {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("the render is UTF-8")
}

/// The terminal columns of each line of `text`, measured by the sweep's own
/// reader, the one `make sweep` runs.
fn sweep_widths(text: &str) -> Vec<usize> {
    let mut child = Command::new("python3")
        .arg(repo_root().join("scripts/pty-sweep.py"))
        .arg("--measure")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("running the sweep's measure");
    child
        .stdin
        .as_mut()
        .expect("the sweep's stdin")
        .write_all(text.as_bytes())
        .expect("writing the render");
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("the sweep's measure");
    assert!(
        output.status.success(),
        "the sweep refused the render: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("the measurement is UTF-8")
        .lines()
        .map(|line| line.trim().parse().expect("a measured width"))
        .collect()
}

#[test]
fn the_window_and_its_footer_fit_every_width_the_sweep_covers() {
    for icons in ICON_SETS {
        for height in HEIGHTS {
            for width in FIRST_WIDTH..=LAST_WIDTH {
                let text = render(width, height, icons);
                let rows = text.lines().count();
                assert_eq!(rows, usize::from(height), "{icons} {width}x{height}");

                let measured = sweep_widths(&text);
                assert_eq!(measured.len(), rows, "{icons} {width}x{height}");
                for (number, (columns, row)) in measured.iter().zip(text.lines()).enumerate() {
                    assert!(
                        *columns <= usize::from(width),
                        "{icons} {width}x{height} row {}: {columns} columns: {row:?}",
                        number + 1
                    );
                }

                for chip in CLOSING_CHIPS {
                    assert!(
                        text.contains(chip),
                        "{icons} {width}x{height}: the footer drops {chip:?}\n{text}"
                    );
                }
            }
        }
    }
}
