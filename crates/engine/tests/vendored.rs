//! Keeps `VENDORED.md` honest without the network.
//!
//! The manifest records the SHA-256 of each upstream module and of the
//! vendored copy. This test checks the bytes on disk against that record, so
//! a change to a vendored file has to change the ledger in the same commit.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().join("../..")
}

fn read_vendored() -> String {
    fs::read_to_string(manifest_dir().join("VENDORED.md")).expect("read VENDORED.md")
}

fn trim_cell(cell: &str) -> String {
    cell.trim().trim_matches('`').trim().to_string()
}

fn sha256(path: &Path) -> String {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .or_else(|_| {
            Command::new("shasum")
                .args(["-a", "256"])
                .arg(path)
                .output()
        })
        .expect("a SHA-256 tool (sha256sum or shasum)");
    assert!(output.status.success(), "failed to hash {}", path.display());
    String::from_utf8(output.stdout)
        .expect("hash output is UTF-8")
        .split_whitespace()
        .next()
        .expect("hash output has a digest")
        .to_string()
}

#[derive(Debug)]
struct ManifestRow {
    upstream: String,
    upstream_sha: String,
    vendored: String,
    vendored_sha: String,
    deviation: String,
}

fn upstream_rows(document: &str) -> Vec<ManifestRow> {
    let mut section = "";
    let mut rows = Vec::new();
    for line in document.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            section = heading.trim();
            continue;
        }
        if section != "Upstream files" || !line.starts_with("| `src/markdown/") {
            continue;
        }
        let cells: Vec<String> = line.split('|').map(trim_cell).collect();
        assert_eq!(cells.len(), 7, "manifest row shape: {line}");
        rows.push(ManifestRow {
            upstream: cells[1].clone(),
            upstream_sha: cells[2].clone(),
            vendored: cells[3].clone(),
            vendored_sha: cells[4].clone(),
            deviation: cells[5].clone(),
        });
    }
    rows
}

fn added_files(document: &str) -> Vec<String> {
    let mut section = "";
    let mut files = Vec::new();
    for line in document.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            section = heading.trim();
            continue;
        }
        if section != "Added engine files" || !line.starts_with("| `crates/engine/") {
            continue;
        }
        let cells: Vec<String> = line.split('|').map(trim_cell).collect();
        assert_eq!(cells.len(), 4, "added-file row shape: {line}");
        files.push(cells[1].clone());
    }
    files
}

fn ledger_numbers(document: &str) -> BTreeSet<u32> {
    let mut numbers = BTreeSet::new();
    for line in document.lines() {
        let Some(heading) = line.strip_prefix("### ") else {
            continue;
        };
        let Some((number, _)) = heading.split_once('.') else {
            continue;
        };
        if let Ok(number) = number.parse::<u32>() {
            numbers.insert(number);
        }
    }
    numbers
}

const EXPECTED_MODULES: [&str; 18] = [
    "blocks",
    "fences",
    "footnotes",
    "frontmatter",
    "highlight",
    "latex",
    "links",
    "lists",
    "markers",
    "mermaid",
    "mod",
    "spans",
    "syntax",
    "table_layout",
    "tables",
    "toc",
    "width",
    "wrapping",
];

#[test]
fn manifest_matches_the_vendored_bytes() {
    let root = workspace_root();
    let document = read_vendored();
    let rows = upstream_rows(&document);
    assert_eq!(rows.len(), 18, "the upstream file set is 18 modules");

    let ledger = ledger_numbers(&document);
    assert!(!ledger.is_empty(), "the deviation ledger is numbered");
    let highest = *ledger.iter().next_back().unwrap();
    for number in 1..=highest {
        assert!(ledger.contains(&number), "deviation {number} is missing");
    }

    let expected_upstream: BTreeSet<String> = EXPECTED_MODULES
        .iter()
        .map(|name| format!("src/markdown/{name}.rs"))
        .collect();
    let expected_vendored: BTreeSet<String> = EXPECTED_MODULES
        .iter()
        .map(|name| format!("crates/engine/src/markdown/{name}.rs"))
        .collect();

    let mut seen = BTreeSet::new();
    let mut seen_upstream = BTreeSet::new();
    for row in &rows {
        let name = row
            .upstream
            .strip_prefix("src/markdown/")
            .and_then(|path| path.strip_suffix(".rs"))
            .unwrap_or_else(|| panic!("noncanonical upstream path {}", row.upstream));
        assert!(
            EXPECTED_MODULES.contains(&name),
            "unexpected module {}",
            row.upstream
        );
        assert_eq!(
            row.upstream,
            format!("src/markdown/{name}.rs"),
            "noncanonical upstream path {}",
            row.upstream
        );
        assert_eq!(
            row.vendored,
            format!("crates/engine/src/markdown/{name}.rs"),
            "noncanonical vendored path {}",
            row.vendored
        );
        let vendored_path = root.join(&row.vendored);
        let actual = sha256(&vendored_path);
        assert_eq!(
            actual, row.vendored_sha,
            "{} changed; update VENDORED.md and the deviation ledger",
            row.vendored
        );

        if row.deviation == "identical" {
            assert_eq!(
                actual, row.upstream_sha,
                "{} is marked identical but differs from upstream",
                row.vendored
            );
        } else {
            assert_ne!(
                actual, row.upstream_sha,
                "{} is marked as a deviation but is byte-identical",
                row.vendored
            );
            for number in row.deviation.split(',') {
                let number: u32 = number
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| panic!("bad deviation number in {row:?}"));
                assert!(
                    ledger.contains(&number),
                    "{} references missing deviation {number}",
                    row.vendored
                );
            }
        }

        assert!(
            seen.insert(row.vendored.clone()),
            "duplicate manifest row for {}",
            row.vendored
        );
        seen_upstream.insert(row.upstream.clone());
    }
    assert_eq!(seen_upstream, expected_upstream, "the manifest module set");
    assert_eq!(seen, expected_vendored, "the canonical vendored paths");

    let mut on_disk = BTreeSet::new();
    for entry in fs::read_dir(manifest_dir().join("src/markdown")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            on_disk.insert(format!(
                "crates/engine/src/markdown/{}",
                path.file_name().unwrap().to_string_lossy()
            ));
        }
    }
    assert_eq!(seen, on_disk, "the manifest and src/markdown disagree");
}

#[cfg(unix)]
struct SyncFixture {
    temp: PathBuf,
    engine: PathBuf,
    upstream: PathBuf,
    bin: PathBuf,
    names: Vec<String>,
}

#[cfg(unix)]
impl SyncFixture {
    fn new(label: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;

        let temp = std::env::temp_dir().join(format!("engine-sync-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let engine = temp.join("crates/engine");
        fs::create_dir_all(engine.join("src/markdown")).unwrap();
        fs::copy(manifest_dir().join("sync.sh"), engine.join("sync.sh")).unwrap();

        let upstream = temp.join("upstream");
        fs::create_dir_all(upstream.join("src/markdown")).unwrap();

        let mut names = Vec::new();
        for index in 0..18 {
            let name = format!("module_{index:02}.rs");
            let source = format!("pub fn module_{index:02}() -> u8 {{ {index} }}\n");
            fs::write(upstream.join("src/markdown").join(&name), &source).unwrap();
            fs::write(engine.join("src/markdown").join(&name), &source).unwrap();
            names.push(name);
        }

        let bin = temp.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let fake_git = bin.join("git");
        fs::write(
            &fake_git,
            "#!/bin/sh\n\
if [ \"$1\" = \"init\" ]; then\n\
    for last; do :; done\n\
    mkdir -p \"$last\"\n\
    cp -R \"$FAKE_UPSTREAM\"/. \"$last\"/\n\
    exit 0\n\
fi\n\
if [ \"$1\" = \"-C\" ]; then shift 2; fi\n\
case \"$1\" in\n\
    remote|fetch|checkout) exit 0 ;;\n\
    rev-parse) echo 29f6ad480d95a4240c63f63d390e9909afdad9e8 ;;\n\
    *) exit 1 ;;\n\
esac\n",
        )
        .unwrap();
        fs::set_permissions(&fake_git, fs::Permissions::from_mode(0o755)).unwrap();

        let fixture = Self {
            temp,
            engine,
            upstream,
            bin,
            names,
        };
        fixture.write_manifest(&fixture.standard_rows());
        fixture
    }

    fn standard_rows(&self) -> Vec<String> {
        self.names
            .iter()
            .map(|name| {
                let sha = sha256(&self.upstream.join("src/markdown").join(name));
                format!(
                    "| `src/markdown/{name}` | `{sha}` | \
`crates/engine/src/markdown/{name}` | `{sha}` | identical |"
                )
            })
            .collect()
    }

    fn write_manifest(&self, rows: &[String]) {
        let ledger: String = (1..=13)
            .map(|number| format!("### {number}. Fixture deviation\n"))
            .collect();
        let manifest = format!(
            "# Vendored test fixture\n\n## Upstream files\n\n\
| upstream file | upstream sha256 | vendored file | vendored sha256 | deviation |\n\
|---|---|---|---|---|\n{}\n\n\
## Deviation ledger\n\n{ledger}",
            rows.join("\n")
        );
        fs::write(self.engine.join("VENDORED.md"), manifest).unwrap();
    }

    fn run_check(&self) -> std::process::Output {
        self.run_sync(&["check"])
    }

    fn run_sync(&self, args: &[&str]) -> std::process::Output {
        Command::new("bash")
            .arg(self.engine.join("sync.sh"))
            .args(args)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.bin.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .env("FAKE_UPSTREAM", &self.upstream)
            .output()
            .expect("run sync.sh")
    }
}

#[cfg(unix)]
impl Drop for SyncFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.temp);
    }
}

#[cfg(unix)]
fn report(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[cfg(unix)]
#[test]
fn sync_check_rejects_a_tampered_vendored_checksum() {
    let fixture = SyncFixture::new("tampered");

    let intact = fixture.run_check();
    assert!(
        intact.status.success(),
        "an intact copy should pass: {}",
        report(&intact)
    );

    let name = &fixture.names[0];
    fs::write(
        fixture.engine.join("src/markdown").join(name),
        b"pub fn example() -> u8 { 2 }\n",
    )
    .unwrap();
    let mut rows = fixture.standard_rows();
    rows[0] = rows[0].replace("| identical |", "| 1 |");
    fixture.write_manifest(&rows);

    let tampered = fixture.run_check();
    assert!(
        !tampered.status.success(),
        "a tampered vendored file must fail the check"
    );
    let report = report(&tampered);
    assert!(
        report.contains("vendored checksum mismatch in VENDORED.md"),
        "the failure names the vendored checksum: {report}"
    );
    assert!(
        report.contains(&format!("crates/engine/src/markdown/{name}")),
        "the failure names the tampered file: {report}"
    );
}

#[cfg(unix)]
#[test]
fn sync_check_rejects_a_wrong_upstream_basename() {
    let fixture = SyncFixture::new("basename");
    let mut rows = fixture.standard_rows();
    rows[0] = rows[0].replace(
        "| `src/markdown/module_00.rs` |",
        "| `src/markdown/wrong.rs` |",
    );
    fixture.write_manifest(&rows);

    let output = fixture.run_check();
    assert!(
        !output.status.success(),
        "a manifest row whose upstream basename differs must fail the check"
    );
    let report = report(&output);
    assert!(
        report.contains("basename mismatch"),
        "the failure names the basename mismatch: {report}"
    );
}

#[cfg(unix)]
#[test]
fn sync_check_rejects_a_nested_upstream_path() {
    let fixture = SyncFixture::new("nested-upstream");
    let mut rows = fixture.standard_rows();
    rows[0] = rows[0].replace(
        "| `src/markdown/module_00.rs` |",
        "| `src/markdown/nested/module_00.rs` |",
    );
    fixture.write_manifest(&rows);

    let output = fixture.run_check();
    assert!(
        !output.status.success(),
        "a noncanonical upstream path must fail the check"
    );
    let report = report(&output);
    assert!(
        report.contains("noncanonical upstream path"),
        "the failure names the upstream path: {report}"
    );
}

#[cfg(unix)]
#[test]
fn sync_check_rejects_a_nested_vendored_path() {
    let fixture = SyncFixture::new("nested-vendored");
    let mut rows = fixture.standard_rows();
    rows[0] = rows[0].replace(
        "`crates/engine/src/markdown/module_00.rs`",
        "`crates/engine/src/markdown/nested/module_00.rs`",
    );
    fixture.write_manifest(&rows);

    let output = fixture.run_check();
    assert!(
        !output.status.success(),
        "a noncanonical vendored path must fail the check"
    );
    let report = report(&output);
    assert!(
        report.contains("noncanonical vendored path"),
        "the failure names the vendored path: {report}"
    );
}

#[cfg(unix)]
#[test]
fn sync_check_rejects_an_unknown_deviation_number() {
    let fixture = SyncFixture::new("deviation");
    let mut rows = fixture.standard_rows();
    rows[0] = rows[0].replace("| identical |", "| 1, 999 |");
    fixture.write_manifest(&rows);

    let output = fixture.run_check();
    assert!(
        !output.status.success(),
        "a deviation number with no ledger heading must fail the check"
    );
    let report = report(&output);
    assert!(
        report.contains("references deviation 999"),
        "the failure names the unknown deviation: {report}"
    );
}

#[cfg(unix)]
#[test]
fn sync_check_rejects_a_nineteenth_ghost_row() {
    let fixture = SyncFixture::new("ghost");
    let mut rows = fixture.standard_rows();
    let sha = sha256(&fixture.upstream.join("src/markdown/module_00.rs"));
    rows.push(format!(
        "| `src/markdown/ghost.rs` | `{sha}` | \
`crates/engine/src/markdown/ghost.rs` | `{sha}` | identical |"
    ));
    fixture.write_manifest(&rows);

    let output = fixture.run_check();
    assert!(
        !output.status.success(),
        "a manifest row with no upstream file must fail the check"
    );
    let report = report(&output);
    assert!(
        report.contains("ghost.rs") && report.contains("no upstream file"),
        "the failure names the ghost row: {report}"
    );
    assert!(
        report.contains("expected 18 manifest rows, found 19"),
        "the failure names the row count: {report}"
    );
}

#[test]
fn added_files_exist() {
    let root = workspace_root();
    let document = read_vendored();
    let files = added_files(&document);
    assert!(!files.is_empty(), "the added-file table is not empty");
    for file in files {
        assert!(
            root.join(&file).exists(),
            "VENDORED.md lists a missing file: {file}"
        );
    }
}

#[cfg(unix)]
impl SyncFixture {
    /// Edits `module_00` on disk and records the edit under `deviations`, so
    /// the checksums agree with the bytes and only the local change differs.
    fn record_local_edit(&self, deviations: &str) {
        let name = &self.names[0];
        let vendored = self.engine.join("src/markdown").join(name);
        fs::write(&vendored, b"pub fn example() -> u8 { 2 }\n").unwrap();
        let upstream_sha = sha256(&self.upstream.join("src/markdown").join(name));
        let vendored_sha = sha256(&vendored);
        let mut rows = self.standard_rows();
        rows[0] = format!(
            "| `src/markdown/{name}` | `{upstream_sha}` | \
`crates/engine/src/markdown/{name}` | `{vendored_sha}` | {deviations} |"
        );
        self.write_manifest(&rows);
    }
}

#[cfg(unix)]
#[test]
fn sync_update_force_refuses_an_unrecorded_local_edit() {
    let fixture = SyncFixture::new("force-refused");
    fixture.record_local_edit("identical");
    let name = fixture.names[0].clone();
    let vendored = fixture.engine.join("src/markdown").join(&name);
    let edited = fs::read(&vendored).unwrap();

    let output = fixture.run_sync(&["update", "--yes", "--force"]);
    assert_eq!(output.status.code(), Some(3), "{}", report(&output));
    assert_eq!(
        fs::read(&vendored).unwrap(),
        edited,
        "a refused force must not touch the vendored file"
    );
    let report = report(&output);
    assert!(
        report.contains("no files were written") && report.contains("--discard-unrecorded"),
        "the refusal names the escape hatch: {report}"
    );
}

#[cfg(unix)]
#[test]
fn sync_update_force_discard_flag_overwrites_an_unrecorded_local_edit() {
    let fixture = SyncFixture::new("force-discard");
    fixture.record_local_edit("identical");
    let name = fixture.names[0].clone();
    let vendored = fixture.engine.join("src/markdown").join(&name);

    let output = fixture.run_sync(&["update", "--yes", "--force", "--discard-unrecorded"]);
    assert!(output.status.success(), "{}", report(&output));
    assert_eq!(
        fs::read(&vendored).unwrap(),
        fs::read(fixture.upstream.join("src/markdown").join(&name)).unwrap(),
        "the discard flag copies upstream over the local edit"
    );
}

#[cfg(unix)]
#[test]
fn sync_update_force_names_the_deviations_it_erases() {
    let fixture = SyncFixture::new("force-names");
    fixture.record_local_edit("3, 9");

    let output = fixture.run_sync(&["update", "--yes", "--force"]);
    assert!(output.status.success(), "{}", report(&output));
    let report = report(&output);
    assert!(
        report.contains("  3. Fixture deviation") && report.contains("  9. Fixture deviation"),
        "the warning lists the manifest's deviations for the copied file: {report}"
    );
    assert!(
        !report.contains("  2. Fixture deviation"),
        "the warning does not list deviations the manifest does not record: {report}"
    );
}
