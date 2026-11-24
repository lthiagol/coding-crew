//! The docs' command surface: no doc outside the two that name the old verbs
//! on purpose names a flat verb. The grammar is noun, then verb (ADR 0012).

use std::fs;
use std::path::{Path, PathBuf};

/// The flat verbs ADR 0012 replaced.
const FLAT_VERBS: [&str; 22] = [
    "create",
    "list",
    "show",
    "groom",
    "approve",
    "ask",
    "answer",
    "set-meta",
    "reopen",
    "resume",
    "block",
    "defer",
    "cancel",
    "unblock",
    "undefer",
    "add-step",
    "review",
    "finding",
    "comment",
    "pass-review",
    "request-changes",
    "escalate",
];

/// The two docs that name the old verbs on purpose: the renaming table and
/// the drift table.
const KEEPERS: [&str; 2] = ["docs/talita/commands.md", "docs/talita/decisions.md"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn markdown_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            markdown_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            found.push(path);
        }
    }
}

#[test]
fn no_doc_names_a_flat_verb() {
    let root = repo_root();
    let mut files = Vec::new();
    markdown_files(&root.join("docs"), &mut files);
    markdown_files(&root.join("crates/hugo/skills"), &mut files);
    assert!(
        files.len() > 20,
        "the docs were not found: {} files",
        files.len()
    );

    let keepers: Vec<PathBuf> = KEEPERS.iter().map(|path| root.join(path)).collect();
    let mut hits = Vec::new();
    for file in &files {
        if keepers.contains(file) {
            continue;
        }
        let text =
            fs::read_to_string(file).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        for (number, line) in text.lines().enumerate() {
            for verb in FLAT_VERBS {
                if names_flat_verb(line, verb) {
                    let relative = file.strip_prefix(&root).unwrap_or(file);
                    hits.push(format!(
                        "{}:{}: talita {verb} — {}",
                        relative.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "{} doc lines name a flat verb:\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// Whether `line` names `verb` as a flat verb: `talita <verb>` with a word
/// boundary after it. Two nouns are also flat verbs, so the grammar's own use
/// is not a hit: `talita finding resolve` and `talita comment add`.
fn names_flat_verb(line: &str, verb: &str) -> bool {
    let needle = format!("talita {verb}");
    let mut from = 0;
    while let Some(offset) = line[from..].find(&needle) {
        let start = from + offset;
        let end = start + needle.len();
        if bounded(line, start, end) && !is_noun_verb(line[end..].trim_start(), verb) {
            return true;
        }
        from = end;
    }
    false
}

fn bounded(line: &str, start: usize, end: usize) -> bool {
    !line[..start].chars().next_back().is_some_and(is_word)
        && !line[end..].chars().next().is_some_and(is_word)
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn is_noun_verb(rest: &str, verb: &str) -> bool {
    let next = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
    match verb {
        "finding" => matches!(next, "open" | "resolve" | "dismiss"),
        "comment" => next == "add",
        _ => false,
    }
}

/// Every `talita <noun> <verb>` a skill names is a pair the command reference
/// documents, and each skill names at least one. The skills are what the
/// agents type, so a call the grammar does not have is a call that fails.
#[test]
fn every_talita_call_in_a_skill_is_in_the_grammar() {
    let root = repo_root();
    let commands =
        fs::read_to_string(root.join("docs/talita/commands.md")).expect("the command reference");
    let mut skills = Vec::new();
    markdown_files(&root.join("crates/hugo/skills"), &mut skills);
    assert_eq!(skills.len(), 3, "the skills were not found: {skills:?}");

    let mut hits = Vec::new();
    for file in &skills {
        let text =
            fs::read_to_string(file).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        let relative = file.strip_prefix(&root).unwrap_or(file);
        let mut calls = 0;
        for (number, line) in text.lines().enumerate() {
            for (noun, verb) in talita_calls(line) {
                calls += 1;
                if !documents(&commands, noun, verb) {
                    hits.push(format!(
                        "{}:{}: talita {noun} {verb} — {}",
                        relative.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
        assert!(calls > 0, "{} names no talita call", relative.display());
    }

    assert!(
        hits.is_empty(),
        "{} skill lines name a talita call the grammar does not document:\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// The `talita <noun> <verb>` pairs a line names. A flag after `talita`, or a
/// word that is not a lowercase word, names no pair.
fn talita_calls(line: &str) -> Vec<(&str, &str)> {
    const WORD: &str = "talita ";
    let mut calls = Vec::new();
    let mut from = 0;
    while let Some(offset) = line[from..].find(WORD) {
        let start = from + offset;
        from = start + WORD.len();
        if !bounded(line, start, start + WORD.len() - 1) {
            continue;
        }
        let mut words = line[from..]
            .split_whitespace()
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-'));
        if let (Some(noun), Some(verb)) = (words.next(), words.next())
            && is_lower_word(noun)
            && is_lower_word(verb)
        {
            calls.push((noun, verb));
        }
    }
    calls
}

/// A lowercase word, as the grammar's nouns and verbs are written.
fn is_lower_word(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_lowercase())
        && word.chars().all(|c| c.is_ascii_lowercase() || c == '-')
}

/// Whether the command reference names `noun verb` as a whole phrase.
fn documents(commands: &str, noun: &str, verb: &str) -> bool {
    let pair = format!("{noun} {verb}");
    let mut from = 0;
    while let Some(offset) = commands[from..].find(&pair) {
        let start = from + offset;
        let end = start + pair.len();
        if bounded(commands, start, end) {
            return true;
        }
        from = end;
    }
    false
}

#[test]
fn the_scan_reads_a_talita_call_as_a_noun_and_a_verb() {
    assert_eq!(
        talita_calls("`talita task finish TSK04`"),
        [("task", "finish")]
    );
    assert_eq!(
        talita_calls("talita claim take <TSK> --lease 2h; then talita evidence run <TSK>"),
        [("claim", "take"), ("evidence", "run")]
    );
    assert!(talita_calls("talita --help").is_empty());
    assert!(talita_calls("notalita task finish").is_empty());
    assert!(talita_calls("talita Task Finish").is_empty());
    assert!(documents("`task finish <TSK>`", "task", "finish"));
    assert!(!documents("`task finished <TSK>`", "task", "finish"));
    assert!(documents("`finding resolve <TSK>`", "finding", "resolve"));
    assert!(!documents("`begin finding resolve`", "finding", "re"));
}

#[test]
fn the_scan_tells_a_flat_verb_from_the_grammar() {
    assert!(names_flat_verb("`talita create \"A task\"`", "create"));
    assert!(names_flat_verb("talita create|list|show", "create"));
    assert!(!names_flat_verb("talita task create --title A", "create"));
    assert!(!names_flat_verb("a talita creates nothing", "create"));
    assert!(!names_flat_verb("notalita create", "create"));
    assert!(!names_flat_verb(
        "talita finding resolve TSK01 FIND-01",
        "finding"
    ));
    assert!(names_flat_verb(
        "talita finding TSK01 --severity high",
        "finding"
    ));
    assert!(!names_flat_verb(
        "talita comment add TSK01 FIND-01 --body hi",
        "comment"
    ));
    assert!(names_flat_verb(
        "talita comment TSK01 FIND-01 \"hi\"",
        "comment"
    ));
}
