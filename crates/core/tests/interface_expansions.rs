//! The three interface expansions, pinned to one spelling.
//!
//! ADR 0001 settles the words each name stands for, and the docs, the
//! manifests and the source doc comments restate them. Every place in a
//! pinned file that spells an expansion must spell the canonical one; a
//! divergent spelling beside a correct one fails too.
//!
//! A history sentence may quote a retired expansion when its line starts with
//! `retired:`. That line is the one sanctioned quote, the way `naming.rs`
//! allows the remote URL.

use std::fs;

mod common;

use common::{repo_root, working_files};

const TALITA: &str = "Task Administration, Lifecycle & Interaction Terminal Application";
const RAUL: &str = "Review, Assess, Understand & Learn";
const HUGO: &str = "Human-guided Unified Governance & Orchestration";

const ALL_THREE: &[&str] = &[TALITA, RAUL, HUGO];
const TALITA_ONLY: &[&str] = &[TALITA];
const RAUL_ONLY: &[&str] = &[RAUL];

const ROLES: &str = include_str!("../../../docs/roles/roles.md");
const VOCABULARY: &str = include_str!("../../../docs/vocabulary.md");
const DOCS_README: &str = include_str!("../../../docs/README.md");
const AGENTS: &str = include_str!("../../../AGENTS.md");
const NAMING: &str = include_str!("../../../docs/decisions/0001-naming.md");
const README: &str = include_str!("../../../README.md");
const TALITA_MANIFEST: &str = include_str!("../../../crates/talita/Cargo.toml");
const RAUL_MANIFEST: &str = include_str!("../../../crates/raul/Cargo.toml");
const TALITA_SOURCE: &str = include_str!("../../../crates/talita/src/main.rs");

/// The files that restate an expansion, and the ones each must spell. The
/// `bool` is whether the file also names the three interfaces.
const PINNED: &[(&str, &str, &[&str], bool)] = &[
    ("docs/roles/roles.md", ROLES, ALL_THREE, true),
    ("docs/vocabulary.md", VOCABULARY, ALL_THREE, true),
    ("docs/README.md", DOCS_README, ALL_THREE, true),
    ("AGENTS.md", AGENTS, ALL_THREE, true),
    ("docs/decisions/0001-naming.md", NAMING, ALL_THREE, true),
    ("README.md", README, ALL_THREE, true),
    (
        "crates/talita/Cargo.toml",
        TALITA_MANIFEST,
        TALITA_ONLY,
        false,
    ),
    ("crates/raul/Cargo.toml", RAUL_MANIFEST, RAUL_ONLY, false),
    (
        "crates/talita/src/main.rs",
        TALITA_SOURCE,
        TALITA_ONLY,
        false,
    ),
];

/// The short forms prose uses when it names part of an expansion instead of
/// spelling it: talita's job name on its own, and hugo's first word.
const SHORT_FORMS: &[&str] = &["Task Administration", "Human-guided"];

/// The mark that a line quotes a retired expansion as history rather than
/// using it.
const HISTORY: &str = "retired:";

/// The punctuation an expansion may wear without changing its words.
const EDGES: &[char] = &[
    '.', ',', ';', ':', '(', ')', '[', ']', '{', '}', '"', '\'', '`', '“', '”', '‘', '’', '!', '?',
    '—',
];

/// The prose of a document: emphasis marks removed and every run of whitespace
/// folded to one space, so an expansion split by bold letters or a line break
/// reads as the words it spells.
fn prose(text: &str) -> String {
    text.replace('*', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text without its sanctioned history lines.
fn without_history(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().to_lowercase().starts_with(HISTORY))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A token without the punctuation around it.
fn bare(token: &str) -> &str {
    token.trim_matches(EDGES)
}

/// The word an expansion starts with. Prose also uses it alone.
fn lead(full: &str) -> &str {
    bare(
        full.split_whitespace()
            .next()
            .expect("an expansion has words"),
    )
}

fn starts_upper(token: &str) -> bool {
    bare(token).chars().next().is_some_and(char::is_uppercase)
}

fn ends_segment(token: &str) -> bool {
    matches!(token.chars().last(), Some('.' | ';' | '!' | '?'))
}

/// The phrase that starts at `start`, read as one spelling: the expansion's
/// own words and punctuation, stopped by prose or the end of a sentence.
fn spelled(tokens: &[&str], start: usize, full: &str) -> String {
    let mut span = vec![tokens[start]];
    let mut index = start + 1;
    while index < tokens.len() && !ends_segment(tokens[index - 1]) {
        let token = tokens[index];
        let word = bare(token).to_lowercase();
        let in_expansion = full
            .split_whitespace()
            .any(|part| bare(part).to_lowercase() == word);
        if !(starts_upper(token) || bare(token) == "&" || in_expansion) {
            break;
        }
        span.push(token);
        index += 1;
    }
    bare(&span.join(" ")).to_string()
}

/// The doc comments of a source file, where an interface introduces itself.
fn doc_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("///") || line.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_place_spells_the_canonical_expansion() {
    for (path, text, expansions, three_interfaces) in PINNED {
        let folded = prose(&without_history(text));
        if *three_interfaces {
            assert!(
                folded.to_lowercase().contains("three interfaces"),
                "{path} does not name three interfaces"
            );
        }
        for expansion in *expansions {
            assert!(
                folded.contains(expansion),
                "{path} does not spell {expansion}"
            );
        }

        // Agreement is per place, not per file: a divergent spelling beside a
        // correct one must fail.
        let tokens: Vec<&str> = folded.split_whitespace().collect();
        for (index, token) in tokens.iter().enumerate() {
            for expansion in ALL_THREE {
                if bare(token) != lead(expansion) {
                    continue;
                }
                let Some(next) = tokens.get(index + 1) else {
                    continue;
                };
                if !(starts_upper(next) || bare(next) == "&") {
                    continue;
                }
                let candidate = spelled(&tokens, index, expansion);
                assert!(
                    candidate == *expansion || SHORT_FORMS.contains(&candidate.as_str()),
                    "{path}: {candidate:?} is not how {expansion:?} is spelled"
                );
            }
        }
    }
}

#[test]
fn the_talita_doc_comment_spells_its_expansion() {
    let comments = prose(&doc_comments(TALITA_SOURCE));
    assert!(
        comments.contains(TALITA),
        "the talita doc comment does not spell {TALITA}"
    );
}

/// The retired interface expansions, built from their parts so that this test
/// does not itself contain the strings it forbids.
fn retired_expansions() -> Vec<String> {
    vec![
        ["Task", "Access"].join(" "),
        ["Lane", "Interaction"].join(" "),
        ["Understand", "Lane"].join(" "),
        ["Review,", "Approve,", "Understand", "Lane"].join(" "),
    ]
}

#[test]
fn no_file_still_spells_a_retired_interface_expansion() {
    let root = repo_root();
    let retired: Vec<String> = retired_expansions()
        .iter()
        .map(|old| old.to_lowercase())
        .collect();

    let mut offenders = Vec::new();
    for path in working_files(&root) {
        let bytes = fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let text = without_history(&String::from_utf8_lossy(&bytes));
        let folded = prose(&text).to_lowercase();
        for old in &retired {
            if folded.contains(old.as_str()) {
                let name = path
                    .strip_prefix(&root)
                    .expect("a path under the repository")
                    .display()
                    .to_string();
                offenders.push(format!("{name}: {old}"));
                break;
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "a retired interface expansion is still in the working tree:\n{}",
        offenders.join("\n")
    );
}
