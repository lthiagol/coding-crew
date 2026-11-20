//! The sidebar's content: the configuration, read-only.
//!
//! The groups, in the order the screen reads them: the **agents** and the
//! model each runs, the **harness**, the **review-round limit**, and the
//! **order** — the numbered steps with their gates. The configuration is
//! provisional and lives in the tool's home; nothing here changes it.
//!
//! Nothing is dropped without a word: a row too wide for the pane is cut with
//! the theme's marker, and a group that does not fit the height is left out
//! whole and named with what it held.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use coding_crew_core::config::Config;
use coding_crew_theme::Theme;
use coding_crew_theme::text::cut_to;

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

/// One row of a group, before it is cut to the pane.
struct Row {
    text: String,
    style: Style,
}

/// One group of the sidebar: its title and its rows.
struct Group {
    title: &'static str,
    style: Style,
    rows: Vec<Row>,
}

/// The sidebar's lines, at a width and a height, cut to fit.
///
/// A group that does not fit the height is left out whole; one line under the
/// groups says `not shown:` with each group's name and how much it held, the
/// way the main pane names the blocks it leaves out. An unreadable
/// configuration names the file and the error, then shows the defaults beside
/// them. An old `config.json` is named at the top: it is never read silently.
pub fn lines(state: &ConfigRead, theme: &Theme, width: usize, height: usize) -> Vec<Line<'static>> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let mut prefix = Vec::new();
    if let Some(file) = &state.legacy {
        prefix.push(line(&legacy_notice(file), theme.text.dim, width));
    }
    match &state.state {
        ConfigState::Absent => {
            prefix.push(line(EMPTY, theme.text.dim, width));
            prefix.truncate(height);
            prefix
        }
        ConfigState::Config(config) => fits(prefix, groups(config, theme), theme, width, height),
        ConfigState::Unreadable(error) => {
            prefix.push(line(UNREADABLE, theme.text.normal, width));
            prefix.push(line(
                &format!("{}: {}", error.file, error.error),
                theme.semantic.error,
                width,
            ));
            prefix.push(line(USING_DEFAULTS, theme.text.dim, width));
            fits(
                prefix,
                groups(&Config::default(), theme),
                theme,
                width,
                height,
            )
        }
    }
}

/// The notice that an old configuration is beside the new one.
fn legacy_notice(file: &str) -> String {
    format!("{file} is not read")
}

/// The configuration's groups, in the order they are read.
fn groups(config: &Config, theme: &Theme) -> Vec<Group> {
    let title = theme.text.strong;
    vec![
        Group {
            title: AGENTS_TITLE,
            style: title,
            rows: agent_rows(config, theme),
        },
        Group {
            title: HARNESS_TITLE,
            style: title,
            rows: vec![
                Row {
                    text: config.harness.tool.clone(),
                    style: theme.text.normal,
                },
                Row {
                    text: format!("  {}", config.harness.driven),
                    style: theme.text.dim,
                },
            ],
        },
        Group {
            title: LIMIT_TITLE,
            style: title,
            rows: vec![Row {
                text: format!("{} build-and-fix rounds", config.round_limit),
                style: theme.text.normal,
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
/// the model's own name — on the rows under it.
fn agent_rows(config: &Config, theme: &Theme) -> Vec<Row> {
    let mut rows = Vec::new();
    for agent in &config.agents {
        // A name that is its role is not said twice.
        let head = if agent.name == agent.role.as_str() {
            agent.name.clone()
        } else {
            format!("{} · {}", agent.name, agent.role.as_str())
        };
        rows.push(Row {
            text: head,
            style: theme.text.normal,
        });
        match agent.model.split_once('/') {
            Some((provider, model)) => {
                rows.push(Row {
                    text: format!("  {provider}"),
                    style: theme.text.dim,
                });
                rows.push(Row {
                    text: format!("  {model}"),
                    style: theme.text.dim,
                });
            }
            None => rows.push(Row {
                text: format!("  {}", agent.model),
                style: theme.text.dim,
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
        });
        for gate in &step.gates {
            rows.push(Row {
                text: format!("  {}", gate.says()),
                style: theme.text.dim,
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
    prefix: Vec<Line<'static>>,
    groups: Vec<Group>,
    theme: &Theme,
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let total: usize = groups.iter().map(|group| group.rows.len() + 1).sum();
    if prefix.len() + total <= height {
        let mut lines = prefix;
        lines.extend(drawn(&groups, width));
        return lines;
    }
    // One row is the announcement's: the groups that fit beside it are drawn
    // whole, and the rest are named with what they held.
    let room = height.saturating_sub(1);
    let mut used = prefix.len();
    let mut shown = 0;
    for group in &groups {
        let held = group.rows.len() + 1;
        if used + held > room {
            break;
        }
        used += held;
        shown += 1;
    }
    let mut lines: Vec<Line<'static>> = prefix.into_iter().take(height).collect();
    lines.extend(drawn(&groups[..shown], width));
    if shown < groups.len() && lines.len() < height {
        let hidden: Vec<String> = groups[shown..]
            .iter()
            .map(|group| format!("{} ({} hidden)", group.title, group.rows.len()))
            .collect();
        lines.push(line(
            &format!("not shown: {}", hidden.join(", ")),
            theme.text.dim,
            width,
        ));
    }
    lines
}

/// The groups drawn: each title, then its rows.
fn drawn(groups: &[Group], width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for group in groups {
        lines.push(line(group.title, group.style, width));
        for row in &group.rows {
            lines.push(line(&row.text, row.style, width));
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
        ConfigRead::of(ConfigState::Unreadable(crate::app::ReadError {
            file: CONFIG_FILE.to_string(),
            error: "failed to parse the configuration at line 1, column 15: expected a value"
                .to_string(),
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
        text(&lines(state, &theme(), width, height))
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
        let lines = lines(&ConfigRead::of(ConfigState::Absent), &theme(), 30, 40);
        assert_eq!(text(&lines), [EMPTY]);
        assert_eq!(lines[0].spans[0].style, theme().text.dim);
    }

    #[test]
    fn a_configuration_that_cannot_be_read_is_named_not_called_empty() {
        let state = unreadable();
        let lines = lines(&state, &theme(), 40, 40);
        let rows = text(&lines);
        assert_eq!(rows[0], UNREADABLE);
        assert!(rows[1].contains("config.toml"), "{rows:#?}");
        assert!(rows[1].contains("failed to parse"), "{rows:#?}");
        assert_eq!(rows[2], USING_DEFAULTS);
        assert!(!rows.iter().any(|row| row == EMPTY), "{rows:#?}");
        // The error is painted on the failure slot, as raul paints one.
        assert_eq!(lines[1].spans[0].style, theme().semantic.error);
    }

    #[test]
    fn an_unreadable_configuration_shows_the_defaults_beside_it() {
        // The human sees what is wrong and what the tool would use: both.
        let rows = rendered(&unreadable(), 30, 40);
        let text = rows.join("\n");
        for fact in [
            UNREADABLE,
            "failed to parse",
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
        assert!(rows[1].contains("failed to parse"), "{rows:#?}");
        assert!(!rows.iter().any(|row| row == USING_DEFAULTS), "{rows:#?}");
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
        let lines = lines(&state, &theme(), 30, 40);
        assert_eq!(lines[0].spans[0].style, theme().text.dim);
    }

    #[test]
    fn an_old_json_file_is_named_beside_an_unreadable_one() {
        let state = ConfigRead {
            state: ConfigState::Unreadable(crate::app::ReadError {
                file: CONFIG_FILE.to_string(),
                error: "failed to parse the configuration".to_string(),
            }),
            legacy: Some("config.json".to_string()),
        };
        let rows = rendered(&state, 30, 40);
        assert!(rows[0].contains("config.json"), "{rows:#?}");
        assert_eq!(rows[1], UNREADABLE);
        assert!(
            rows.join("\n").contains("build-and-fix rounds"),
            "{rows:#?}"
        );
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
