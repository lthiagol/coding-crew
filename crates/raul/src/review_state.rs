//! The review state a row carries: the findings open with their severities,
//! then the external rounds spent of the task's own budget.
//!
//! The triage fact comes first and the context gives way first: a narrow row
//! keeps the severities it can name before it keeps the rounds. The wording
//! and the severity rule live here, so the Needs you and Path tabs cannot
//! grow a copy of their own.
//!
//! `Task::external_rounds()` and `Task::review.max_rounds` are the model's
//! measures; the self pre-screen is round 0 and is not part of the external
//! budget, so it is not counted here. `Task::open_findings()` counts the
//! findings that are open: a finding that is fixed or dismissed is not one.
//!
//! A task with no external round and no open finding has no state: its row
//! says nothing about the review.

use coding_crew_core::{FindingStatus, Severity, Task};
use coding_crew_theme::Theme;
use coding_crew_theme::text::text_width;
use ratatui::text::Span;

/// The severities, the most urgent first: the order a row names them in.
const SEVERITIES: [Severity; 3] = [Severity::High, Severity::Medium, Severity::Low];

/// How a piece of the state is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grade {
    /// A word: the slot of secondary text.
    Label,
    /// A count: `text.strong`, as every count in raul is.
    Count,
}

/// A part of the state: the spans that are shown or given up together.
///
/// A count is never parted from the word it counts, so a narrow row gives up
/// a whole fact or a whole severity rather than half a count. The parts the
/// state is made of are the units a cut works on.
#[derive(Debug, Clone)]
pub struct Part {
    spans: Vec<(String, Grade)>,
}

impl Part {
    /// A part that is a word alone.
    fn label(text: impl Into<String>) -> Self {
        Part {
            spans: vec![(text.into(), Grade::Label)],
        }
    }

    /// A part that starts with a count.
    fn count(text: impl Into<String>) -> Self {
        Part {
            spans: vec![(text.into(), Grade::Count)],
        }
    }

    /// The part with another span after its own.
    fn then(mut self, text: impl Into<String>, grade: Grade) -> Self {
        self.spans.push((text.into(), grade));
        self
    }

    /// The columns the part takes.
    pub fn width(&self) -> usize {
        self.spans
            .iter()
            .map(|(text, _)| text_width(text))
            .sum::<usize>()
    }

    /// The part's text.
    #[cfg(test)]
    pub fn text(&self) -> String {
        self.spans.iter().map(|(text, _)| text.as_str()).collect()
    }
}

/// The state of a task's review: the findings open with their severities,
/// then the round spent of the budget.
///
/// The triage fact comes first: *is there something high here?* is what the
/// human decides on, and the rounds — *how far through the review is it?* —
/// are the context that gives way when the row is narrow. A task with
/// nothing to say has no state.
pub fn state(task: &Task) -> Option<Vec<Part>> {
    let mut parts = Vec::new();
    let open = task.open_findings();
    if open > 0 {
        parts.push(Part::count(open.to_string()).then(
            format!(" open {}", if open == 1 { "finding" } else { "findings" }),
            Grade::Label,
        ));
        parts.extend(severity_parts(task));
    }
    let rounds = task.external_rounds();
    if rounds > 0 {
        let separator = if parts.is_empty() { "" } else { " · " };
        parts.push(
            Part::label(separator)
                .then(rounds.to_string(), Grade::Count)
                .then(format!("/{} rounds", task.review.max_rounds), Grade::Label),
        );
    }
    (!parts.is_empty()).then_some(parts)
}

/// The severities among the open findings, each one present with its count.
///
/// The severities are named, not implied, and the rule for which ones count
/// is the one place a fixed or dismissed finding is left out. The list is
/// split per severity, most urgent first, so a cut between parts keeps the
/// most urgent severity rather than dropping the list whole.
fn severity_parts(task: &Task) -> Vec<Part> {
    let present: Vec<(Severity, usize)> = SEVERITIES
        .iter()
        .filter_map(|severity| {
            let count = task
                .review
                .findings
                .iter()
                .filter(|finding| finding.status == FindingStatus::Open)
                .filter(|finding| finding.severity == *severity)
                .count();
            (count > 0).then_some((*severity, count))
        })
        .collect();
    let last = present.len();
    present
        .into_iter()
        .enumerate()
        .map(|(index, (severity, count))| {
            let lead = if index == 0 { " (" } else { ", " };
            let mut part = Part::label(lead)
                .then(count.to_string(), Grade::Count)
                .then(format!(" {}", severity_word(severity)), Grade::Label);
            if index + 1 == last {
                part = part.then(")", Grade::Label);
            }
            part
        })
        .collect()
}

/// The word for a severity.
fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::High => "high",
        Severity::Medium => "medium",
        Severity::Low => "low",
    }
}

/// The state, cut to `room` columns by whole parts.
///
/// The longest whole prefix wins. A prefix that leaves a part out is marked
/// with `…`, so a cut is never read as the whole state; when not even one
/// part and the marker fit, the row has no state. A count is never cut.
pub fn fit(parts: &[Part], room: usize) -> Option<Vec<Part>> {
    if room == 0 || parts.is_empty() {
        return None;
    }
    let full: usize = parts.iter().map(Part::width).sum();
    if full <= room {
        return Some(parts.to_vec());
    }
    let marker = Part::label("…");
    for keep in (1..parts.len()).rev() {
        let width: usize = parts[..keep].iter().map(Part::width).sum::<usize>() + marker.width();
        if width <= room {
            let mut out = parts[..keep].to_vec();
            out.push(marker);
            return Some(out);
        }
    }
    None
}

/// The state's spans, painted: words dim, counts strong.
pub fn paint(parts: &[Part], theme: &Theme) -> Vec<Span<'static>> {
    parts
        .iter()
        .flat_map(|part| {
            part.spans.iter().map(|(text, grade)| {
                let style = match grade {
                    Grade::Label => theme.text.dim,
                    Grade::Count => theme.text.strong,
                };
                Span::styled(text.clone(), style)
            })
        })
        .collect()
}

/// The text of the state.
#[cfg(test)]
pub fn text(parts: &[Part]) -> String {
    parts.iter().map(Part::text).collect()
}

#[cfg(test)]
mod tests {
    use coding_crew_core::{Finding, Round, RoundKind, Task};

    use super::*;

    /// A task with the review facts given: external rounds, a self round, and
    /// findings of a severity and a status.
    fn task_with(rounds: usize, findings: &[(Severity, FindingStatus)]) -> Task {
        let mut task = Task::new("TSK01", "a task");
        task.review.max_rounds = 5;
        task.review.rounds.push(Round {
            n: 0,
            kind: RoundKind::SelfReview,
            ..Round::default()
        });
        for n in 1..=rounds {
            task.review.rounds.push(Round {
                n: n as u32,
                kind: RoundKind::External,
                ..Round::default()
            });
        }
        for (index, (severity, status)) in findings.iter().enumerate() {
            task.review.findings.push(Finding {
                id: format!("FIND-{index:02}"),
                severity: *severity,
                status: *status,
                ..Finding::default()
            });
        }
        task
    }

    #[test]
    fn the_state_says_the_findings_and_the_rounds_apart() {
        let task = task_with(
            3,
            &[
                (Severity::High, FindingStatus::Open),
                (Severity::Medium, FindingStatus::Open),
                (Severity::High, FindingStatus::Fixed),
                (Severity::Low, FindingStatus::Dismissed),
            ],
        );
        let parts = state(&task).expect("a state");
        assert_eq!(
            text(&parts),
            "2 open findings (1 high, 1 medium) · 3/5 rounds"
        );
        // The two facts are separable: each is its own part, and so is every
        // severity in the list.
        assert_eq!(parts.len(), 4, "{parts:?}");
        assert_eq!(parts[0].text(), "2 open findings");
        assert_eq!(parts[1].text(), " (1 high");
        assert_eq!(parts[2].text(), ", 1 medium)");
        assert_eq!(parts[3].text(), " · 3/5 rounds");
    }

    #[test]
    fn a_fixed_or_dismissed_finding_is_not_open() {
        let task = task_with(
            0,
            &[
                (Severity::High, FindingStatus::Fixed),
                (Severity::Medium, FindingStatus::Dismissed),
            ],
        );
        assert!(state(&task).is_none(), "the resolved findings are counted");
    }

    #[test]
    fn the_self_pre_screen_is_not_part_of_the_budget() {
        // Only the self round is in the record: there is no external round,
        // and the findings are said without a round count.
        let task = task_with(0, &[(Severity::Low, FindingStatus::Open)]);
        let parts = state(&task).expect("a state");
        assert_eq!(text(&parts), "1 open finding (1 low)");
        assert!(
            !text(&parts).contains("0/5"),
            "the self round is counted in the budget"
        );
    }

    #[test]
    fn the_severities_are_named_the_most_urgent_first() {
        let task = task_with(
            1,
            &[
                (Severity::Low, FindingStatus::Open),
                (Severity::High, FindingStatus::Open),
                (Severity::Medium, FindingStatus::Open),
                (Severity::High, FindingStatus::Open),
            ],
        );
        let parts = state(&task).expect("a state");
        assert_eq!(
            text(&parts),
            "4 open findings (2 high, 1 medium, 1 low) · 1/5 rounds"
        );
    }

    #[test]
    fn a_budget_with_no_open_finding_says_only_the_rounds() {
        let task = task_with(5, &[]);
        let parts = state(&task).expect("a state");
        assert_eq!(text(&parts), "5/5 rounds");
    }

    #[test]
    fn the_state_does_not_name_the_phase() {
        // The Path group header names the phase; the row says how far the
        // review got and what it found, and never the phase again.
        let task = task_with(5, &[(Severity::High, FindingStatus::Open)]);
        let said = text(&state(&task).expect("a state"));
        for phase in ["escalated", "approve", "needs-human", "code-review"] {
            assert!(!said.contains(phase), "{said} names the phase {phase}");
        }
    }

    #[test]
    fn a_task_with_nothing_to_report_has_no_state() {
        let mut task = task_with(0, &[]);
        assert!(state(&task).is_none());
        // A task that has not been reviewed at all says nothing either.
        task.review.rounds.clear();
        assert!(state(&task).is_none());
    }

    #[test]
    fn a_cut_state_keeps_every_count_whole() {
        let task = task_with(
            5,
            &[
                (Severity::High, FindingStatus::Open),
                (Severity::High, FindingStatus::Open),
                (Severity::Medium, FindingStatus::Open),
                (Severity::Medium, FindingStatus::Open),
                (Severity::Low, FindingStatus::Open),
            ],
        );
        let parts = state(&task).expect("a state");
        let whole = text(&parts);
        let full = parts.iter().map(Part::width).sum::<usize>();
        for room in 0..=full {
            let Some(fitted) = fit(&parts, room) else {
                continue;
            };
            let shown = text(&fitted);
            assert!(fitted.iter().map(Part::width).sum::<usize>() <= room);
            // What is shown is a prefix of the whole state, cut between
            // whole parts, and marked when it is not the whole.
            if shown == whole {
                continue;
            }
            let head = shown.strip_suffix('…').expect("a cut is marked");
            assert!(whole.starts_with(head), "{room}: {shown:?} is not a prefix");
            // The cut is between parts: no part is shown in half. Every part
            // of the whole that is not kept is absent from the shown text.
            for part in &parts[..fitted.len() - 1] {
                assert!(
                    shown.contains(&part.text()),
                    "{room}: a part is cut: {shown:?}"
                );
            }
            // No count is partial: a digit or a slash never ends the text
            // before the marker.
            let last = head.chars().next_back().expect("a head");
            assert!(
                !last.is_ascii_digit() && last != '/',
                "{room}: a count is cut: {shown:?}"
            );
        }
    }

    #[test]
    fn the_state_is_not_shown_at_all_when_no_part_fits() {
        let task = task_with(5, &[(Severity::High, FindingStatus::Open)]);
        let parts = state(&task).expect("a state");
        assert!(fit(&parts, 0).is_none());
        assert!(fit(&parts, 2).is_none(), "the marker alone is not a state");
        // The narrowest whole piece is the findings count.
        assert!(fit(&parts, 14).is_none());
        let shown = fit(&parts, 15).expect("the findings and the marker");
        assert_eq!(text(&shown), "1 open finding…");
    }

    #[test]
    fn the_counts_are_strong_and_the_words_dim() {
        let theme = Theme::preset(coding_crew_theme::Preset::Dark);
        let task = task_with(2, &[(Severity::High, FindingStatus::Open)]);
        let parts = state(&task).expect("a state");
        let spans = paint(&parts, &theme);
        assert_eq!(spans[0].content.as_ref(), "1");
        assert_eq!(spans[0].style, theme.text.strong, "a count is strong");
        assert_eq!(spans[1].content.as_ref(), " open finding");
        assert_eq!(spans[1].style, theme.text.dim, "a word is dim");
        let count = spans
            .iter()
            .find(|span| span.content.as_ref() == "1")
            .expect("the severity count");
        assert_eq!(count.style, theme.text.strong);
    }
}
