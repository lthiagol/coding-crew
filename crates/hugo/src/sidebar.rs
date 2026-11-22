//! The sidebar's content: the configuration, and the cursor that picks each
//! agent's harness, model and effort.
//!
//! The groups, in the order the screen reads them: the **agents**, each with
//! its harness, model and effort, the **review-round limit**, and the
//! **order** — the numbered steps with their gates. The configuration is
//! provisional and lives in the tool's home. The agents group has a cursor on
//! each agent's harness, model and effort: every one of those rows is picked
//! from a choice, and the agent's name and role are read-only.
//!
//! Nothing is dropped without a word: a row too wide for the pane is cut with
//! the theme's marker, and a group that does not fit the height is left out
//! whole and named with what it held.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use coding_crew_core::FlowRole;
use coding_crew_core::config::{Agent, Config};
use coding_crew_theme::text::{cut_to, text_width};
use coding_crew_theme::{Icon, IconSet, Theme, role_style, selected_line};

use crate::config::{ConfigRead, ConfigState};
use crate::pick::{Levels, effort_word};

/// The rows of an agent that a human picks, in the order they are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Harness,
    Model,
    Effort,
}

/// The fields of an agent, in the order they are drawn.
pub const FIELDS: [Field; 3] = [Field::Harness, Field::Model, Field::Effort];

impl Field {
    /// The field's name, as its row and its choice say it.
    pub fn label(self) -> &'static str {
        match self {
            Field::Harness => "harness",
            Field::Model => "model",
            Field::Effort => "effort",
        }
    }
}

/// The number of rows the cursor moves over: each agent's three fields.
pub fn pick_rows(config: &Config) -> usize {
    config.agents.len() * FIELDS.len()
}

/// The cursor's row for an agent's field.
pub fn cursor_of(agent: usize, field: Field) -> usize {
    let position = FIELDS
        .iter()
        .position(|known| *known == field)
        .unwrap_or_default();
    agent * FIELDS.len() + position
}

/// The agent and the field a cursor's row is.
pub fn field_at(cursor: usize) -> (usize, Field) {
    (cursor / FIELDS.len(), FIELDS[cursor % FIELDS.len()])
}

/// The sidebar's groups, in the order they are read. Each label is the dim
/// upper-case word the screen draws.
pub const AGENTS_TITLE: &str = "AGENTS";
pub const LIMIT_TITLE: &str = "ROUNDS";
pub const ORDER_TITLE: &str = "ORDER";

/// The columns the cursor's bar takes: the first column of every row.
const BAR: usize = 1;

/// The columns a field's key takes: the longest key, `harness`, and a space.
const KEY_COLUMN: usize = 8;

/// The sidebar's named empty state: there is no home, so nothing is configured.
pub const EMPTY: &str = "no configuration yet";
/// The sidebar's state when the file is there but cannot be read.
pub const UNREADABLE: &str = "configuration is unreadable";
/// The line that says the defaults are what the tool would use, beside an
/// error: the human sees both what is wrong and what is in force.
pub const USING_DEFAULTS: &str = "using the defaults:";
/// The tag that marks what this session has changed from the file: a group or
/// an agent that differs from the file carries it.
pub const SESSION_TAG: &str = "session";

/// One row of a group, before it is cut to the pane: its spans, each with
/// the style it is painted in.
struct Row {
    spans: Vec<(String, Style)>,
    /// The cursor is on this row.
    selected: bool,
}

impl Row {
    /// A row of one span, painted in one style.
    fn plain(text: String, style: Style, selected: bool) -> Self {
        Row {
            spans: vec![(text, style)],
            selected,
        }
    }
}

/// One group of the sidebar: its title, the tag the title carries, and its rows.
struct Group {
    title: &'static str,
    tag: Option<&'static str>,
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
/// `session` is the crew this session runs, when there is one: it is drawn in
/// place of the file's, and the values that differ from the file carry the
/// session tag. `None` draws the file as it is. `cursor` is the agents group's
/// row the cursor is on, as [`cursor_of`] numbers it; `usize::MAX` draws no
/// cursor. A group that does not fit the height is left out whole; one line
/// under the groups says `not shown:` with each group's name and how much it
/// held, the way the main pane names the blocks it leaves out. An unreadable
/// configuration names the file and the error, then shows the defaults beside
/// them. An old `config.json` is named at the top: it is never read silently.
#[allow(clippy::too_many_arguments)]
pub fn lines(
    state: &ConfigRead,
    session: Option<&Config>,
    levels: &Levels,
    theme: &Theme,
    icons: IconSet,
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
    // row does not fit. The error comes before the old file's notice: what is
    // wrong outranks what is merely ignored.
    let mut prefix: Vec<(String, Line<'static>)> = Vec::new();
    match &state.state {
        ConfigState::Absent => prefix.push((EMPTY.to_string(), line(EMPTY, theme.text.dim, width))),
        ConfigState::Config(config) => {
            if let Some(file) = &state.legacy {
                prefix.push((
                    legacy_notice(file),
                    line(&legacy_notice(file), theme.text.dim, width),
                ));
            }
            let shown = session.unwrap_or(config);
            return fits(
                prefix,
                groups(shown, config, levels, theme, icons, cursor),
                theme,
                icons,
                width,
                height,
            );
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
            groups(
                &Config::default(),
                &Config::default(),
                levels,
                theme,
                icons,
                usize::MAX,
            ),
            theme,
            icons,
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

/// The configuration's groups, in the order they are read. `config` is the
/// crew in force and `file` is the configuration it differs from: the agents
/// group carries the session tag when the crew's roles are not the file's.
fn groups(
    config: &Config,
    file: &Config,
    levels: &Levels,
    theme: &Theme,
    icons: IconSet,
    cursor: usize,
) -> Vec<Group> {
    let crew_differs = !same_crew(config, file);
    vec![
        Group {
            title: AGENTS_TITLE,
            tag: crew_differs.then_some(SESSION_TAG),
            rows: agent_rows(config, file, levels, theme, icons, cursor),
        },
        Group {
            title: LIMIT_TITLE,
            tag: None,
            rows: vec![Row::plain(
                format!("{} build-and-fix rounds", config.round_limit),
                theme.text.normal,
                false,
            )],
        },
        Group {
            title: ORDER_TITLE,
            tag: None,
            rows: order_rows(config, theme),
        },
    ]
}

/// Whether two configurations have the same roles in their crew.
fn same_crew(config: &Config, file: &Config) -> bool {
    config.agents.len() == file.agents.len()
        && config
            .agents
            .iter()
            .all(|agent| file.agents.iter().any(|known| known.role == agent.role))
}

/// Whether a field of an agent differs from the file's agent of the same role.
/// An agent the file does not have differs in every field.
fn differs(agent: &Agent, file: Option<&Agent>, field: Field) -> bool {
    let Some(file) = file else {
        return true;
    };
    match field {
        Field::Harness => agent.harness != file.harness,
        Field::Model => agent.model != file.model,
        Field::Effort => agent.effort != file.effort,
    }
}

/// One entry per agent: its role icon and its name, the name on the role's
/// slot and the role's word dim, then the harness it runs in, its model and
/// its effort on the rows under it. The keys are dim and aligned in a column;
/// the model is the strong value. The head row is not picked; the cursor is on
/// one of the three rows under it. A model that is empty, or an effort that is
/// absent, says so on the warning slot. An agent that differs from the file
/// carries the session tag on its head, and a field that differs has a strong
/// key.
fn agent_rows(
    config: &Config,
    file: &Config,
    levels: &Levels,
    theme: &Theme,
    icons: IconSet,
    cursor: usize,
) -> Vec<Row> {
    let mut rows = Vec::new();
    for (index, agent) in config.agents.iter().enumerate() {
        let was = file.agents.iter().find(|known| known.role == agent.role);
        // The agent is named by its role: its table is the role's name. The
        // role's word stays beside its icon: an icon decorates a word, it never
        // replaces one.
        let head = format!(
            "{} {}",
            role_icon(agent.role).glyph(icons),
            agent.role.as_str()
        );
        let mut spans = vec![(head, role_style(theme, agent.role))];
        if FIELDS.iter().any(|field| differs(agent, was, *field)) {
            spans.push((format!(" {SESSION_TAG}"), theme.text.dim));
        }
        rows.push(Row {
            spans,
            selected: false,
        });
        for field in FIELDS {
            let key = if differs(agent, was, field) {
                theme.text.strong
            } else {
                theme.text.dim
            };
            let (value, style) = match field {
                Field::Harness => (agent.harness.clone(), theme.text.normal),
                Field::Model if agent.model.is_empty() => {
                    ("pick a model".to_string(), theme.semantic.warn)
                }
                Field::Model => (agent.model.clone(), theme.text.strong),
                Field::Effort => match &agent.effort {
                    Some(effort) => (effort_word(effort).to_string(), theme.text.normal),
                    // A model that takes no level needs none: its listing says so.
                    None if levels.takes_none(agent) => ("—".to_string(), theme.text.dim),
                    None => ("pick an effort".to_string(), theme.semantic.warn),
                },
            };
            rows.push(Row {
                spans: vec![
                    (
                        format!("  {:<width$}", field.label(), width = KEY_COLUMN),
                        key,
                    ),
                    (value, style),
                ],
                selected: cursor_of(index, field) == cursor,
            });
        }
    }
    rows
}

/// The icon of a role. A creator grooms with a human, so it takes the
/// human's icon: the table has no creator of its own.
pub fn role_icon(role: FlowRole) -> Icon {
    match role {
        FlowRole::Creator => Icon::RoleHuman,
        FlowRole::Builder => Icon::RoleRunner,
        FlowRole::Reviewer => Icon::RoleReviewer,
        FlowRole::Orchestrator => Icon::RoleOrchestrator,
    }
}

/// The order's steps, numbered, each with the gates it passes.
fn order_rows(config: &Config, theme: &Theme) -> Vec<Row> {
    let mut rows = Vec::new();
    for (index, step) in config.order.iter().enumerate() {
        rows.push(Row::plain(
            format!("{} {} · {}", index + 1, step.step, step.role.as_str()),
            theme.text.normal,
            false,
        ));
        for gate in &step.gates {
            rows.push(Row::plain(
                format!("  {}", gate.says()),
                theme.text.dim,
                false,
            ));
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
    icons: IconSet,
    width: usize,
    height: usize,
) -> Drawn {
    let total: usize = groups.iter().map(|group| group.rows.len() + 1).sum();
    if prefix.len() + total <= height {
        let mut lines: Vec<Line<'static>> = prefix.into_iter().map(|(_, line)| line).collect();
        lines.extend(drawn(&groups, width, icons, theme));
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
    lines.extend(drawn(&groups[..shown], width, icons, theme));
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
/// selection's bar and tint, bar to the pane's edge.
fn drawn(groups: &[Group], width: usize, icons: IconSet, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for group in groups {
        lines.push(group_line(group.title, group.tag, width, theme));
        for row in &group.rows {
            lines.push(row_line(&row.spans, row.selected, width, icons, theme));
        }
    }
    lines
}

/// A group's title: the label bold on the accent, its tag dim after it when it
/// has one, then a dim rule to the pane's edge.
fn group_line(title: &str, tag: Option<&str>, width: usize, theme: &Theme) -> Line<'static> {
    let label = cut_to(title, width.saturating_sub(BAR));
    let mut used = text_width(&label);
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(label, theme.chrome.tab_active.add_modifier(Modifier::BOLD)),
    ];
    if let Some(tag) = tag {
        let tagged = format!(" {tag}");
        let tagged = cut_to(&tagged, width.saturating_sub(BAR + used));
        used += text_width(&tagged);
        spans.push(Span::styled(tagged, theme.text.dim));
    }
    let rest = width.saturating_sub(BAR + used);
    if rest > 1 {
        spans.push(Span::styled(
            format!(" {}", "─".repeat(rest - 1)),
            theme.text.dim,
        ));
    }
    Line::from(spans)
}

/// A row with the bar column first: the cursor bar in the focus slot when the
/// cursor is on it, a space otherwise, so every row shares one left edge. The
/// row's spans keep their own colours; a selected row takes the tint under
/// them.
fn row_line(
    spans: &[(String, Style)],
    selected: bool,
    width: usize,
    icons: IconSet,
    theme: &Theme,
) -> Line<'static> {
    let body = cut_spans(spans, width.saturating_sub(BAR));
    let (bar, body) = if selected {
        let bar = Span::styled(Icon::CursorBar.glyph(icons), theme.chrome.border_focus);
        (
            bar,
            selected_line(Line::from(body), width.saturating_sub(BAR), theme).spans,
        )
    } else {
        (Span::raw(" "), body)
    };
    let mut line = vec![bar];
    line.extend(body);
    Line::from(line)
}

/// The spans cut to `width` columns: whole spans while they fit, and the one
/// that runs out cut with the theme's marker.
fn cut_spans(spans: &[(String, Style)], width: usize) -> Vec<Span<'static>> {
    let mut left = width;
    let mut cut = Vec::new();
    for (text, style) in spans {
        if left == 0 {
            break;
        }
        let used = text_width(text);
        if used <= left {
            cut.push(Span::styled(text.clone(), *style));
            left -= used;
        } else {
            cut.push(Span::styled(cut_to(text, left), *style));
            break;
        }
    }
    cut
}

/// A line that is not a row: a title, a state or an announcement. It takes
/// the bar column too, so the sidebar has one left edge.
fn line(text: &str, style: Style, width: usize) -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled(cut_to(text, width.saturating_sub(BAR)), style),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_crew_core::config::{Agent, CONFIG_FILE, OrderStep};
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

    /// The lines' text, one string per line, without the bar column.
    fn text(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
                    .chars()
                    .skip(1)
                    .collect()
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
        text(
            &lines(
                state,
                None,
                &Levels::default(),
                &theme(),
                IconSet::Nerd,
                width,
                height,
                cursor,
            )
            .lines,
        )
    }

    #[test]
    fn the_groups_are_the_agents_the_limit_and_the_order() {
        let rows = rendered(&configured(), 30, 40);
        let at = |needle: &str| {
            rows.iter()
                .position(|row| row.starts_with(needle))
                .unwrap_or_else(|| panic!("{needle} is not a group: {rows:#?}"))
        };
        let agents = at(AGENTS_TITLE);
        let limit = at(LIMIT_TITLE);
        let order = at(ORDER_TITLE);
        assert!(agents < limit, "{agents} {limit}");
        assert!(limit < order, "{limit} {order}");
        // The harness is an agent's, not a group of its own.
        assert!(!rows.iter().any(|row| row == "HARNESS"), "{rows:#?}");
    }

    #[test]
    fn every_agent_is_its_role_then_its_harness_model_and_effort() {
        let rows = rendered(&configured(), 30, 40);
        let at = |needle: &str| {
            rows.iter()
                .position(|row| row.ends_with(needle))
                .unwrap_or_else(|| panic!("{needle} is not on the sidebar: {rows:#?}"))
        };
        let head = at("builder");
        assert_eq!(rows[head + 1], "  harness claude", "{rows:#?}");
        assert_eq!(rows[head + 2], "  model   claude-haiku-5-5", "{rows:#?}");
        assert_eq!(rows[head + 3], "  effort  high", "{rows:#?}");
        let reviewer = at("reviewer");
        assert_eq!(rows[reviewer + 2], "  model   claude-opus-5-5", "{rows:#?}");
        assert_eq!(rows[reviewer + 3], "  effort  medium", "{rows:#?}");
        // The name is said once when it is the role.
        let text = rows.join("\n");
        assert!(!text.contains("reviewer · reviewer"), "{text}");
        assert!(!text.contains("orchestrator · orchestrator"), "{text}");
        assert!(
            !text.contains('…'),
            "the default configuration is cut: {text}"
        );
    }

    #[test]
    fn a_model_that_takes_no_level_shows_a_dash_and_no_warning() {
        // The listing says the model takes no level: its effort is not a gap,
        // and the row says so with a dash.
        let mut config = Config::default();
        config.agents[1].harness = "opencode".to_string();
        config.agents[1].model = "opencode-go/kimi-k3".to_string();
        config.agents[1].effort = None;
        let mut levels = Levels::default();
        levels.insert(
            "opencode",
            coding_crew_core::catalog::Catalog::from_curated(&[
                coding_crew_core::harness::CuratedModel {
                    id: "opencode-go/kimi-k3",
                    efforts: &[],
                },
            ]),
        );
        let state = ConfigRead::of(ConfigState::Config(config));
        let drawn = lines(
            &state,
            None,
            &levels,
            &theme(),
            IconSet::Nerd,
            30,
            40,
            usize::MAX,
        );
        let text: Vec<String> = drawn
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.to_string())
                    .collect()
            })
            .collect();
        assert!(
            text.iter()
                .any(|row| row.contains("effort") && row.contains('—')),
            "{text:#?}"
        );
        assert!(
            !text.iter().any(|row| row.contains("pick an effort")),
            "{text:#?}"
        );
    }

    #[test]
    fn an_agent_with_no_effort_says_pick_an_effort_on_the_warning_slot() {
        let mut config = Config::default();
        config.agents[1].effort = None;
        let state = ConfigRead::of(ConfigState::Config(config));
        let drawn = lines(
            &state,
            None,
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            40,
            usize::MAX,
        );
        let row = drawn
            .lines
            .iter()
            .find(|line| {
                line.spans
                    .iter()
                    .any(|span| span.content.contains("pick an effort"))
            })
            .expect("the effort row");
        assert!(
            row.spans
                .iter()
                .any(|span| span.style == theme().semantic.warn),
            "{row:?}"
        );
        assert!(
            !rendered(&state, 30, 40)
                .iter()
                .any(|row| row.contains("default")),
            "an absent effort is not the default"
        );
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
            (2, "build", "builder"),
            (3, "self-review", "builder"),
            (4, "review", "reviewer"),
            (5, "remediate", "builder"),
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
            let drawn = lines(
                &configured(),
                None,
                &Levels::default(),
                &theme(),
                IconSet::Nerd,
                width,
                100,
                usize::MAX,
            );
            for line in &drawn.lines {
                assert!(line.width() <= width, "{width}: {line:?}");
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
        assert_eq!(last, "not shown: ORDER (14 hidden)", "{rows:#?}");
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
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            40,
            usize::MAX,
        );
        assert_eq!(text(&drawn.lines), [EMPTY]);
        assert!(!drawn.agents, "the cursor has no rows in the empty state");
        assert_eq!(drawn.lines[0].spans[1].style, theme().text.dim);
    }

    #[test]
    fn a_configuration_that_cannot_be_read_is_named_not_called_empty() {
        let state = unreadable();
        let drawn = lines(
            &state,
            None,
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            40,
            40,
            usize::MAX,
        );
        let rows = text(&drawn.lines);
        assert_eq!(rows[0], UNREADABLE);
        assert!(rows[1].contains("config.toml"), "{rows:#?}");
        assert!(rows[1].contains("syntax error"), "{rows:#?}");
        assert_eq!(rows[2], USING_DEFAULTS);
        assert!(!rows.iter().any(|row| row == EMPTY), "{rows:#?}");
        // The defaults are drawn, but they are not the human's to change.
        assert!(drawn.agents, "the defaults' agents are not drawn");
        // The error is painted on the failure slot, as raul paints one.
        assert_eq!(drawn.lines[1].spans[1].style, theme().semantic.error);
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
            "builder",
            "harness claude",
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
        let drawn = lines(
            &state,
            None,
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            40,
            usize::MAX,
        );
        assert_eq!(drawn.lines[0].spans[1].style, theme().text.dim);
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
    fn a_session_value_that_differs_from_the_file_is_tagged_on_its_agent() {
        // The session's model is drawn, the agent that changed says session,
        // and an agent that did not change carries no tag.
        let file = Config::default();
        let mut session = Config::default();
        session.agents[0].model = "claude-sonnet-5-5".to_string();
        let state = ConfigRead::of(ConfigState::Config(file));
        let drawn = lines(
            &state,
            Some(&session),
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            40,
            usize::MAX,
        );
        let rows = text(&drawn.lines);
        let head = rows
            .iter()
            .position(|row| row.contains("orchestrator"))
            .expect("the orchestrator's head");
        assert!(rows[head].ends_with("session"), "{rows:#?}");
        assert!(rows[head + 2].contains("claude-sonnet-5-5"), "{rows:#?}");
        let builder = rows
            .iter()
            .position(|row| row.contains("builder"))
            .expect("the builder's head");
        assert!(!rows[builder].contains("session"), "{rows:#?}");
    }

    #[test]
    fn a_crew_with_other_roles_than_the_file_tags_the_agents_group() {
        // One agent in the session and three in the file: the agents group
        // says session, and the roles the session left off are not drawn.
        let file = Config::default();
        let mut session = Config::default();
        session.agents.truncate(1);
        let state = ConfigRead::of(ConfigState::Config(file));
        let drawn = lines(
            &state,
            Some(&session),
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            40,
            usize::MAX,
        );
        let rows = text(&drawn.lines);
        assert!(
            rows[0].contains("AGENTS") && rows[0].contains("session"),
            "{rows:#?}"
        );
        // One agent's rows: its head, harness, model and effort.
        assert_eq!(
            rows.iter().filter(|row| row.contains("harness")).count(),
            1,
            "{rows:#?}"
        );
    }
    #[test]
    fn the_cursor_is_on_a_pickable_row_and_the_bar_reaches_the_edge() {
        let drawn = lines(
            &configured(),
            None,
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            40,
            cursor_of(2, Field::Model),
        );
        let selected: Vec<&Line<'static>> = drawn
            .lines
            .iter()
            .filter(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.bg == theme().text.selected_bg.bg)
            })
            .collect();
        assert_eq!(selected.len(), 1, "the cursor covers more than one row");
        let says: String = selected[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(says.starts_with('▌'), "{says:?}");
        assert!(says.contains("model   claude-opus-5-5"), "{says:?}");
        assert_eq!(
            selected[0].width(),
            30,
            "the selection bar stops before the pane's edge"
        );
        assert!(drawn.agents, "the cursor's group is not drawn");
    }

    #[test]
    fn a_model_that_is_empty_says_pick_a_model_on_the_warning_slot() {
        let mut config = Config::default();
        config.agents[1].model.clear();
        let state = ConfigRead::of(ConfigState::Config(config));
        let drawn = lines(
            &state,
            None,
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            40,
            usize::MAX,
        );
        let row = drawn
            .lines
            .iter()
            .find(|line| {
                line.spans
                    .iter()
                    .any(|span| span.content.contains("pick a model"))
            })
            .expect("the model row");
        assert!(
            row.spans
                .iter()
                .any(|span| span.style == theme().semantic.warn),
            "{row:?}"
        );
        // The cleared model's old id is not carried onto the row.
        assert!(
            !rendered(&state, 30, 40)
                .iter()
                .any(|row| row.contains("claude-haiku-5-5"))
        );
    }

    #[test]
    fn no_cursor_paints_no_row() {
        let drawn = lines(
            &configured(),
            None,
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            40,
            usize::MAX,
        );
        assert!(
            !drawn.lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.bg == theme().text.selected_bg.bg)
            }),
            "a cursor is painted where there is none"
        );
    }

    #[test]
    fn a_short_pane_that_leaves_the_agents_out_says_the_cursor_has_nowhere_to_be() {
        // The agents group is left out whole, and the announcement says so:
        // the cursor must not act on a row the pane does not draw.
        let drawn = lines(
            &configured(),
            None,
            &Levels::default(),
            &theme(),
            IconSet::Nerd,
            30,
            5,
            0,
        );
        assert!(!drawn.agents, "{:#?}", text(&drawn.lines));
        assert!(
            drawn
                .lines
                .iter()
                .any(|line| line.spans[1].content.starts_with("not shown:")),
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
                    role: coding_crew_core::FlowRole::Builder,
                    harness: "cursor-agent".to_string(),
                    model: "auto".to_string(),
                    effort: Some("low".to_string()),
                },
                Agent {
                    role: coding_crew_core::FlowRole::Reviewer,
                    harness: "cursor-agent".to_string(),
                    model: "auto".to_string(),
                    effort: None,
                },
            ],
            round_limit: 2,
            order: vec![OrderStep {
                step: "plan".to_string(),
                role: coding_crew_core::FlowRole::Creator,
                gates: vec![coding_crew_core::Gate {
                    from: coding_crew_core::Status::Draft,
                    to: coding_crew_core::Status::Groomed,
                }],
            }],
            ..Config::default()
        };
        let rows = rendered(&ConfigRead::of(ConfigState::Config(config)), 30, 40);
        let text = rows.join("\n");
        for fact in [
            "2 build-and-fix rounds",
            "harness cursor-agent",
            "effort  low",
            "1 plan · creator",
            "draft → groomed",
        ] {
            assert!(text.contains(fact), "{fact}: {text}");
        }
    }
}
