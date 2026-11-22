//! The sidebar's content: the configuration, and the cursor that picks the
//! orchestrator's model.
//!
//! The groups, in the order the screen reads them: the **agents** and the
//! model each runs, the **harness**, the **review-round limit**, and the
//! **order** — the numbered steps with their gates. The configuration is
//! provisional and lives in the tool's home. The agents group has a cursor:
//! the orchestrator's row is the one row a screen changes, because the
//! orchestrator is the agent hugo starts. The builder's and the reviewer's
//! rows are read-only.
//!
//! Nothing is dropped without a word: a row too wide for the pane is cut with
//! the theme's marker, and a group that does not fit the height is left out
//! whole and named with what it held.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use coding_crew_core::config::Config;
use coding_crew_theme::text::cut_to;
use coding_crew_theme::{Theme, selected_line};

use crate::config::{ConfigRead, ConfigState};

/// The sidebar's groups, in the order they are read.
pub const AGENTS_TITLE: &str = "agents";
pub const HARNESS_TITLE: &str = "harness";
pub const LIMIT_TITLE: &str = "review-round limit";
pub const ORDER_TITLE: &str = "order";

/// The sidebar's named empty state: there is no home, so nothing is configured.
pub const EMPTY: &str = "no configuration yet";
/// The sidebar's state when the file is there but cannot be read.
pub const UNREADABLE: &str = "configuration is unreadable";
/// The line that says the defaults are what the tool would use, beside an
/// error: the human sees both what is wrong and what is in force.
pub const USING_DEFAULTS: &str = "using the defaults:";
/// The line that says a pick could not be saved. The configuration on screen
/// is the one still in force.
pub const NOT_SAVED: &str = "the pick was not saved";

/// One row of a group, before it is cut to the pane.
struct Row {
    text: String,
    style: Style,
    /// The cursor is on this row.
    selected: bool,
}

/// One group of the sidebar: its title and its rows.
struct Group {
    title: &'static str,
    style: Style,
    rows: Vec<Row>,
}

/// What the sidebar drew, and whether the cursor's group is on screen.
pub struct Drawn {
    /// The lines, cut to the pane.
    pub lines: Vec<Line<'static>>,
    /// The agents group is drawn: the cursor has rows to be on.
    pub agents: bool,
}

/// The sidebar's lines, at a width and a height, cut to fit.
///
/// `notice` is a pick that could not be saved: it is drawn first, as the
/// sidebar's leading rows, so the fit counts it. `cursor` is the agents
/// group's row the cursor is on; `usize::MAX` draws no cursor. A group that
/// does not fit the height is left out whole; one line under the groups says
/// `not shown:` with each group's name and how much it held, the way the main
/// pane names the blocks it leaves out. An unreadable configuration names the
/// file and the error, then shows the defaults beside them. An old
/// `config.json` is named at the top: it is never read silently.
pub fn lines(
    state: &ConfigRead,
    notice: Option<&crate::app::ReadError>,
    theme: &Theme,
    width: usize,
    height: usize,
    cursor: usize,
) -> Drawn {
    if width == 0 || height == 0 {
        return Drawn {
            lines: Vec::new(),
            agents: false,
        };
    }
    // Each prefix row carries the short name a marker uses for it when the
    // row does not fit. The pick's notice comes first: it says what the
    // screen just failed to do. The error comes before the old file's
    // notice: what is wrong outranks what is merely ignored.
    let mut prefix: Vec<(String, Line<'static>)> = Vec::new();
    if let Some(error) = notice {
        prefix.push((
            NOT_SAVED.to_string(),
            line(NOT_SAVED, theme.text.normal, width),
        ));
        prefix.push((
            format!("{} error", error.file),
            line(
                &format!("{}: {}", error.file, error.error),
                theme.semantic.error,
                width,
            ),
        ));
    }
    match &state.state {
        ConfigState::Absent => prefix.push((EMPTY.to_string(), line(EMPTY, theme.text.dim, width))),
        ConfigState::Config(config) => {
            if let Some(file) = &state.legacy {
                prefix.push((
                    legacy_notice(file),
                    line(&legacy_notice(file), theme.text.dim, width),
                ));
            }
            return fits(prefix, groups(config, theme, cursor), theme, width, height);
        }
        ConfigState::Unreadable(error) => {
            prefix.push((
                UNREADABLE.to_string(),
                line(UNREADABLE, theme.text.normal, width),
            ));
            prefix.push((
                format!("{} error", error.file),
                line(
                    &format!("{}: {}", error.file, error.error),
                    theme.semantic.error,
                    width,
                ),
            ));
            prefix.push((
                USING_DEFAULTS.to_string(),
                line(USING_DEFAULTS, theme.text.dim, width),
            ));
        }
    }
    if let Some(file) = &state.legacy {
        prefix.push((
            legacy_notice(file),
            line(&legacy_notice(file), theme.text.dim, width),
        ));
    }
    match &state.state {
        ConfigState::Unreadable(_) => fits(
            prefix,
            groups(&Config::default(), theme, usize::MAX),
            theme,
            width,
            height,
        ),
        _ => Drawn {
            lines: prefix
                .into_iter()
                .map(|(_, line)| line)
                .take(height)
                .collect(),
            agents: false,
        },
    }
}

/// The notice that an old configuration is beside the new one.
fn legacy_notice(file: &str) -> String {
    format!("{file} is not read")
}

/// The configuration's groups, in the order they are read.
fn groups(config: &Config, theme: &Theme, cursor: usize) -> Vec<Group> {
    let title = theme.text.strong;
    vec![
        Group {
            title: AGENTS_TITLE,
            style: title,
            rows: agent_rows(config, theme, cursor),
        },
        Group {
            title: HARNESS_TITLE,
            style: title,
            rows: vec![
                Row {
                    text: config.harness.tool.clone(),
                    style: theme.text.normal,
                    selected: false,
                },
                Row {
                    text: format!("  {}", config.harness.driven),
                    style: theme.text.dim,
                    selected: false,
                },
            ],
        },
        Group {
            title: LIMIT_TITLE,
            style: title,
            rows: vec![Row {
                text: format!("{} build-and-fix rounds", config.round_limit),
                style: theme.text.normal,
                selected: false,
            }],
        },
        Group {
            title: ORDER_TITLE,
            style: title,
            rows: order_rows(config, theme),
        },
    ]
}

/// One entry per agent: its name and role, then its model — the provider and
/// the model's own name — on the rows under it. The cursor is on an entry's
/// head row.
fn agent_rows(config: &Config, theme: &Theme, cursor: usize) -> Vec<Row> {
    let mut rows = Vec::new();
    for (index, agent) in config.agents.iter().enumerate() {
        // A name that is its role is not said twice.
        let head = if agent.name == agent.role.as_str() {
            agent.name.clone()
        } else {
            format!("{} · {}", agent.name, agent.role.as_str())
        };
        let selected = index == cursor;
        rows.push(Row {
            text: head,
            style: theme.text.normal,
            selected,
        });
        match agent.model.split_once('/') {
            Some((provider, model)) => {
                rows.push(Row {
                    text: format!("  {provider}"),
                    style: theme.text.dim,
                    selected: false,
                });
                rows.push(Row {
                    text: format!("  {model}"),
                    style: theme.text.dim,
                    selected: false,
                });
            }
            None => rows.push(Row {
                text: format!("  {}", agent.model),
                style: theme.text.dim,
                selected: false,
            }),
        }
    }
    rows
}

/// The order's steps, numbered, each with the gates it passes.
fn order_rows(config: &Config, theme: &Theme) -> Vec<Row> {
    let mut rows = Vec::new();
    for (index, step) in config.order.iter().enumerate() {
        rows.push(Row {
            text: format!("{} {} · {}", index + 1, step.step, step.role.as_str()),
            style: theme.text.normal,
            selected: false,
        });
        for gate in &step.gates {
            rows.push(Row {
                text: format!("  {}", gate.says()),
                style: theme.text.dim,
                selected: false,
            });
        }
    }
    rows
}

/// The groups as lines, with the ones that do not fit named under them.
///
/// The prefix — an old file's notice, an error and the line that says the
/// defaults follow — is drawn first: what is wrong is never hidden by what
/// would be used.
fn fits(
    prefix: Vec<(String, Line<'static>)>,
    groups: Vec<Group>,
    theme: &Theme,
    width: usize,
    height: usize,
) -> Drawn {
    let total: usize = groups.iter().map(|group| group.rows.len() + 1).sum();
    if prefix.len() + total <= height {
        let mut lines: Vec<Line<'static>> = prefix.into_iter().map(|(_, line)| line).collect();
        lines.extend(drawn(&groups, width, theme));
        return Drawn {
            lines,
            agents: !groups.is_empty(),
        };
    }
    // One row is the announcement's: what fits beside it is drawn whole, and
    // the rest is named with what it held. A prefix longer than the pane is
    // shown as far as it goes; only a one-row pane gives it all up for the
    // marker, because the state has to say what it cannot show.
    let room = height.saturating_sub(1);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut kept = 0;
    if prefix.len() <= room {
        lines.extend(prefix.iter().map(|(_, line)| line.clone()));
        kept = prefix.len();
    } else if prefix.len() > height {
        if height <= 1 {
            // One row: the marker is the row, and it names the state.
        } else {
            lines.extend(prefix.iter().take(height).map(|(_, line)| line.clone()));
            return Drawn {
                lines,
                agents: false,
            };
        }
    } else {
        lines.extend(prefix.iter().take(room).map(|(_, line)| line.clone()));
        kept = room;
    }
    let mut used = lines.len();
    let mut shown = 0;
    for group in &groups {
        let held = group.rows.len() + 1;
        if used + held > room {
            break;
        }
        used += held;
        shown += 1;
    }
    lines.extend(drawn(&groups[..shown], width, theme));
    let mut hidden: Vec<String> = prefix[kept..]
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    hidden.extend(
        groups[shown..]
            .iter()
            .map(|group| format!("{} ({} hidden)", group.title, group.rows.len())),
    );
    if !hidden.is_empty() && lines.len() < height {
        lines.push(line(
            &format!("not shown: {}", hidden.join(", ")),
            theme.text.dim,
            width,
        ));
    }
    Drawn {
        lines,
        agents: shown >= 1,
    }
}

/// The groups drawn: each title, then its rows. The cursor's row takes the
/// selection's paint, bar to the pane's edge.
fn drawn(groups: &[Group], width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for group in groups {
        lines.push(line(group.title, group.style, width));
        for row in &group.rows {
            let text = line(&row.text, row.style, width);
            lines.push(if row.selected {
                selected_line(text, width, theme)
            } else {
                text
            });
        }
    }
    lines
}

fn line(text: &str, style: Style, width: usize) -> Line<'static> {
    Line::from(Span::styled(cut_to(text, width), style))
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_crew_core::config::{Agent, CONFIG_FILE, Harness, OrderStep};
    use coding_crew_theme::Preset;

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    fn configured() -> ConfigRead {
        ConfigRead::of(ConfigState::Config(Config::default()))
    }

    fn unreadable() -> ConfigRead {
        // The error is the reader's own: a row-level wording the pipeline
        // never produces is not a test of the screen.
        let error = Config::from_toml("round_limit = \n")
            .expect_err("a broken configuration")
            .to_string();
        ConfigRead::of(ConfigState::Unreadable(crate::app::ReadError {
            file: CONFIG_FILE.to_string(),
            error,
        }))
    }

    /// The lines' text, one string per line.
    fn text(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    fn rendered(state: &ConfigRead, width: usize, height: usize) -> Vec<String> {
        rendered_with(state, width, height, usize::MAX)
    }

    /// The lines' text with the cursor on an agent row (or on none).
    fn rendered_with(
        state: &ConfigRead,
        width: usize,
        height: usize,
        cursor: usize,
    ) -> Vec<String> {
        text(&lines(state, None, &theme(), width, height, cursor).lines)
    }

    #[test]
    fn the_groups_are_the_agents_the_harness_the_limit_and_the_order() {
        let rows = rendered(&configured(), 30, 40);
        let at = |needle: &str| {
            rows.iter()
                .position(|row| row == needle)
                .unwrap_or_else(|| panic!("{needle} is not a group: {rows:#?}"))
        };
        let agents = at(AGENTS_TITLE);
        let harness = at(HARNESS_TITLE);
        let limit = at(LIMIT_TITLE);
        let order = at(ORDER_TITLE);
        assert!(agents < harness, "{agents} {harness}");
        assert!(harness < limit, "{harness} {limit}");
        assert!(limit < order, "{limit} {order}");
    }

    #[test]
    fn every_agent_is_its_role_and_the_model_it_runs() {
        let rows = rendered(&configured(), 30, 40);
        let text = rows.join("\n");
        for fact in [
            "builder · runner",
            "reviewer",
            "orchestrator",
            "opencode-go",
            "deepseek-v4.1-flash#high",
            "glm-5.3#high",
        ] {
            assert!(text.contains(fact), "{fact} is not on the sidebar: {text}");
        }
        // The name is said once when it is the role.
        assert!(!text.contains("reviewer · reviewer"), "{text}");
        assert!(!text.contains("orchestrator · orchestrator"), "{text}");
        // The provider and the model's own name are both whole.
        assert!(
            rows.iter()
                .any(|row| row.trim() == "deepseek-v4.1-flash#high"),
            "the model's variant is cut: {text}"
        );
        assert!(
            !text.contains('…'),
            "the default configuration is cut: {text}"
        );
    }

    #[test]
    fn the_harness_names_the_tool_and_the_way_it_is_driven() {
        let rows = rendered(&configured(), 30, 40);
        let text = rows.join("\n");
        assert!(text.contains("opencode"), "{text}");
        assert!(text.contains("--standalone · role config"), "{text}");
    }

    #[test]
    fn the_round_limit_names_the_set_it_counts() {
        let rows = rendered(&configured(), 30, 40);
        assert!(
            rows.iter().any(|row| row == "5 build-and-fix rounds"),
            "{rows:#?}"
        );
    }

    #[test]
    fn the_order_is_numbered_and_each_step_carries_its_gates() {
        let rows = rendered(&configured(), 30, 40);
        let text = rows.join("\n");
        for (number, step, role) in [
            (1, "groom", "creator"),
            (2, "build", "runner"),
            (3, "self-review", "runner"),
            (4, "review", "reviewer"),
            (5, "remediate", "runner"),
            (6, "re-review", "reviewer"),
        ] {
            let head = format!("{number} {step} · {role}");
            assert!(text.contains(&head), "{head} is not on the sidebar: {text}");
        }
        for gate in [
            "groomed → approved",
            "approved → executing",
            "executing → code-review",
            "code-review → done",
            "code-review → remediating",
            "code-review → needs-human",
            "remediating → code-review",
        ] {
            assert!(text.contains(gate), "{gate} is not on the sidebar: {text}");
        }
        // The numbers are the order the rows are read in.
        let numbers: Vec<String> = rows
            .iter()
            .filter(|row| row.contains(" · "))
            .filter(|row| {
                row.split_once(' ')
                    .is_some_and(|(head, _)| head.parse::<u32>().is_ok())
            })
            .map(|row| row.chars().next().expect("a digit").to_string())
            .collect();
        assert_eq!(numbers, ["1", "2", "3", "4", "5", "6"], "{rows:#?}");
    }

    #[test]
    fn every_row_fits_the_pane_and_a_row_too_wide_is_cut_with_the_marker() {
        for width in 1..=40usize {
            for row in rendered(&configured(), width, 100) {
                assert!(
                    coding_crew_theme::text::text_width(&row) <= width,
                    "{width}: {row:?}"
                );
            }
        }
        // A row that does not fit is cut with the theme's marker, never
        // silently: the model's name is whole at 30 and marked at 12.
        let narrow = rendered(&configured(), 12, 40);
        assert!(
            narrow.iter().any(|row| row.ends_with('…')),
            "nothing is marked: {narrow:#?}"
        );
        assert!(
            !narrow
                .iter()
                .any(|row| row.contains("deepseek-v4.1-flash#high")),
            "a row was drawn past the pane: {narrow:#?}"
        );
    }

    #[test]
    fn a_group_that_does_not_fit_is_left_out_whole_and_named_with_what_it_held() {
        // Sixteen rows hold the agents, the harness and the limit; the
        // order's fifteen do not fit, so it is left out whole and named.
        let rows = rendered(&configured(), 30, 17);
        assert!(rows.len() <= 17, "{rows:#?}");
        let last = rows.last().expect("the announcement");
        assert_eq!(last, "not shown: order (14 hidden)", "{rows:#?}");
        // No part of the order is drawn, and its title is not a group on its
        // own.
        assert!(!rows.iter().any(|row| row == ORDER_TITLE), "{rows:#?}");
        assert!(!rows.iter().any(|row| row.contains("groom")), "{rows:#?}");
    }

    #[test]
    fn a_short_pane_keeps_the_announcement_and_never_a_bare_blank() {
        let all = rendered(&configured(), 30, 40);
        for height in 1..=40usize {
            let rows = rendered(&configured(), 30, height);
            assert!(rows.len() <= height, "{height}: {rows:#?}");
            let total: usize = all.len();
            if height < total {
                assert!(
                    rows.iter().any(|row| row.starts_with("not shown:")),
                    "{height}: the hidden groups are not named: {rows:#?}"
                );
            }
        }
    }

    #[test]
    fn the_empty_state_is_the_named_one_and_on_the_dim_slot() {
        let drawn = lines(
            &ConfigRead::of(ConfigState::Absent),
            None,
            &theme(),
            30,
            40,
            usize::MAX,
        );
        assert_eq!(text(&drawn.lines), [EMPTY]);
        assert!(!drawn.agents, "the cursor has no rows in the empty state");
        assert_eq!(drawn.lines[0].spans[0].style, theme().text.dim);
    }

    #[test]
    fn a_configuration_that_cannot_be_read_is_named_not_called_empty() {
        let state = unreadable();
        let drawn = lines(&state, None, &theme(), 40, 40, usize::MAX);
        let rows = text(&drawn.lines);
        assert_eq!(rows[0], UNREADABLE);
        assert!(rows[1].contains("config.toml"), "{rows:#?}");
        assert!(rows[1].contains("syntax error"), "{rows:#?}");
        assert_eq!(rows[2], USING_DEFAULTS);
        assert!(!rows.iter().any(|row| row == EMPTY), "{rows:#?}");
        // The defaults are drawn, but they are not the human's to change.
        assert!(drawn.agents, "the defaults' agents are not drawn");
        // The error is painted on the failure slot, as raul paints one.
        assert_eq!(drawn.lines[1].spans[0].style, theme().semantic.error);
    }

    #[test]
    fn an_unreadable_configuration_shows_the_defaults_beside_it() {
        // The human sees what is wrong and what the tool would use: both.
        let rows = rendered(&unreadable(), 30, 40);
        let text = rows.join("\n");
        for fact in [
            UNREADABLE,
            "syntax error",
            USING_DEFAULTS,
            AGENTS_TITLE,
            "builder · runner",
            HARNESS_TITLE,
            LIMIT_TITLE,
            "5 build-and-fix rounds",
            ORDER_TITLE,
            "groomed → approved",
        ] {
            assert!(
                text.contains(fact),
                "{fact} is not beside the error: {text}"
            );
        }
        // The error outranks the defaults: at two rows the error is whole and
        // the defaults' line is the one that gives way.
        let rows = rendered(&unreadable(), 30, 2);
        assert_eq!(rows[0], UNREADABLE);
        assert!(rows[1].contains("syntax error"), "{rows:#?}");
        assert!(!rows.iter().any(|row| row == USING_DEFAULTS), "{rows:#?}");
    }

    #[test]
    fn a_one_row_pane_says_what_the_unreadable_state_cannot_show() {
        // The state keeps its rows or says they are hidden: a one-row pane
        // neither clips them whole nor leaves a bare state.
        let rows = rendered(&unreadable(), 60, 1);
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert!(rows[0].starts_with("not shown:"), "{rows:#?}");
        assert!(rows[0].contains(UNREADABLE), "{rows:#?}");
        // Two rows keep the state and the error whole, which outrank the
        // defaults and the groups below them.
        let rows = rendered(&unreadable(), 60, 2);
        assert_eq!(rows[0], UNREADABLE);
        assert!(rows[1].contains("syntax error"), "{rows:#?}");
    }

    #[test]
    fn a_three_row_pane_does_not_promise_the_defaults_it_cannot_show() {
        // The prefix fills the pane whole and no group fits: the defaults'
        // line gives way to the marker that names it and hides nothing.
        let rows = rendered(&unreadable(), 60, 3);
        assert_eq!(rows[0], UNREADABLE);
        assert!(!rows.iter().any(|row| row == USING_DEFAULTS), "{rows:#?}");
        assert!(
            rows.iter().any(|row| row.starts_with("not shown:")),
            "{rows:#?}"
        );
        assert!(rows.join("\n").contains(USING_DEFAULTS), "{rows:#?}");
    }

    #[test]
    fn an_old_json_configuration_is_named_and_never_read() {
        let state = ConfigRead {
            state: ConfigState::Config(Config::default()),
            legacy: Some("config.json".to_string()),
        };
        let rows = rendered(&state, 30, 40);
        let notice = rows.first().expect("the notice");
        assert!(notice.contains("config.json"), "{rows:#?}");
        assert!(notice.contains("not read"), "{rows:#?}");
        // The configuration is still drawn under it.
        assert!(
            rows.join("\n").contains("build-and-fix rounds"),
            "{rows:#?}"
        );
        // The notice is secondary text, not the failure slot.
        let drawn = lines(&state, None, &theme(), 30, 40, usize::MAX);
        assert_eq!(drawn.lines[0].spans[0].style, theme().text.dim);
    }

    #[test]
    fn an_old_json_file_is_named_beside_an_unreadable_one() {
        let state = ConfigRead {
            state: ConfigState::Unreadable(crate::app::ReadError {
                file: CONFIG_FILE.to_string(),
                error: "syntax error at line 1, column 15: expected a value".to_string(),
            }),
            legacy: Some("config.json".to_string()),
        };
        let rows = rendered(&state, 30, 40);
        // The error comes first; the old file's notice is kept after it.
        assert_eq!(rows[0], UNREADABLE);
        assert!(rows[1].contains(CONFIG_FILE), "{rows:#?}");
        assert!(rows[1].contains("syntax error"), "{rows:#?}");
        assert!(
            rows.iter().any(|row| row.contains("config.json")),
            "{rows:#?}"
        );
        assert!(
            rows.join("\n").contains("build-and-fix rounds"),
            "{rows:#?}"
        );
    }

    #[test]
    fn the_error_outranks_the_legacy_notice_when_both_are_there() {
        // With an old config.json and a broken config.toml at once, the
        // error is the first line, not the first to be cut.
        let state = ConfigRead {
            state: ConfigState::Unreadable(crate::app::ReadError {
                file: CONFIG_FILE.to_string(),
                error: "syntax error at line 1, column 15: expected a value".to_string(),
            }),
            legacy: Some("config.json".to_string()),
        };
        // At three rows the error is still the first line after the state.
        let rows = rendered(&state, 60, 3);
        assert_eq!(rows[0], UNREADABLE, "{rows:#?}");
        assert!(rows[1].contains("syntax error"), "{rows:#?}");
        // At four rows the notice is kept too, after the error.
        for height in [4usize, 5] {
            let rows = rendered(&state, 60, height);
            assert_eq!(rows[0], UNREADABLE, "{height}: {rows:#?}");
            let error = rows
                .iter()
                .position(|row| row.contains("syntax error") || row.contains("config.toml error"))
                .unwrap_or_else(|| panic!("{height}: the error is gone: {rows:#?}"));
            let legacy = rows
                .iter()
                .position(|row| row.contains("config.json"))
                .unwrap_or_else(|| panic!("{height}: the notice is gone: {rows:#?}"));
            assert!(
                error < legacy,
                "{height}: the notice outranks the error: {rows:#?}"
            );
        }
    }

    #[test]
    fn a_short_pane_never_drops_a_group_without_a_word() {
        // With the three prefix rows of an unreadable configuration, the
        // groups still give way whole and are named.
        for height in 1..=24usize {
            let rows = rendered(&unreadable(), 30, height);
            assert!(rows.len() <= height, "{height}: {rows:#?}");
            if height >= 4 {
                assert!(
                    rows.iter().any(|row| row.starts_with("not shown:")),
                    "{height}: the hidden groups are not named: {rows:#?}"
                );
            }
        }
    }

    #[test]
    fn a_pick_notice_is_budgeted_with_the_groups_and_nothing_is_cut_silently() {
        // The notice is two rows drawn inside the fit: the groups it pushes
        // out are named, and the pane never holds more rows than it has.
        let notice = crate::app::ReadError {
            file: CONFIG_FILE.to_string(),
            error: "denied".to_string(),
        };
        let full = rendered(&configured(), 30, 1000).len() + 2;
        for height in 3..=40usize {
            let drawn = lines(
                &configured(),
                Some(&notice),
                &theme(),
                30,
                height,
                usize::MAX,
            );
            let rows = text(&drawn.lines);
            assert!(rows.len() <= height, "{height}: {rows:#?}");
            assert_eq!(rows[0], NOT_SAVED, "{height}: {rows:#?}");
            let named = rows.iter().any(|row| row.starts_with("not shown:"));
            assert!(
                named || rows.len() == full,
                "{height}: a group is gone without a word: {rows:#?}"
            );
        }
    }

    #[test]
    fn the_cursor_is_on_an_agents_head_row_and_the_bar_reaches_the_edge() {
        let drawn = lines(&configured(), None, &theme(), 30, 40, 2);
        let selected: Vec<&Line<'static>> = drawn
            .lines
            .iter()
            .filter(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style == theme().text.selected)
            })
            .collect();
        assert_eq!(selected.len(), 1, "the cursor covers more than one row");
        let says: String = selected[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(says.starts_with("orchestrator"), "{says:?}");
        assert_eq!(
            selected[0].width(),
            30,
            "the selection bar stops before the pane's edge"
        );
        assert!(drawn.agents, "the cursor's group is not drawn");
    }

    #[test]
    fn no_cursor_paints_no_row() {
        let drawn = lines(&configured(), None, &theme(), 30, 40, usize::MAX);
        assert!(
            !drawn.lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style == theme().text.selected)
            }),
            "a cursor is painted where there is none"
        );
    }

    #[test]
    fn a_short_pane_that_leaves_the_agents_out_says_the_cursor_has_nowhere_to_be() {
        // The agents group is left out whole, and the announcement says so:
        // the cursor must not act on a row the pane does not draw.
        let drawn = lines(&configured(), None, &theme(), 30, 5, 0);
        assert!(!drawn.agents, "{:#?}", text(&drawn.lines));
        assert!(
            drawn
                .lines
                .iter()
                .any(|line| line.spans[0].content.starts_with("not shown:")),
            "{:#?}",
            text(&drawn.lines)
        );
    }

    #[test]
    fn a_configuration_of_another_shape_is_drawn_as_it_is() {
        // The pane reads the configuration; it does not assert the project's
        // flow over a human's. A four-step order with two agents is drawn.
        let config = Config {
            agents: vec![
                Agent {
                    name: "builder".to_string(),
                    role: coding_crew_core::Role::Runner,
                    model: "provider/model#low".to_string(),
                },
                Agent {
                    name: "reviewer".to_string(),
                    role: coding_crew_core::Role::Reviewer,
                    model: "provider/model#low".to_string(),
                },
            ],
            harness: Harness {
                tool: "a-harness".to_string(),
                driven: "driven this way".to_string(),
            },
            round_limit: 2,
            order: vec![OrderStep {
                step: "plan".to_string(),
                role: coding_crew_core::Role::Creator,
                gates: vec![coding_crew_core::Gate {
                    from: coding_crew_core::Status::Draft,
                    to: coding_crew_core::Status::Groomed,
                }],
            }],
        };
        let rows = rendered(&ConfigRead::of(ConfigState::Config(config)), 30, 40);
        let text = rows.join("\n");
        for fact in [
            "2 build-and-fix rounds",
            "a-harness",
            "driven this way",
            "1 plan · creator",
            "draft → groomed",
        ] {
            assert!(text.contains(fact), "{fact}: {text}");
        }
    }
}
