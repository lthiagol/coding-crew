//! The three interface expansions and descriptions, pinned to one spelling.
//!
//! ADR 0001 settles the words each name stands for and what each interface
//! is, and the docs, the manifests and the source doc comments restate them.
//! Every place in a pinned file that spells an expansion or describes an
//! interface must spell the canonical one; a divergent spelling beside a
//! correct one fails too, and a lowercase lead word is a divergence.
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
const HUGO_ONLY: &[&str] = &[HUGO];

const ROLES: &str = include_str!("../../../docs/roles/roles.md");
const VOCABULARY: &str = include_str!("../../../docs/vocabulary.md");
const DOCS_README: &str = include_str!("../../../docs/README.md");
const AGENTS: &str = include_str!("../../../AGENTS.md");
const NAMING: &str = include_str!("../../../docs/decisions/0001-naming.md");
const README: &str = include_str!("../../../README.md");
const SCREENS: &str = include_str!("../../../docs/tui/screens.md");
const HUGO_RECORD: &str = include_str!("../../../docs/hugo/design-record.md");
const TALITA_MANIFEST: &str = include_str!("../../../crates/talita/Cargo.toml");
const RAUL_MANIFEST: &str = include_str!("../../../crates/raul/Cargo.toml");
const TALITA_SOURCE: &str = include_str!("../../../crates/talita/src/main.rs");
const RAUL_SOURCE: &str = include_str!("../../../crates/raul/src/main.rs");
const TASK_AUTHORING: &str = include_str!("../../../docs/skills/task-authoring.md");

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

/// The places outside the expansion-pinned files that describe an interface
/// and are asserted to state the canonical description. The working-tree
/// scan below fails on a divergent description in any file.
const DESCRIBED: &[(&str, &str, &[&str])] = &[
    ("docs/tui/screens.md", SCREENS, RAUL_ONLY),
    ("docs/hugo/design-record.md", HUGO_RECORD, HUGO_ONLY),
    ("crates/raul/src/main.rs", RAUL_SOURCE, RAUL_ONLY),
    ("docs/skills/task-authoring.md", TASK_AUTHORING, TALITA_ONLY),
];

/// What each interface is, in the words every place uses.
const DESCRIPTIONS: &[(&str, &str)] = &[
    (TALITA, "the CLI tool"),
    (RAUL, "the TUI for the work"),
    (HUGO, "the TUI for the flow"),
];

/// The canonical description of an expansion.
fn description_of(expansion: &str) -> &'static str {
    DESCRIPTIONS
        .iter()
        .find(|(name, _)| *name == expansion)
        .map(|(_, description)| *description)
        .expect("every interface has a description")
}

/// The continuations a description may take after "the TUI" and "the CLI".
/// "the TUI binary" names a binary; "the TUIs" is the plural.
const TUI_CONTINUATIONS: &[&str] = &[
    " for the work",
    " for the flow",
    " binary for the work",
    " binary for the flow",
    "s",
];
const CLI_CONTINUATIONS: &[&str] = &[" tool", " binary"];

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

/// The prose of a document: emphasis marks removed, doc-comment markers
/// dropped, and every run of whitespace folded to one space, so an expansion
/// split by bold letters, a comment's line break or a line break reads as the
/// words it spells.
fn prose(text: &str) -> String {
    text.lines()
        .map(|line| {
            let line = line.trim_start();
            line.strip_prefix("///")
                .or_else(|| line.strip_prefix("//!"))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
        .replace('*', "")
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

/// Whether `token` is the word the expansion starts with, in any case: a
/// lowercase lead word is a divergence, not a different word.
fn is_lead(token: &str, expansion: &str) -> bool {
    bare(token).eq_ignore_ascii_case(lead(expansion))
}

/// Whether `token` is the expansion's second word. A spelling only starts
/// with the lead word and its second word; the words after it may differ.
fn is_second(token: &str, expansion: &str) -> bool {
    expansion
        .split_whitespace()
        .nth(1)
        .is_some_and(|word| bare(token).eq_ignore_ascii_case(bare(word)))
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

/// Every expansion spelled wrongly in a folded text.
fn divergent_expansions(folded: &str) -> Vec<String> {
    let tokens: Vec<&str> = folded.split_whitespace().collect();
    let mut divergences = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        for expansion in ALL_THREE {
            if !is_lead(token, expansion) {
                continue;
            }
            let Some(next) = tokens.get(index + 1) else {
                continue;
            };
            if !is_second(next, expansion) {
                continue;
            }
            let candidate = spelled(&tokens, index, expansion);
            if candidate != *expansion && !SHORT_FORMS.contains(&candidate.as_str()) {
                divergences.push(candidate);
            }
        }
    }
    divergences
}

/// The words that turn "the TUI" or "the CLI" into a description that must be
/// the canonical phrase. Other continuations are not descriptions.
const DESCRIPTION_CONTINUATIONS: &[&str] = &["of", "for", "that", "which"];

/// Every description spelled wrongly in a folded text.
///
/// Only the canonical continuations are allowed after "the TUI" and "the
/// CLI"; "the TUI binary" names a binary and "the TUIs" is the plural. A
/// phrase that runs into one of the description prepositions and is not
/// canonical fails, whether or not an interface name stands on the same
/// line.
fn divergent_descriptions(folded: &str) -> Vec<String> {
    let lower = folded.to_lowercase();
    let mut divergences = Vec::new();
    for (phrase, allowed) in [
        ("the tui", TUI_CONTINUATIONS),
        ("the cli", CLI_CONTINUATIONS),
    ] {
        for (index, _) in lower.match_indices(phrase) {
            let after = &lower[index + phrase.len()..];
            if allowed
                .iter()
                .any(|continuation| after.starts_with(continuation))
            {
                continue;
            }
            let next = after
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|character: char| !character.is_alphanumeric());
            if !DESCRIPTION_CONTINUATIONS.contains(&next) {
                continue;
            }
            let tail = lower[index..].split('.').next().unwrap_or("").trim();
            divergences.push(tail.to_string());
        }
    }
    divergences
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
            let description = description_of(expansion);
            assert!(
                folded.to_lowercase().contains(&description.to_lowercase()),
                "{path} does not describe {expansion:?} as {description:?}"
            );
        }

        // Agreement is per place, not per file: a divergent spelling beside a
        // correct one must fail.
        let divergences = divergent_expansions(&folded);
        assert!(
            divergences.is_empty(),
            "{path}: {divergences:?} is not how the interfaces are spelled"
        );
        let descriptions = divergent_descriptions(&folded);
        assert!(
            descriptions.is_empty(),
            "{path}: {descriptions:?} is not how the interfaces are described"
        );
    }
}

#[test]
fn a_place_that_describes_an_interface_pins_its_description() {
    for (path, text, descriptions) in DESCRIBED {
        let folded = prose(&without_history(text));
        for expansion in *descriptions {
            let description = description_of(expansion);
            assert!(
                folded.to_lowercase().contains(&description.to_lowercase()),
                "{path} does not describe {expansion:?} as {description:?}"
            );
        }
        let divergences = divergent_descriptions(&folded);
        assert!(
            divergences.is_empty(),
            "{path}: {divergences:?} is not how the interfaces are described"
        );
    }
}

#[test]
fn no_file_describes_an_interface_divergently() {
    // The pin is over the working tree, not a hand-picked list: a divergent
    // description in a file nobody remembered to add fails here. TODO.txt is
    // the orchestrator's history ledger and quotes wording that has been
    // replaced, so it is the one file outside the scan.
    let root = repo_root();
    let mut offenders = Vec::new();
    for path in working_files(&root) {
        let name = path
            .strip_prefix(&root)
            .expect("a path under the repository");
        if name == std::path::Path::new("TODO.txt") {
            continue;
        }
        let bytes = fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let text = without_history(&String::from_utf8_lossy(&bytes));
        for divergence in divergent_descriptions(&prose(&text)) {
            offenders.push(format!("{}: {divergence}", name.display()));
        }
    }
    assert!(
        offenders.is_empty(),
        "a divergent interface description is in the working tree:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn a_lowercase_lead_word_is_a_divergence() {
    // The capital is part of the spelling: the lowercase words are prose that
    // borrowed the name, not the expansion.
    let divergences = divergent_expansions(&prose(
        "talita is task administration, lifecycle & interaction terminal application",
    ));
    assert_eq!(
        divergences,
        vec!["task administration, lifecycle & interaction terminal application"],
        "a lowercase lead word passed"
    );
    assert!(
        divergent_expansions(&prose(TALITA)).is_empty(),
        "the canonical expansion was refused"
    );
}

#[test]
fn a_divergent_description_beside_a_correct_one_fails() {
    // Built from parts so that this file does not itself spell the phrase the
    // working-tree scan forbids.
    let wrong = format!("the {} of coding-crew", "TUI");
    let page = format!("raul, the TUI for the work. Some pages still said {wrong}.");
    assert_eq!(
        divergent_descriptions(&prose(&page)),
        vec![wrong.to_lowercase()],
        "a divergent description passed beside the canonical one"
    );
    // The recorded defect stood on a line that named no interface: the scan
    // reads the description wherever it is spelled.
    assert!(
        !divergent_descriptions(&prose(&format!("The {} is the screen spec.", wrong))).is_empty(),
        "a divergent description with no interface name on its line passed"
    );
    assert!(
        divergent_descriptions(&prose("raul, the TUI for the work.")).is_empty(),
        "the canonical description was refused"
    );
    assert!(
        divergent_descriptions(&prose(
            "talita is the CLI tool; its binary is the CLI binary"
        ))
        .is_empty(),
        "a description of a binary was refused"
    );
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
