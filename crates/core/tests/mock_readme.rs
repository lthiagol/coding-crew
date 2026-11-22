//! The mock folder's README, matched to the fixtures it describes.
//!
//! The folder was rewritten from lorem ipsum to real work; its README is the
//! folder's own documentation. Every count and phase shape it states is
//! recomputed here from the fixtures, and a count sentence with no claim
//! fails the gate. This is the pin the theme catalog unit added for
//! `docs/tui/palettes.md`, applied to `mock-data/README.md`.

use std::path::PathBuf;

use coding_crew_core::{Status, StepStatus, Store, Task, Verdict};

const README: &str = include_str!("../../../mock-data/README.md");

/// The instant the fixtures are read with: a claim is live at this clock.
const NOW: &str = "2026-11-19T08:00:00Z";

const PHASES: [(&str, Status); 8] = [
    ("draft", Status::Draft),
    ("groomed", Status::Groomed),
    ("approved", Status::Approved),
    ("executing", Status::Executing),
    ("code-review", Status::CodeReview),
    ("remediating", Status::Remediating),
    ("needs-human", Status::NeedsHuman),
    ("done", Status::Done),
];

fn tasks() -> Vec<Task> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("mock-data/coding-crew/work");
    Store::new(root).list().expect("listing the mock tasks")
}

fn of_status(tasks: &[Task], status: Status) -> Vec<&Task> {
    tasks.iter().filter(|task| task.status == status).collect()
}

/// The number a word states, up to twelve.
fn word(n: usize) -> String {
    const WORDS: [&str; 13] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven", "twelve",
    ];
    WORDS
        .get(n)
        .map(|word| (*word).to_string())
        .unwrap_or_else(|| n.to_string())
}

/// The number a word states, including the tens and their compounds.
fn number_word(word: &str) -> Option<usize> {
    const UNITS: [(&str, usize); 20] = [
        ("zero", 0),
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
        ("thirteen", 13),
        ("fourteen", 14),
        ("fifteen", 15),
        ("sixteen", 16),
        ("seventeen", 17),
        ("eighteen", 18),
        ("nineteen", 19),
    ];
    const TENS: [(&str, usize); 8] = [
        ("twenty", 20),
        ("thirty", 30),
        ("forty", 40),
        ("fifty", 50),
        ("sixty", 60),
        ("seventy", 70),
        ("eighty", 80),
        ("ninety", 90),
    ];
    let mut total = 0;
    for part in word.split('-') {
        let value = UNITS
            .iter()
            .chain(TENS.iter())
            .find(|(name, _)| *name == part)
            .map(|(_, value)| *value)
            .or(match part {
                "hundred" => Some(100),
                "thousand" => Some(1000),
                _ => None,
            })?;
        total += value;
    }
    Some(total)
}

/// The text of a sentence with its code spans removed: a number inside a code
/// span names a task or a path, it does not state a count.
fn without_code_spans(sentence: &str) -> String {
    let mut text = String::new();
    let mut in_code = false;
    for character in sentence.chars() {
        if character == '`' {
            in_code = !in_code;
        } else if !in_code {
            text.push(character);
        }
    }
    text
}

/// The text of a sentence with its link targets removed: a link points at a
/// section, and its anchor is not a count.
fn without_link_targets(sentence: &str) -> String {
    let mut text = String::new();
    let mut characters = sentence.chars().peekable();
    while let Some(character) = characters.next() {
        if character == ']' && characters.peek() == Some(&'(') {
            characters.next();
            for character in characters.by_ref() {
                if character == ')' {
                    break;
                }
            }
        } else {
            text.push(character);
        }
    }
    text
}

/// The number tokens a sentence states, in order: a run of alphanumerics,
/// dots and hyphens that carries a digit, or a number word.
fn number_tokens(sentence: &str) -> Vec<String> {
    let text = without_link_targets(&without_code_spans(sentence));
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '.' || character == '-')
    })
    .map(|token| token.trim_matches(|character: char| character == '.' || character == '-'))
    .filter(|token| {
        !token.is_empty()
            && (token.chars().any(|character| character.is_ascii_digit())
                || number_word(&token.to_lowercase()).is_some())
    })
    .map(str::to_string)
    .collect()
}

/// The numbers a sentence states, in order: digits and number words alike.
fn numbers_in(sentence: &str) -> Vec<usize> {
    number_tokens(sentence)
        .iter()
        .filter_map(|token| {
            token
                .parse::<usize>()
                .ok()
                .or_else(|| number_word(&token.to_lowercase()))
        })
        .collect()
}

/// Push the sentences of one paragraph, split on `.`, `!` and `?`.
fn flush_paragraph(paragraph: &mut String, sentences: &mut Vec<String>) {
    let text = std::mem::take(paragraph);
    let mut start = 0;
    for (i, byte) in text.bytes().enumerate() {
        if !matches!(byte, b'.' | b'!' | b'?') {
            continue;
        }
        let rest = &text[i + 1..];
        let after = rest.trim_start_matches(['*', '_', '`', '"', '\'']);
        let next = after.chars().next();
        if next.is_none() || next.is_some_and(char::is_whitespace) {
            let sentence = text[start..=i].trim();
            if !sentence.is_empty() {
                sentences.push(sentence.to_string());
            }
            start = i + 1;
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        sentences.push(tail.to_string());
    }
}

/// The prose sentences of the README: tables, headings and fenced code are
/// data the table pins read, not count statements.
fn prose_sentences(readme: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut paragraph = String::new();
    let mut in_fence = false;
    for line in readme.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            flush_paragraph(&mut paragraph, &mut sentences);
            in_fence = !in_fence;
            continue;
        }
        if in_fence || line.is_empty() || line.starts_with('|') || line.starts_with('#') {
            flush_paragraph(&mut paragraph, &mut sentences);
            continue;
        }
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(line);
    }
    flush_paragraph(&mut paragraph, &mut sentences);
    sentences
}

/// The numbers the README states that are not counts: the section reference.
/// Every other number word or digit a prose sentence states is a count the
/// fixtures decide.
const NOT_COUNTS: [&str; 1] = ["11.2"];

/// Whether a sentence states a count: a number word or a digit that is not a
/// listed non-count. A sentence is a count sentence by its number, not by a
/// noun that happens to sit near it.
fn states_a_count(sentence: &str) -> bool {
    number_tokens(sentence)
        .iter()
        .any(|token| !NOT_COUNTS.contains(&token.as_str()))
}

/// The cells of the phase table row that starts with `` `phase` ``.
fn phase_row(readme: &str, phase: &str) -> (usize, String) {
    let prefix = format!("| `{phase}` |");
    let line = readme
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))
        .unwrap_or_else(|| panic!("the README has no {phase} row"));
    let cells: Vec<&str> = line.split('|').map(str::trim).collect();
    assert!(
        cells.len() >= 4,
        "the {phase} row is not a phase table row: {line}"
    );
    let count = cells[2]
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("the {phase} row has no count: {line}"));
    (count, cells[3].to_string())
}

/// The second cell of the runtime table row whose first cell is `name`.
fn runtime_row(readme: &str, name: &str) -> String {
    let prefix = format!("| {name} |");
    let line = readme
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))
        .unwrap_or_else(|| panic!("the README has no {name} row"));
    let cells: Vec<&str> = line.split('|').map(str::trim).collect();
    assert!(cells.len() >= 3, "the {name} row has no value: {line}");
    cells[2].to_string()
}

/// The four dependency shapes the README says are covered, recomputed from
/// the fixtures. A shape that no longer holds fails here.
fn dependency_shapes(tasks: &[Task]) -> Vec<&'static str> {
    let by_id: std::collections::HashMap<&str, &Task> =
        tasks.iter().map(|task| (task.id.as_str(), task)).collect();

    // Forward edges: the phase tasks past approved wait on done work whose
    // ids are higher than their own.
    for task in tasks.iter().filter(|task| {
        matches!(
            task.status,
            Status::Executing
                | Status::CodeReview
                | Status::Remediating
                | Status::NeedsHuman
                | Status::Done
        )
    }) {
        if task.status == Status::Done {
            continue;
        }
        for dependency in &task.depends_on {
            let dependency = by_id[dependency.as_str()];
            assert_eq!(
                dependency.status,
                Status::Done,
                "{} waits on {} which is {:?}",
                task.id,
                dependency.id,
                dependency.status
            );
            assert!(
                dependency.id > task.id,
                "{} waits on {} whose id is lower",
                task.id,
                dependency.id
            );
        }
    }

    // Fan-in: the last done task waits on three.
    let mut done: Vec<&Task> = of_status(tasks, Status::Done);
    done.sort_by(|one, other| one.id.cmp(&other.id));
    let last_done = done.last().expect("a done task");
    assert_eq!(
        last_done.depends_on.len(),
        3,
        "the last done task waits on {} others",
        last_done.depends_on.len()
    );

    // Chains: the done tasks form a chain among themselves.
    for pair in done.windows(2) {
        let (before, after) = (pair[0], pair[1]);
        assert!(
            after
                .depends_on
                .iter()
                .any(|dependency| dependency == &before.id),
            "{} does not wait on {}",
            after.id,
            before.id
        );
    }

    // Waiting on a blocked task: a task waits on one that carries the
    // blocked overlay.
    let blocked: Vec<&str> = tasks
        .iter()
        .filter(|task| task.overlays.blocked.is_some())
        .map(|task| task.id.as_str())
        .collect();
    assert!(
        tasks.iter().any(|task| task
            .depends_on
            .iter()
            .any(|dependency| blocked.contains(&dependency.as_str()))),
        "no task waits on a blocked one"
    );

    vec![
        "forward edges",
        "fan-in",
        "chains",
        "waiting on a blocked task",
    ]
}

/// The reasons an approved task is not claimable.
fn reasons_of(task: &Task, by_id: &std::collections::HashMap<&str, &Task>) -> Vec<usize> {
    let mut reasons = Vec::new();
    if task
        .depends_on
        .iter()
        .any(|dependency| by_id[dependency.as_str()].status != Status::Done)
    {
        reasons.push(0);
    }
    if task.overlays.blocked.is_some() {
        reasons.push(1);
    }
    if task
        .claim
        .as_ref()
        .is_some_and(|claim| !claim.lease_expired(NOW))
    {
        reasons.push(2);
    }
    reasons
}

/// The three reasons an approved task is not claimable, and how many
/// fixtures cover each. The README says three, one each.
fn reason_counts(tasks: &[Task]) -> Vec<usize> {
    let by_id: std::collections::HashMap<&str, &Task> =
        tasks.iter().map(|task| (task.id.as_str(), task)).collect();
    let mut counts = [0usize; 3];
    for task in of_status(tasks, Status::Approved) {
        let reasons = reasons_of(task, &by_id);
        // The queue holds an approved task with no reason against it; the
        // held ones are held by one reason alone.
        assert!(
            reasons.len() <= 1,
            "{} is held by {} reasons, not one",
            task.id,
            reasons.len()
        );
        for reason in reasons {
            counts[reason] += 1;
        }
    }
    assert_eq!(counts, [1, 1, 1], "the reasons' fixtures moved");
    counts.to_vec()
}

/// The most reasons any approved fixture is held by: the README says each
/// held task carries one.
fn most_reasons(tasks: &[Task]) -> usize {
    let by_id: std::collections::HashMap<&str, &Task> =
        tasks.iter().map(|task| (task.id.as_str(), task)).collect();
    of_status(tasks, Status::Approved)
        .iter()
        .map(|task| reasons_of(task, &by_id).len())
        .max()
        .unwrap_or(0)
}

/// The shape the phase row states, rebuilt from the fixtures.
fn recomputed_shape(phase: &str, tasks: &[Task]) -> String {
    let status = PHASES
        .iter()
        .find(|(name, _)| *name == phase)
        .map(|(_, status)| *status)
        .expect("a phase");
    let fixtures = of_status(tasks, status);
    assert!(!fixtures.is_empty(), "no fixture for {phase}");

    match phase {
        "draft" => {
            for task in fixtures.iter().filter(|task| task.promoted_from.is_none()) {
                assert!(
                    task.intent.outcome.is_empty()
                        && task.scope.in_scope.is_empty()
                        && task.acceptance_criteria.is_empty(),
                    "{} is a draft with more than a title",
                    task.id
                );
            }
            let plain = fixtures
                .iter()
                .filter(|task| task.parent.is_none() && task.promoted_from.is_none())
                .count();
            let promoted = fixtures
                .iter()
                .filter(|task| task.promoted_from.is_some())
                .count();
            let children: Vec<&Task> = fixtures
                .iter()
                .copied()
                .filter(|task| task.parent.is_some())
                .collect();
            let parent = children
                .first()
                .and_then(|task| task.parent.clone())
                .expect("a child draft");
            assert!(
                children
                    .iter()
                    .all(|task| task.parent.as_deref() == Some(parent.as_str())),
                "the child drafts have more than one parent"
            );
            let parent_task = tasks
                .iter()
                .find(|task| task.id == parent)
                .unwrap_or_else(|| panic!("{parent} names no task"));
            assert_eq!(
                parent_task.status,
                Status::Approved,
                "{parent} is not approved"
            );
            format!(
                "title only — {} plain drafts, {} promoted bug and {} children of `{parent}`",
                word(plain),
                word(promoted),
                word(children.len())
            )
        }
        "groomed" => {
            for task in &fixtures {
                assert!(
                    !task.intent.outcome.is_empty() && !task.intent.problem.is_empty(),
                    "{} has no intent",
                    task.id
                );
                assert!(
                    !task.scope.in_scope.is_empty() && !task.scope.out_of_scope.is_empty(),
                    "{} has no scope",
                    task.id
                );
                assert!(
                    !task.acceptance_criteria.is_empty(),
                    "{} has no criteria",
                    task.id
                );
            }
            let blocked = fixtures
                .iter()
                .filter(|task| task.overlays.blocked.is_some())
                .count();
            let carries = if blocked == 1 { "carries" } else { "carry" };
            format!(
                "intent, scope, criteria; {} {carries} a `blocked` overlay",
                word(blocked)
            )
        }
        "approved" => {
            for task in &fixtures {
                assert!(
                    task.criteria_covered(),
                    "{} has uncovered criteria",
                    task.id
                );
                assert!(
                    task.steps_have_done_when(),
                    "{} has a step without done_when",
                    task.id
                );
            }
            let blocked = fixtures
                .iter()
                .filter(|task| task.overlays.blocked.is_some())
                .count();
            let held = fixtures
                .iter()
                .filter(|task| {
                    task.claim
                        .as_ref()
                        .is_some_and(|claim| !claim.lease_expired(NOW))
                })
                .count();
            let carries = if blocked == 1 { "carries" } else { "carry" };
            format!(
                "criteria covered by steps, every step has `done_when`; {} {carries} a `blocked` overlay, {} a claim held elsewhere",
                word(blocked),
                word(held)
            )
        }
        "executing" => {
            for task in &fixtures {
                assert!(
                    task.claim
                        .as_ref()
                        .is_some_and(|claim| !claim.lease_expires_at.is_empty()),
                    "{} is not claimed with a lease",
                    task.id
                );
                assert!(
                    task.acceptance_criteria
                        .iter()
                        .any(|criterion| !criterion.evidence.is_empty()),
                    "{} has no evidence",
                    task.id
                );
            }
            let defers = fixtures
                .iter()
                .filter(|task| {
                    task.acceptance_criteria
                        .iter()
                        .any(|criterion| criterion.deferral.is_some())
                })
                .count();
            format!(
                "claimed with a lease, evidence attached; {} defers a criterion",
                word(defers)
            )
        }
        "code-review" => {
            for task in &fixtures {
                assert!(
                    task.steps
                        .iter()
                        .all(|step| step.status == StepStatus::Done),
                    "{} has unsettled steps",
                    task.id
                );
                assert!(task.self_rounds() > 0, "{} has no self round", task.id);
                assert!(task.open_findings() > 0, "{} has no open finding", task.id);
            }
            let open = fixtures
                .iter()
                .map(|task| task.open_findings())
                .min()
                .expect("a code-review fixture");
            format!(
                "steps settled, self round filed, {} open finding",
                word(open)
            )
        }
        "remediating" => {
            for task in &fixtures {
                assert!(
                    task.external_rounds() > 0,
                    "{} has no external round",
                    task.id
                );
                assert_eq!(
                    task.review.verdict,
                    Some(Verdict::ChangesNeeded),
                    "{} is not changes-needed",
                    task.id
                );
                assert!(task.open_findings() > 0, "{} has no open finding", task.id);
            }
            "external round, `changes-needed`, findings open".to_string()
        }
        "needs-human" => {
            let rounds = fixtures[0].external_rounds();
            for task in &fixtures {
                assert_eq!(
                    task.external_rounds(),
                    task.review.max_rounds as usize,
                    "{} has not exhausted its budget",
                    task.id
                );
                assert_eq!(
                    task.external_rounds(),
                    rounds,
                    "{} spent a different budget",
                    task.id
                );
            }
            format!("{} external rounds exhausted", word(rounds))
        }
        "done" => {
            for task in &fixtures {
                assert_eq!(
                    task.review.verdict,
                    Some(Verdict::Ok),
                    "{} has no clean verdict",
                    task.id
                );
                assert_eq!(task.open_findings(), 0, "{} has an open finding", task.id);
            }
            "clean verdict, no open findings".to_string()
        }
        _ => unreachable!("a phase without a shape"),
    }
}

/// Every disagreement between a phase row and the fixtures, so a planted
/// wrong count can be caught without panicking the checker.
fn phase_errors(readme: &str, tasks: &[Task]) -> Vec<String> {
    let mut errors = Vec::new();
    for (phase, status) in PHASES {
        let (stated, shape) = phase_row(readme, phase);
        let fixtures = of_status(tasks, status);
        if stated != fixtures.len() {
            errors.push(format!(
                "{phase} says {stated} but the fixtures hold {}",
                fixtures.len()
            ));
        }
        let expected = recomputed_shape(phase, tasks);
        if shape != expected {
            errors.push(format!(
                "{phase} says \"{shape}\" and the fixtures say \"{expected}\""
            ));
        }
    }
    errors
}

/// The prose count sentences and the numbers each one should state.
fn count_claims(tasks: &[Task]) -> Vec<(&'static str, Vec<usize>)> {
    let shapes = dependency_shapes(tasks);
    let reasons = reason_counts(tasks);
    let mut done: Vec<&Task> = of_status(tasks, Status::Done);
    done.sort_by(|one, other| one.id.cmp(&other.id));
    let last_done = done.last().expect("a done task");
    vec![
        ("Four shapes are covered", vec![shapes.len()]),
        (
            "most tasks wait on two others",
            vec![2, last_done.depends_on.len()],
        ),
        (
            "can be held by three things",
            vec![reasons.len(), *reasons.iter().min().expect("a reason")],
        ),
        (
            "each is held by one reason alone",
            vec![most_reasons(tasks)],
        ),
    ]
}

/// Every disagreement between the README's prose counts and the fixtures, so
/// a planted count can be caught without panicking the checker.
fn count_errors(readme: &str, tasks: &[Task]) -> Vec<String> {
    let claims = count_claims(tasks);
    let sentences = prose_sentences(readme);
    let mut errors = Vec::new();
    let mut covered = Vec::new();
    for (anchor, expected) in &claims {
        let sentence = match sentences.iter().find(|sentence| sentence.contains(anchor)) {
            Some(sentence) => sentence,
            None => {
                errors.push(format!("the README no longer states {anchor}"));
                continue;
            }
        };
        let stated = numbers_in(sentence);
        if &stated != expected {
            errors.push(format!(
                "the README says {stated:?} in \"{sentence}\", and the recompute is {expected:?}"
            ));
        }
        covered.push(sentence.clone());
    }
    for sentence in &sentences {
        if states_a_count(sentence) && !covered.contains(sentence) {
            errors.push(format!(
                "the README states a count no test recomputes: \"{sentence}\""
            ));
        }
    }
    errors
}

#[test]
fn the_readme_phase_counts_and_shapes_follow_the_fixtures() {
    let errors = phase_errors(README, &tasks());
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn a_planted_wrong_phase_count_fails_the_pin() {
    // The pin is only a pin if a wrong number in the table fails it.
    let planted = README.replace("| `draft` | 6 |", "| `draft` | 5 |");
    let errors = phase_errors(&planted, &tasks());
    assert!(
        errors.iter().any(|error| error.starts_with("draft says 5")),
        "a planted draft count passed: {errors:?}"
    );
}

#[test]
fn every_count_the_readme_states_is_recomputed() {
    // Every prose count sentence is matched to a recomputed claim, and a
    // count sentence with no claim fails the gate. The phase table and the
    // runtime table are read by their own pins.
    let errors = count_errors(README, &tasks());
    assert!(errors.is_empty(), "{}", errors.join("\n"));

    let claims = count_claims(&tasks());
    let checked = prose_sentences(README)
        .iter()
        .filter(|sentence| states_a_count(sentence))
        .count();
    assert_eq!(
        checked,
        claims.len(),
        "the guard reads {checked} count sentences and the claims name {}",
        claims.len()
    );
}

#[test]
fn a_planted_count_without_a_claim_fails_the_guard() {
    // The guard reads the number, not a noun near it: "reasons" is on no noun
    // list, and the planted seven still has to fail.
    let planted = README.replace("one reason alone", "seven reasons alone");
    assert_ne!(planted, README, "the plant did not land");
    let errors = count_errors(&planted, &tasks());
    assert!(
        errors
            .iter()
            .any(|error| error.contains("seven reasons alone")),
        "a planted count passed: {errors:?}"
    );
}

#[test]
fn the_runtime_counts_follow_the_fixtures() {
    // The run's task in flight is held by the crew, and its limit and its open
    // findings are its own. The rounds used are the run's, stated in the README.
    let tasks = tasks();
    let in_flight = tasks
        .iter()
        .find(|task| task.id == "TSK14")
        .expect("the task in flight");
    assert_eq!(in_flight.status, Status::CodeReview);
    assert_eq!(
        numbers_in(&runtime_row(README, "the budget")),
        vec![2, in_flight.review.max_rounds as usize],
        "the budget row does not follow the fixtures"
    );

    assert_eq!(
        numbers_in(&runtime_row(README, "the findings")),
        vec![in_flight.open_findings()],
        "the findings row does not follow the fixtures"
    );
}

#[test]
fn the_readme_does_not_call_the_text_lorem_ipsum() {
    assert!(
        !README.to_lowercase().contains("lorem ipsum"),
        "the README still says the text is lorem ipsum"
    );
}
