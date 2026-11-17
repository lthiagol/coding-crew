//! The task → markdown projection.

use coding_lanes_core::lifecycle::{Overlay, Overlays};
use coding_lanes_core::task::{
    AcceptanceCriterion, Claim, Comment, Deferral, Effort, Evidence, Finding, FindingStatus, Focus,
    Intent, Meta, Note, Priority, Question, QuestionStatus, Review, Round, RoundKind, Scope,
    Severity, Step, StepStatus, Task, TaskKind, Verdict,
};
use coding_lanes_core::{CriterionStatus, Status, task_to_markdown};

const HEADINGS: [&str; 10] = [
    "## Header",
    "## Intent",
    "## Scope",
    "## Acceptance criteria",
    "## Steps",
    "## Review",
    "## Questions",
    "## Notes",
    "## Dependencies",
    "## Children",
];

fn minimal(id: &str) -> Task {
    Task {
        schema_version: 1,
        id: id.to_string(),
        title: String::new(),
        slug: String::new(),
        parent: None,
        children: Vec::new(),
        kind: TaskKind::Task,
        promoted_from: None,
        created: "2026-11-14T08:00:00Z".to_string(),
        updated: "2026-11-14T08:00:00Z".to_string(),
        status: Status::Draft,
        status_since: "2026-11-14T08:00:00Z".to_string(),
        overlays: Overlays::default(),
        claim: None,
        planned_by: None,
        meta: Meta::default(),
        intent: Intent::default(),
        scope: Scope::default(),
        acceptance_criteria: Vec::new(),
        steps: Vec::new(),
        review: Review::default(),
        questions: Vec::new(),
        notes: Vec::new(),
        depends_on: Vec::new(),
    }
}

fn evidence(command: &str, exit_code: Option<i32>, summary: &str, at: &str) -> Evidence {
    Evidence {
        command: command.to_string(),
        exit_code,
        summary: summary.to_string(),
        at: at.to_string(),
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
    }
}

fn step(id: &str, order: u32, status: StepStatus) -> Step {
    Step {
        id: id.to_string(),
        order,
        action: format!("Do the work of {id}."),
        done_when: format!("{id} is observable."),
        files: vec!["crates/raul/src/lane.rs".to_string()],
        status,
        covers: vec!["ACCEPT-01".to_string()],
        depends_on: Vec::new(),
        deviation: String::new(),
    }
}

/// A stable fixture: every section has content, including one criterion whose
/// stored status disagrees with its latest evidence.
fn fixture() -> Task {
    Task {
        schema_version: 1,
        id: "TSK07".to_string(),
        title: "Add lane filtering to raul".to_string(),
        slug: "add-lane-filtering-to-raul".to_string(),
        parent: None,
        children: vec!["TSK07.1".to_string()],
        kind: TaskKind::Task,
        promoted_from: None,
        created: "2026-11-14T08:00:00Z".to_string(),
        updated: "2026-11-16T17:30:00Z".to_string(),
        status: Status::CodeReview,
        status_since: "2026-11-16T16:00:00Z".to_string(),
        overlays: Overlays {
            blocked: Some(Overlay {
                reason: "The upstream lane table is still being renamed.".to_string(),
                by: "creator-1".to_string(),
                at: "2026-11-16T15:00:00Z".to_string(),
            }),
            deferred: None,
            cancelled: None,
        },
        claim: Some(Claim {
            by: "runner-1".to_string(),
            session: Some("s-runner".to_string()),
            at: "2026-11-16T12:00:00Z".to_string(),
            lease_expires_at: "2026-11-16T18:00:00Z".to_string(),
        }),
        planned_by: Some("s-creator".to_string()),
        meta: Meta {
            priority: Priority::High,
            effort: Effort::Large,
            tags: vec!["tui".to_string(), "lane-filter".to_string()],
        },
        intent: Intent {
            outcome: "raul can show one lane at a time, and the counts match the screen.".to_string(),
            problem: "The Overview shows every lane at once, so a long list hides the\nlane the human is working in.".to_string(),
        },
        scope: Scope {
            in_scope: vec![
                "A lane selector on the Overview.".to_string(),
                "Counts that follow the selector.".to_string(),
            ],
            out_of_scope: vec![
                "A saved default lane.".to_string(),
                "Filtering the Backlog tab.".to_string(),
            ],
        },
        acceptance_criteria: vec![
            AcceptanceCriterion {
                id: "ACCEPT-01".to_string(),
                description: "The selector hides a task that matches no selected lane.".to_string(),
                verification: "cargo test -p coding-lanes-raul lane_filter".to_string(),
                status: CriterionStatus::Pending,
                evidence: vec![evidence(
                    "cargo test -p coding-lanes-raul lane_filter",
                    Some(0),
                    "12 passed; 0 failed",
                    "2026-11-16T16:10:00Z",
                )],
                deferral: None,
                failure_reason: String::new(),
            },
            AcceptanceCriterion {
                id: "ACCEPT-02".to_string(),
                description: "The saved default lane is not in this task.".to_string(),
                verification: "cargo test -p coding-lanes-raul saved_lane".to_string(),
                status: CriterionStatus::Deferred,
                evidence: Vec::new(),
                deferral: Some(Deferral {
                    reason: "The settings store does not exist yet.".to_string(),
                    follow_up: "TSK09".to_string(),
                }),
                failure_reason: String::new(),
            },
            AcceptanceCriterion {
                id: "ACCEPT-03".to_string(),
                description: "The empty lane announces that it is empty.".to_string(),
                verification: "cargo test -p coding-lanes-raul empty_lane".to_string(),
                status: CriterionStatus::Failed,
                evidence: vec![evidence(
                    "cargo test -p coding-lanes-raul empty_lane",
                    Some(1),
                    "1 failed",
                    "2026-11-16T16:20:00Z",
                )],
                deferral: None,
                failure_reason: "The lane count is drawn before the filter runs.".to_string(),
            },
        ],
        steps: vec![
            Step {
                id: "STP-01".to_string(),
                order: 1,
                action: "Read the Overview's lane mapping.".to_string(),
                done_when: "The mapping is named in the notes.".to_string(),
                files: vec!["crates/raul/src/overview.rs".to_string()],
                status: StepStatus::Done,
                covers: vec!["ACCEPT-01".to_string()],
                depends_on: Vec::new(),
                deviation: String::new(),
            },
            Step {
                id: "STP-02".to_string(),
                order: 2,
                action: "Draw the selector above the list.".to_string(),
                done_when: "The selector is on the first row.".to_string(),
                files: vec![
                    "crates/raul/src/overview.rs".to_string(),
                    "crates/raul/src/theme.rs".to_string(),
                ],
                status: StepStatus::InProgress,
                covers: vec!["ACCEPT-01".to_string(), "ACCEPT-03".to_string()],
                depends_on: vec!["STP-01".to_string()],
                deviation: "Used the existing slot table instead of a new one.".to_string(),
            },
        ],
        review: Review {
            max_rounds: 5,
            rounds: vec![
                Round {
                    n: 1,
                    kind: RoundKind::SelfReview,
                    focus: Focus::SpecCompliance,
                    reviewer: "runner-1".to_string(),
                    verdict: Verdict::Ok,
                    at: "2026-11-16T16:30:00Z".to_string(),
                    notes: "Read the diff against the criteria.".to_string(),
                    findings: Vec::new(),
                },
                Round {
                    n: 2,
                    kind: RoundKind::External,
                    focus: Focus::EdgeCases,
                    reviewer: "reviewer-1".to_string(),
                    verdict: Verdict::ChangesNeeded,
                    at: "2026-11-16T17:00:00Z".to_string(),
                    notes: "The empty lane is counted before the filter.".to_string(),
                    findings: vec!["FIND-01".to_string()],
                },
            ],
            verdict: Some(Verdict::ChangesNeeded),
            findings: vec![
                Finding {
                    id: "FIND-01".to_string(),
                    severity: Severity::High,
                    status: FindingStatus::Open,
                    description: "An empty lane counts the rows it does not show.".to_string(),
                    created: "2026-11-16T17:00:00Z".to_string(),
                    resolved: String::new(),
                    resolution: String::new(),
                    comments: vec![Comment {
                        id: "CMT-01".to_string(),
                        author: "reviewer-1".to_string(),
                        body: "The count is drawn from the unfiltered list.".to_string(),
                        at: "2026-11-16T17:01:00Z".to_string(),
                    }],
                },
                Finding {
                    id: "FIND-02".to_string(),
                    severity: Severity::Low,
                    status: FindingStatus::Fixed,
                    description: "The selector's label is one column too wide.".to_string(),
                    created: "2026-11-16T14:00:00Z".to_string(),
                    resolved: "2026-11-16T14:30:00Z".to_string(),
                    resolution: "Cut the label at the column that fits.".to_string(),
                    comments: Vec::new(),
                },
            ],
        },
        questions: vec![
            Question {
                id: "QST-01".to_string(),
                question: "Does the selector remember its lane between runs?".to_string(),
                status: QuestionStatus::Open,
                answer: String::new(),
                raised_by: "creator-1".to_string(),
                raised_at: "2026-11-14T09:00:00Z".to_string(),
                resolved_by: String::new(),
                resolved_at: String::new(),
            },
            Question {
                id: "QST-02".to_string(),
                question: "Is the selector a tab or a row?".to_string(),
                status: QuestionStatus::Resolved,
                answer: "A row above the list; it is a filter, not a screen.".to_string(),
                raised_by: "creator-1".to_string(),
                raised_at: "2026-11-14T09:05:00Z".to_string(),
                resolved_by: "human-1".to_string(),
                resolved_at: "2026-11-14T10:00:00Z".to_string(),
            },
        ],
        notes: vec![Note {
            id: "NOTE-01".to_string(),
            author: "runner-1".to_string(),
            body: "The lane mapping was already a table; reuse it instead of adding one."
                .to_string(),
            at: "2026-11-16T12:30:00Z".to_string(),
        }],
        depends_on: vec!["TSK01".to_string()],
    }
}

/// The context the projection reads: a finished dependency, a task waiting on
/// the fixture, and a child with partial progress.
fn context() -> Vec<Task> {
    let mut dependency = minimal("TSK01");
    dependency.title = "The lane table".to_string();
    dependency.status = Status::Done;

    let mut waiting = minimal("TSK02");
    waiting.title = "Sort the filtered list".to_string();
    waiting.status = Status::Groomed;
    waiting.depends_on = vec!["TSK07".to_string()];

    let mut child = minimal("TSK07.1");
    child.title = "Refine the selector's label".to_string();
    child.parent = Some("TSK07".to_string());
    child.status = Status::Groomed;
    child.acceptance_criteria = vec![
        AcceptanceCriterion {
            id: "ACCEPT-01".to_string(),
            description: "The label fits.".to_string(),
            verification: "cargo test label_fits".to_string(),
            status: CriterionStatus::Passed,
            evidence: vec![evidence(
                "cargo test label_fits",
                Some(0),
                "1 passed",
                "2026-11-15T09:00:00Z",
            )],
            ..AcceptanceCriterion::default()
        },
        AcceptanceCriterion {
            id: "ACCEPT-02".to_string(),
            description: "The label is stable.".to_string(),
            verification: "cargo test label_stable".to_string(),
            status: CriterionStatus::Pending,
            evidence: vec![evidence(
                "cargo test label_stable",
                Some(0),
                "1 passed",
                "2026-11-15T09:10:00Z",
            )],
            ..AcceptanceCriterion::default()
        },
    ];
    child.steps = vec![
        step("STP-01", 1, StepStatus::Done),
        step("STP-02", 2, StepStatus::Pending),
    ];

    let unrelated = minimal("TSK99");

    vec![dependency, waiting, child, unrelated]
}

fn section<'a>(document: &'a str, heading: &str) -> &'a str {
    let start = document
        .find(heading)
        .unwrap_or_else(|| panic!("the document has no {heading}"));
    let after = start + heading.len();
    match document[after..].find("\n## ").map(|offset| after + offset) {
        Some(end) => &document[start..end],
        None => &document[start..],
    }
}

fn headings(document: &str) -> Vec<&str> {
    document
        .lines()
        .filter(|line| line.starts_with("## "))
        .collect()
}

/// The cells of one table row. A pipe escaped by `cell()` does not split it.
fn columns(line: &str) -> usize {
    let mut pipes = 0usize;
    let mut backslashes = 0;
    for ch in line.chars() {
        match ch {
            '\\' => backslashes += 1,
            '|' => {
                if backslashes % 2 == 0 {
                    pipes += 1;
                }
                backslashes = 0;
            }
            _ => backslashes = 0,
        }
    }
    pipes.saturating_sub(1)
}

fn table_rows<'a>(document: &'a str, heading: &str) -> Vec<&'a str> {
    section(document, heading)
        .lines()
        .filter(|line| line.starts_with('|') && !line.starts_with("|-"))
        .collect()
}

/// The document with every inline code span removed.
///
/// A fixed field lives inside a code span, so an attack that is absent here
/// did not escape its span and cannot act on the document.
fn without_code_spans(document: &str) -> String {
    let bytes = document.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'`' {
                i += 1;
            }
            let run = i - start;
            let mut j = i;
            let mut close = None;
            while j < bytes.len() {
                if bytes[j] == b'`' {
                    let opened = j;
                    while j < bytes.len() && bytes[j] == b'`' {
                        j += 1;
                    }
                    if j - opened == run {
                        close = Some((opened, j));
                        break;
                    }
                } else {
                    j += 1;
                }
            }
            match close {
                Some((_, end)) => {
                    for &byte in &bytes[start..end] {
                        if byte == b'\n' {
                            out.push('\n');
                        }
                    }
                    i = end;
                }
                None => {
                    out.push_str(&document[start..i]);
                }
            }
        } else {
            let ch = document[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

#[test]
fn the_projection_is_pure() {
    let task = fixture();
    let tasks = context();
    let task_before = task.clone();
    let tasks_before = tasks.clone();

    let first = task_to_markdown(&task, &tasks);
    let second = task_to_markdown(&task, &tasks);

    assert_eq!(first, second);
    assert_eq!(task, task_before);
    assert_eq!(tasks, tasks_before);
}

#[test]
fn the_sections_are_the_ten_in_order_and_no_others() {
    let document = task_to_markdown(&fixture(), &context());
    let found: Vec<&str> = document
        .lines()
        .filter(|line| line.starts_with("## "))
        .collect();

    assert_eq!(found, HEADINGS);
}

#[test]
fn every_section_is_present_and_marks_its_empty_state() {
    let document = task_to_markdown(&minimal("TSK01"), &[]);
    let mut last = 0;

    for heading in HEADINGS {
        let at = document
            .find(heading)
            .unwrap_or_else(|| panic!("no {heading}"));
        assert!(at >= last, "{heading} is out of order");
        last = at;

        let body = section(&document, heading);
        assert!(
            body.contains("_none_"),
            "{heading} has no empty marker:\n{body}"
        );
    }

    assert_eq!(
        document
            .lines()
            .filter(|line| line.starts_with("## "))
            .count(),
        HEADINGS.len()
    );
}

#[test]
fn dependencies_name_what_waits_on_the_task_from_the_context() {
    let task = minimal("TSK10");
    let mut first = minimal("TSK02");
    first.depends_on = vec!["TSK10".to_string()];
    let mut second = minimal("TSK01");
    second.depends_on = vec!["TSK10".to_string()];
    let mut unrelated = minimal("TSK03");
    unrelated.depends_on = vec!["TSK11".to_string()];

    let document = task_to_markdown(&task, &[first, second, unrelated]);
    let body = section(&document, "## Dependencies");

    assert!(body.contains("`TSK01` — `draft`"), "{body}");
    assert!(body.contains("`TSK02` — `draft`"), "{body}");
    assert!(!body.contains("TSK03"), "{body}");
    assert!(!body.contains("TSK11"), "{body}");
}

#[test]
fn children_show_their_status_and_progress_from_the_context() {
    let task = minimal("TSK10");
    let mut child = minimal("TSK10.1");
    child.parent = Some("TSK10".to_string());
    child.status = Status::Groomed;
    child.acceptance_criteria = vec![
        AcceptanceCriterion {
            id: "ACCEPT-01".to_string(),
            status: CriterionStatus::Passed,
            ..AcceptanceCriterion::default()
        },
        AcceptanceCriterion {
            id: "ACCEPT-02".to_string(),
            status: CriterionStatus::Pending,
            evidence: vec![evidence(
                "cargo test stable",
                Some(0),
                "1 passed",
                "2026-11-15T09:10:00Z",
            )],
            ..AcceptanceCriterion::default()
        },
    ];
    child.steps = vec![
        step("STP-01", 1, StepStatus::Done),
        step("STP-02", 2, StepStatus::Pending),
    ];

    let document = task_to_markdown(&task, &[child]);
    let body = section(&document, "## Children");

    assert!(body.contains("`TSK10.1`"), "{body}");
    assert!(body.contains("`groomed`"), "{body}");
    assert!(body.contains("criteria `2/2` passed"), "{body}");
    assert!(body.contains("steps `1/2` done"), "{body}");
}

#[test]
fn a_criterion_shows_its_effective_status_not_its_stored_one() {
    let mut task = minimal("TSK01");
    task.acceptance_criteria = vec![
        AcceptanceCriterion {
            id: "ACCEPT-01".to_string(),
            description: "free text".to_string(),
            verification: "cargo test".to_string(),
            status: CriterionStatus::Pending,
            evidence: vec![evidence(
                "cargo test",
                Some(0),
                "1 passed",
                "2026-11-14T09:00:00Z",
            )],
            ..AcceptanceCriterion::default()
        },
        AcceptanceCriterion {
            id: "ACCEPT-02".to_string(),
            status: CriterionStatus::Pending,
            ..AcceptanceCriterion::default()
        },
    ];

    let document = task_to_markdown(&task, &[]);
    let body = section(&document, "## Acceptance criteria");

    assert!(
        body.contains("| `ACCEPT-01` | free text | `cargo test` | `passed` |"),
        "{body}"
    );
    assert!(
        body.contains("| `ACCEPT-02` | _none_ | _none_ | `pending` |"),
        "{body}"
    );
}

#[test]
fn fixed_values_cannot_create_markdown_structure() {
    let mut task = minimal("TSK01");
    task.title = "# Heading [link](https://example.com) | cell\n- item".to_string();
    task.meta.tags = vec!["a|b".to_string(), "`tick`".to_string()];
    task.acceptance_criteria = vec![AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        description: "free text".to_string(),
        verification: "echo `pwn` && # not a heading".to_string(),
        ..AcceptanceCriterion::default()
    }];
    task.steps = vec![Step {
        id: "STP-01".to_string(),
        files: vec!["src/`x`.rs".to_string()],
        ..Step::default()
    }];

    let document = task_to_markdown(&task, &[]);
    let body = section(&document, "## Header");

    assert!(body.contains("\\# Heading \\[link\\]"), "{body}");
    assert!(!document.lines().any(|line| line.starts_with("# Heading")));
    assert!(
        !document.contains("[link](https://example.com)"),
        "{document}"
    );
    assert!(body.contains("`a\\|b`"), "{body}");
    assert!(body.contains("`` `tick` ``"), "{body}");

    let criteria = section(&document, "## Acceptance criteria");
    assert!(
        criteria.contains("``echo `pwn` && # not a heading``"),
        "{criteria}"
    );

    let steps = section(&document, "## Steps");
    assert!(steps.contains("``src/`x`.rs``"), "{steps}");
}

#[test]
fn free_text_keeps_its_markdown_meaning() {
    let mut task = minimal("TSK01");
    task.intent.outcome = "**Done** when\n\n- one\n- two".to_string();
    task.scope.in_scope = vec!["A *list*:\n\n1. first\n2. second".to_string()];
    task.acceptance_criteria = vec![AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        description: "not **flat**\nstill markdown".to_string(),
        ..AcceptanceCriterion::default()
    }];
    task.steps = vec![Step {
        id: "STP-01".to_string(),
        action: "Run:\n\n```sh\ncargo test\n```".to_string(),
        ..Step::default()
    }];
    task.notes = vec![Note {
        id: "NOTE-01".to_string(),
        body: "# What we learned\n\nThe **lane** table was enough.".to_string(),
        ..Note::default()
    }];

    let document = task_to_markdown(&task, &[]);

    assert!(section(&document, "## Intent").contains("**Done** when\n\n- one\n- two"));
    let scope = section(&document, "## Scope");
    assert!(scope.contains("A *list*:"), "{scope}");
    assert!(scope.contains("1. first"), "{scope}");
    assert!(scope.contains("2. second"), "{scope}");
    assert!(
        section(&document, "## Acceptance criteria").contains("not **flat**<br>still markdown")
    );
    assert!(section(&document, "## Steps").contains("```sh"));
    assert!(section(&document, "## Steps").contains("  cargo test"));
    assert!(section(&document, "## Notes").contains("# What we learned"));
    assert!(section(&document, "## Notes").contains("The **lane** table was enough."));
}

#[test]
fn verification_commands_are_code_never_prose() {
    let mut task = minimal("TSK01");
    task.acceptance_criteria = vec![AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        verification: "cargo test --workspace".to_string(),
        ..AcceptanceCriterion::default()
    }];

    let document = task_to_markdown(&task, &[]);
    let body = section(&document, "## Acceptance criteria");

    assert!(body.contains("`cargo test --workspace`"), "{body}");
    assert!(!body.contains("| cargo test --workspace |"), "{body}");
}

#[test]
fn the_header_names_the_kind_and_the_review_shows_the_budget() {
    let mut task = minimal("TSK01");
    task.kind = TaskKind::Bug;
    task.review = Review {
        max_rounds: 5,
        rounds: vec![
            Round {
                n: 1,
                kind: RoundKind::External,
                ..Round::default()
            },
            Round {
                n: 2,
                kind: RoundKind::SelfReview,
                ..Round::default()
            },
            Round {
                n: 3,
                kind: RoundKind::External,
                ..Round::default()
            },
        ],
        ..Review::default()
    };

    let document = task_to_markdown(&task, &[]);

    let header = section(&document, "## Header");
    assert!(header.contains("| kind | `bug` |"), "{header}");
    let review = section(&document, "## Review");
    assert!(
        review.contains("- **budget:** `2/5` external rounds"),
        "{review}"
    );
}

#[test]
fn a_listed_child_absent_from_the_context_is_unknown() {
    let mut task = minimal("TSK10");
    task.children = vec!["TSK10.2".to_string(), "TSK10.2".to_string()];

    let document = task_to_markdown(&task, &[]);
    let body = section(&document, "## Children");

    assert!(body.contains("- `TSK10.2` — `unknown`"), "{body}");
    assert_eq!(body.matches("`TSK10.2`").count(), 1, "{body}");
}

#[test]
fn derived_lists_are_sorted_by_id_and_ignore_the_context_order() {
    let mut task = minimal("TSK10");
    task.children = vec!["TSK10.9".to_string()];

    let mut waiting_b = minimal("TSK30");
    waiting_b.depends_on = vec!["TSK10".to_string()];
    let mut waiting_a = minimal("TSK20");
    waiting_a.depends_on = vec!["TSK10".to_string()];
    let mut discovered_a = minimal("TSK10.2");
    discovered_a.parent = Some("TSK10".to_string());
    let mut discovered_b = minimal("TSK10.9");
    discovered_b.parent = Some("TSK10".to_string());

    let forward = vec![waiting_b, discovered_b, waiting_a, discovered_a];
    let mut backward = forward.clone();
    backward.reverse();

    let first = task_to_markdown(&task, &forward);
    let second = task_to_markdown(&task, &backward);
    assert_eq!(first, second);

    let dependencies = section(&first, "## Dependencies");
    assert!(
        dependencies.find("`TSK20`") < dependencies.find("`TSK30`"),
        "{dependencies}"
    );
    let children = section(&first, "## Children");
    assert!(
        children.find("`TSK10.2`") < children.find("`TSK10.9`"),
        "{children}"
    );
}

#[test]
fn duplicate_ids_in_the_context_resolve_the_same_record_in_any_order() {
    let mut task = minimal("TSK10");
    task.children = vec!["TSK10.1".to_string()];
    task.depends_on = vec!["TSK20".to_string()];

    let mut child_a = minimal("TSK10.1");
    child_a.parent = Some("TSK10".to_string());
    child_a.title = "Child A".to_string();
    child_a.status = Status::Done;
    child_a.acceptance_criteria = vec![AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        status: CriterionStatus::Passed,
        ..AcceptanceCriterion::default()
    }];
    let mut child_b = child_a.clone();
    child_b.title = "Child B".to_string();
    child_b.status = Status::Draft;
    child_b.acceptance_criteria.clear();

    let mut wait_a = minimal("TSK20");
    wait_a.title = "Wait A".to_string();
    wait_a.status = Status::Done;
    wait_a.depends_on = vec!["TSK10".to_string()];
    let mut wait_b = wait_a.clone();
    wait_b.title = "Wait B".to_string();
    wait_b.status = Status::Draft;

    let mut context = vec![child_a, child_b, wait_a, wait_b];
    let first = task_to_markdown(&task, &context);
    context.reverse();
    let second = task_to_markdown(&task, &context);

    assert_eq!(first, second);

    // The child's status, title and progress come from the same record.
    let children = section(&first, "## Children");
    let line = children
        .lines()
        .find(|line| line.starts_with("- "))
        .unwrap_or_else(|| panic!("no child line:\n{children}"));
    if line.contains("Child A") {
        assert!(line.contains("`done`"), "{line}");
        assert!(line.contains("criteria `1/1` passed"), "{line}");
    } else {
        assert!(line.contains("Child B"), "{line}");
        assert!(line.contains("`draft`"), "{line}");
        assert!(line.contains("criteria `0/0` passed"), "{line}");
    }
}

#[test]
fn a_non_canonical_duplicate_cannot_add_a_reverse_dependency() {
    let task = minimal("TSK10");

    let mut canonical_record = minimal("TSK20");
    canonical_record.title = "Canonical TSK20".to_string();
    let mut duplicate_record = canonical_record.clone();
    duplicate_record.title = "Duplicate TSK20".to_string();
    duplicate_record.depends_on = vec!["TSK10".to_string()];

    let canonical_key = serde_json::to_string(&canonical_record).unwrap();
    let duplicate_key = serde_json::to_string(&duplicate_record).unwrap();
    assert!(
        canonical_key < duplicate_key,
        "the record without the dependency must be canonical:\n{canonical_key}\n{duplicate_key}"
    );

    for context in [
        vec![canonical_record.clone(), duplicate_record.clone()],
        vec![duplicate_record, canonical_record],
    ] {
        let document = task_to_markdown(&task, &context);
        let dependencies = section(&document, "## Dependencies");
        assert!(
            !dependencies.contains("`TSK20`"),
            "a non-canonical duplicate added a reverse dependency:\n{dependencies}"
        );
    }
}

#[test]
fn duplicate_ids_render_once() {
    let mut task = minimal("TSK10");
    task.depends_on = vec!["TSK01".to_string(), "TSK01".to_string()];
    task.children = vec!["TSK10.1".to_string(), "TSK10.1".to_string()];

    let dependency = minimal("TSK01");
    let mut waiting = minimal("TSK02");
    waiting.depends_on = vec!["TSK10".to_string()];
    let mut child = minimal("TSK10.1");
    child.parent = Some("TSK10".to_string());

    let document = task_to_markdown(
        &task,
        &[dependency, waiting.clone(), child.clone(), waiting, child],
    );

    let dependencies = section(&document, "## Dependencies");
    assert_eq!(dependencies.matches("`TSK01`").count(), 1, "{dependencies}");
    assert_eq!(dependencies.matches("`TSK02`").count(), 1, "{dependencies}");
    let children = section(&document, "## Children");
    assert_eq!(children.matches("`TSK10.1`").count(), 1, "{children}");
}

#[test]
fn a_newline_in_a_fixed_field_cannot_break_the_line_structure() {
    let mut task = minimal("TSK01");
    task.id = "TSK01\n## injected-heading\n- injected-item".to_string();
    task.notes = vec![Note {
        id: "NOTE-01".to_string(),
        author: "runner-1".to_string(),
        body: "body".to_string(),
        at: "2026-11-14T08:00:00Z\n## injected-time-heading\n- injected-time-item".to_string(),
    }];
    task.acceptance_criteria = vec![AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        verification: "cargo test".to_string(),
        evidence: vec![evidence(
            "cargo test",
            Some(0),
            "1 passed",
            "2026-11-14T08:00:00Z\n- injected-evidence-item",
        )],
        ..AcceptanceCriterion::default()
    }];

    let document = task_to_markdown(&task, &[]);

    assert_eq!(headings(&document), HEADINGS);
    let subheadings: Vec<&str> = document
        .lines()
        .filter(|line| line.starts_with("### "))
        .collect();
    assert_eq!(subheadings, ["### `NOTE-01`"]);
    assert!(
        document.contains(
            "- **at:** `2026-11-14T08:00:00Z ## injected-time-heading - injected-time-item`"
        ),
        "{document}"
    );
    assert!(!document.lines().any(|line| line.starts_with("- injected")));
    for line in table_rows(&document, "## Acceptance criteria") {
        assert_eq!(columns(line), 7, "{line}");
    }
}

#[test]
fn a_backslash_run_resets_at_a_pipe() {
    let mut task = minimal("TSK01");
    task.meta.tags = vec!["a\\||b".to_string()];
    let document = task_to_markdown(&task, &[]);

    assert!(
        document.contains("| tags | `a\\|\\|b` |"),
        "the backslash run did not reset at the first pipe:\n{document}"
    );
    for line in table_rows(&document, "## Header") {
        assert_eq!(columns(line), 2, "{line}");
    }
}

#[test]
fn backslashes_before_pipes_never_split_a_table_row() {
    let mut values: Vec<String> = (0..=5)
        .map(|backslashes| format!("left{}|right", "\\".repeat(backslashes)))
        .collect();
    // Adjacent pipes: after the first pipe the backslash run is zero, so the
    // second pipe must be escaped on its own. Without the reset this splits
    // the row.
    values.push("a\\||b".to_string());

    for value in values {
        let mut task = minimal("TSK01");
        task.title = format!("title {value}");
        task.meta.tags = vec![value.clone()];
        task.acceptance_criteria = vec![
            AcceptanceCriterion {
                id: value.clone(),
                description: format!("criterion {value}"),
                verification: format!("echo {value}"),
                status: CriterionStatus::Passed,
                evidence: vec![Evidence {
                    command: format!("echo {value}"),
                    exit_code: Some(0),
                    summary: "1 passed".to_string(),
                    at: "2026-11-14T09:00:00Z".to_string(),
                    by: "runner-1".to_string(),
                    session: None,
                }],
                ..AcceptanceCriterion::default()
            },
            AcceptanceCriterion {
                id: "ACCEPT-02".to_string(),
                ..AcceptanceCriterion::default()
            },
        ];
        task.review = Review {
            max_rounds: 5,
            rounds: vec![Round {
                n: 1,
                kind: RoundKind::External,
                focus: Focus::Security,
                reviewer: value.clone(),
                verdict: Verdict::Ok,
                at: "2026-11-16T12:00:00Z".to_string(),
                notes: format!("notes {value}"),
                findings: vec![value.clone()],
            }],
            ..Review::default()
        };

        let document = task_to_markdown(&task, &[]);
        assert_eq!(headings(&document), HEADINGS, "{value}");

        for (heading, expected_columns, expected_rows) in [
            ("## Header", 2, 10),
            ("## Acceptance criteria", 7, 3),
            ("## Review", 8, 2),
        ] {
            let rows = table_rows(&document, heading);
            assert_eq!(
                rows.len(),
                expected_rows,
                "{heading} has {} rows with {value}:\n{document}",
                rows.len()
            );
            for line in rows {
                assert_eq!(
                    columns(line),
                    expected_columns,
                    "{heading} broke with {value}: {line}"
                );
            }
        }
    }
}

#[test]
fn every_fixed_field_keeps_its_code_span_under_attack() {
    const ATTACK: &str = "FIXED-ATTACK\n# injected-heading\n- injected-item\n1. injected-order\n> injected-quote\n```injected-fence\n[link](https://injected.example)\n| injected-cell |";

    let mut task = minimal(ATTACK);
    task.title = "# Title attack [link](https://title.example) | cell\n- title item".to_string();
    task.meta.tags = vec![ATTACK.to_string()];
    task.overlays.blocked = Some(Overlay {
        reason: "**bold** overlay".to_string(),
        by: ATTACK.to_string(),
        at: ATTACK.to_string(),
    });
    task.claim = Some(Claim {
        by: ATTACK.to_string(),
        session: Some(ATTACK.to_string()),
        at: ATTACK.to_string(),
        lease_expires_at: ATTACK.to_string(),
    });
    task.acceptance_criteria = vec![AcceptanceCriterion {
        id: ATTACK.to_string(),
        description: "**bold** criterion".to_string(),
        verification: "cargo test `x` | # not a heading".to_string(),
        status: CriterionStatus::Passed,
        evidence: vec![Evidence {
            command: ATTACK.to_string(),
            exit_code: Some(0),
            summary: "**bold** evidence".to_string(),
            at: ATTACK.to_string(),
            by: ATTACK.to_string(),
            session: Some(ATTACK.to_string()),
        }],
        deferral: Some(Deferral {
            reason: "**bold** deferral".to_string(),
            follow_up: ATTACK.to_string(),
        }),
        failure_reason: "**bold** failure".to_string(),
    }];
    task.steps = vec![Step {
        id: ATTACK.to_string(),
        order: 1,
        action: "**bold** action".to_string(),
        done_when: "**bold** done when".to_string(),
        files: vec![ATTACK.to_string()],
        status: StepStatus::Done,
        covers: vec![ATTACK.to_string()],
        depends_on: vec![ATTACK.to_string()],
        deviation: "**bold** deviation".to_string(),
    }];
    task.review = Review {
        max_rounds: 5,
        rounds: vec![Round {
            n: 1,
            kind: RoundKind::External,
            focus: Focus::Security,
            reviewer: ATTACK.to_string(),
            verdict: Verdict::ChangesNeeded,
            at: ATTACK.to_string(),
            notes: "**bold** notes".to_string(),
            findings: vec![ATTACK.to_string()],
        }],
        verdict: Some(Verdict::ChangesNeeded),
        findings: vec![Finding {
            id: ATTACK.to_string(),
            severity: Severity::High,
            status: FindingStatus::Open,
            description: "**bold** finding".to_string(),
            created: ATTACK.to_string(),
            resolved: ATTACK.to_string(),
            resolution: "**bold** resolution".to_string(),
            comments: vec![Comment {
                id: ATTACK.to_string(),
                author: ATTACK.to_string(),
                body: "**bold** comment".to_string(),
                at: ATTACK.to_string(),
            }],
        }],
    };
    task.questions = vec![Question {
        id: ATTACK.to_string(),
        question: "**bold** question".to_string(),
        status: QuestionStatus::Resolved,
        answer: "**bold** answer".to_string(),
        raised_by: ATTACK.to_string(),
        raised_at: ATTACK.to_string(),
        resolved_by: ATTACK.to_string(),
        resolved_at: ATTACK.to_string(),
    }];
    task.notes = vec![Note {
        id: ATTACK.to_string(),
        author: ATTACK.to_string(),
        body: "**bold** note".to_string(),
        at: ATTACK.to_string(),
    }];
    task.depends_on = vec![ATTACK.to_string(), ATTACK.to_string()];
    task.children = vec![ATTACK.to_string(), ATTACK.to_string()];
    task.intent.outcome = "**bold** outcome".to_string();
    task.scope.in_scope = vec!["**bold** scope".to_string()];

    let document = task_to_markdown(&task, &[]);

    assert_eq!(headings(&document), HEADINGS);
    assert!(document.contains("FIXED-ATTACK"), "{document}");
    assert!(!document.lines().any(|line| line.starts_with("```")));

    // The title is a label: escaped, not interpreted.
    let header = section(&document, "## Header");
    assert!(header.contains("\\# Title attack \\[link\\]"), "{header}");
    assert!(!document.contains("](https://title.example)"), "{document}");

    // Everything fixed is inside a code span and cannot act on the document.
    let outside = without_code_spans(&document);
    for token in [
        "FIXED-ATTACK",
        "injected-heading",
        "injected-item",
        "injected-order",
        "injected-quote",
        "injected-fence",
        "injected.example",
        "injected-cell",
    ] {
        assert!(
            !outside.contains(token),
            "{token} escaped its span:\n{outside}"
        );
    }
    for line in outside.lines() {
        assert!(!line.starts_with("# "), "{line}");
        assert!(!line.starts_with("```"), "{line}");
    }

    // Prose keeps its markdown.
    for prose in [
        "**bold** outcome",
        "**bold** scope",
        "**bold** criterion",
        "**bold** action",
        "**bold** notes",
        "**bold** finding",
        "**bold** resolution",
        "**bold** comment",
        "**bold** question",
        "**bold** answer",
        "**bold** note",
    ] {
        assert!(document.contains(prose), "{prose} was escaped:\n{document}");
    }

    // The fixed-value attacks cannot open a cell or a row.
    for (heading, expected) in [
        ("## Header", 2),
        ("## Acceptance criteria", 7),
        ("## Review", 8),
    ] {
        let rows = table_rows(&document, heading);
        assert!(!rows.is_empty(), "{heading} has no table");
        for line in rows {
            assert_eq!(columns(line), expected, "{heading}: {line}");
        }
    }
}

#[test]
fn every_task_status_renders_as_one_code_span() {
    for status in [
        Status::Draft,
        Status::Groomed,
        Status::Approved,
        Status::Executing,
        Status::CodeReview,
        Status::Remediating,
        Status::NeedsHuman,
        Status::Done,
    ] {
        let mut task = minimal("TSK01");
        task.status = status;

        let document = task_to_markdown(&task, &[]);
        let header = section(&document, "## Header");
        let line = header
            .lines()
            .find(|line| line.starts_with("| status |"))
            .unwrap_or_else(|| panic!("no status row:\n{header}"));

        assert_eq!(columns(line), 2, "{line}");
        assert!(line.contains(&format!("`{}`", status.as_str())), "{line}");
    }
}

#[test]
fn a_multiline_verification_is_a_fenced_block_beside_the_table() {
    let mut task = minimal("TSK01");
    task.acceptance_criteria = vec![
        AcceptanceCriterion {
            id: "ACCEPT-01".to_string(),
            verification: "cargo test --workspace".to_string(),
            ..AcceptanceCriterion::default()
        },
        AcceptanceCriterion {
            id: "ACCEPT-02".to_string(),
            verification: "cargo test\ncargo clippy --all-targets".to_string(),
            ..AcceptanceCriterion::default()
        },
        AcceptanceCriterion {
            id: "ACCEPT-03".to_string(),
            verification: "echo `x`\n```\nnot a fence".to_string(),
            ..AcceptanceCriterion::default()
        },
    ];

    let document = task_to_markdown(&task, &[]);
    let body = section(&document, "## Acceptance criteria");

    for line in table_rows(&document, "## Acceptance criteria") {
        assert_eq!(columns(line), 7, "{line}");
    }
    assert!(
        body.contains("| `ACCEPT-01` | _none_ | `cargo test --workspace` |"),
        "{body}"
    );
    assert!(
        body.contains("| `ACCEPT-02` | _none_ | _see below_ |"),
        "{body}"
    );
    assert!(body.contains("**Verification for `ACCEPT-02`**"), "{body}");
    assert!(
        body.contains("```\ncargo test\ncargo clippy --all-targets\n```"),
        "{body}"
    );
    assert!(body.contains("**Verification for `ACCEPT-03`**"), "{body}");
    assert!(
        body.contains("````\necho `x`\n```\nnot a fence\n````"),
        "{body}"
    );
    assert!(!body.contains("| `ACCEPT-03` | _none_ | `echo"), "{body}");
}

#[test]
fn the_markdown_matches_the_snapshot_byte_for_byte() {
    let document = task_to_markdown(&fixture(), &context());
    assert_eq!(
        document,
        include_str!("snapshots/task.md"),
        "the projection differs from tests/snapshots/task.md"
    );
}
