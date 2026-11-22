//! The icons raul and hugo draw, each with its glyph in both sets.
//!
//! An icon decorates a word; it never replaces one. The table below is the
//! only place a glyph is written: every other module names an [`Icon`] and
//! asks it for the glyph of the set in force. The names and glyphs are the
//! spec's, in `docs/tui/redesign.md` § The icons.

use std::env;

/// The environment variable that chooses the set when no flag does.
pub const ENV: &str = "CODING_CREW_ICONS";

/// The two glyph sets.
///
/// `Nerd` draws Nerd Fonts v3 glyphs, which need a patched font. `Unicode`
/// draws single-width characters a terminal font carries without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSet {
    Nerd,
    Unicode,
}

impl IconSet {
    /// Every set, in the order the spec names them.
    pub const ALL: [IconSet; 2] = [IconSet::Nerd, IconSet::Unicode];

    /// The name of the set, as the flag and the environment spell it.
    pub fn name(self) -> &'static str {
        match self {
            IconSet::Nerd => "nerd",
            IconSet::Unicode => "unicode",
        }
    }

    /// The set of a name.
    pub fn from_name(name: &str) -> Option<Self> {
        IconSet::ALL.into_iter().find(|set| set.name() == name)
    }

    /// The set in force: the flag's, then the environment's, then `nerd`.
    ///
    /// A name that is neither set is refused rather than replaced by the
    /// default: a typo would otherwise draw the wrong set without a word.
    pub fn resolve(flag: Option<&str>) -> Result<Self, String> {
        let env = env::var(ENV).ok();
        Self::choose(flag, env.as_deref())
    }

    /// The set the flag and the environment name, with the default last.
    /// An empty environment variable is unset.
    fn choose(flag: Option<&str>, env: Option<&str>) -> Result<Self, String> {
        match (flag, env) {
            (Some(name), _) => Self::named("--icons", name),
            (None, Some(name)) if !name.is_empty() => Self::named(ENV, name),
            _ => Ok(IconSet::Nerd),
        }
    }

    fn named(source: &str, name: &str) -> Result<Self, String> {
        Self::from_name(name).ok_or_else(|| {
            let names: Vec<&str> = IconSet::ALL.iter().map(|set| set.name()).collect();
            format!(
                "{source}: unknown icon set {name}: the sets are {}",
                names.join(", ")
            )
        })
    }
}

/// An icon of the spec's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// raul's title.
    AppRaul,
    /// hugo's title.
    AppHugo,
    /// The Overview tab.
    TabOverview,
    /// The Needs you tab.
    TabNeeds,
    /// The Path tab.
    TabPath,
    /// The Tasks tab.
    TabTasks,
    /// The Backlog tab.
    TabBacklog,
    /// The Settings tab.
    TabSettings,
    /// The draft phase.
    StatusDraft,
    /// The groomed phase.
    StatusGroomed,
    /// The approved phase.
    StatusApproved,
    /// The executing phase.
    StatusExecuting,
    /// The code-review phase.
    StatusReview,
    /// The remediating phase.
    StatusRemediating,
    /// The needs-human phase.
    StatusNeedsHuman,
    /// The done phase.
    StatusDone,
    /// An idea.
    KindIdea,
    /// A feature.
    KindFeature,
    /// A bug.
    KindBug,
    /// The orchestrator role.
    RoleOrchestrator,
    /// The runner, or builder, role.
    RoleRunner,
    /// The reviewer role.
    RoleReviewer,
    /// The human role.
    RoleHuman,
    /// Blocked, or waiting on another task.
    Blocked,
    /// An open finding.
    Finding,
    /// A dependency, a link.
    Link,
    /// A subtask's parent.
    Parent,
    /// An age, a hold, a stamp.
    Clock,
    /// The automation runs.
    Running,
    /// The automation stopped.
    Stopped,
    /// hugo's configuration.
    Config,
    /// The task that happens next: the Path's marker.
    Next,
    /// The selected row's bar.
    CursorBar,
    /// The value in use, in a settings group.
    InUse,
    /// An open task-list item.
    TaskOpen,
    /// A note alert.
    AlertNote,
    /// A tip alert.
    AlertTip,
    /// An important alert.
    AlertImportant,
    /// A warning alert.
    AlertWarning,
    /// A caution alert.
    AlertCaution,
    /// The mark before the active tab's label.
    TabMarkerLeft,
    /// The mark after the active tab's label.
    TabMarkerRight,
    /// The rule under the active tab's label.
    TabUnderline,
}

impl Icon {
    /// Every icon, in the order of the spec's table.
    pub const ALL: [Icon; 43] = [
        Icon::AppRaul,
        Icon::AppHugo,
        Icon::TabOverview,
        Icon::TabNeeds,
        Icon::TabPath,
        Icon::TabTasks,
        Icon::TabBacklog,
        Icon::TabSettings,
        Icon::StatusDraft,
        Icon::StatusGroomed,
        Icon::StatusApproved,
        Icon::StatusExecuting,
        Icon::StatusReview,
        Icon::StatusRemediating,
        Icon::StatusNeedsHuman,
        Icon::StatusDone,
        Icon::KindIdea,
        Icon::KindFeature,
        Icon::KindBug,
        Icon::RoleOrchestrator,
        Icon::RoleRunner,
        Icon::RoleReviewer,
        Icon::RoleHuman,
        Icon::Blocked,
        Icon::Finding,
        Icon::Link,
        Icon::Parent,
        Icon::Clock,
        Icon::Running,
        Icon::Stopped,
        Icon::Config,
        Icon::Next,
        Icon::CursorBar,
        Icon::InUse,
        Icon::TaskOpen,
        Icon::AlertNote,
        Icon::AlertTip,
        Icon::AlertImportant,
        Icon::AlertWarning,
        Icon::AlertCaution,
        Icon::TabMarkerLeft,
        Icon::TabMarkerRight,
        Icon::TabUnderline,
    ];

    /// The glyph of the icon in a set.
    pub fn glyph(self, set: IconSet) -> &'static str {
        let (_, _, nerd, unicode) = row(self);
        match set {
            IconSet::Nerd => nerd,
            IconSet::Unicode => unicode,
        }
    }

    /// The icon's name in the spec's table.
    pub fn name(self) -> &'static str {
        row(self).1
    }

    /// The icon of a phase, by its name: `draft`, `needs-human`, and so on.
    pub fn for_status(status: &str) -> Option<Self> {
        match status {
            "draft" => Some(Icon::StatusDraft),
            "groomed" => Some(Icon::StatusGroomed),
            "approved" => Some(Icon::StatusApproved),
            "executing" => Some(Icon::StatusExecuting),
            "code-review" => Some(Icon::StatusReview),
            "remediating" => Some(Icon::StatusRemediating),
            "needs-human" => Some(Icon::StatusNeedsHuman),
            "done" => Some(Icon::StatusDone),
            _ => None,
        }
    }

    /// The icon of a task kind, by its name: `idea`, `feature` or `bug`.
    pub fn for_kind(kind: &str) -> Option<Self> {
        match kind {
            "idea" => Some(Icon::KindIdea),
            "feature" => Some(Icon::KindFeature),
            "bug" => Some(Icon::KindBug),
            _ => None,
        }
    }
}

/// The icon table: each icon, its name in the spec, and its glyph in the
/// nerd and unicode sets. Its order is the spec's.
const TABLE: [(Icon, &str, &str, &str); 43] = [
    (Icon::AppRaul, "app_raul", "\u{F0645}", "◆"),
    (Icon::AppHugo, "app_hugo", "\u{F06A9}", "◇"),
    (Icon::TabOverview, "tab_overview", "\u{F056E}", "▦"),
    (Icon::TabNeeds, "tab_needs", "\u{F009A}", "◉"),
    (Icon::TabPath, "tab_path", "\u{F062C}", "⑂"),
    (Icon::TabTasks, "tab_tasks", "\u{F0AE7}", "☑"),
    (Icon::TabBacklog, "tab_backlog", "\u{F0687}", "≡"),
    (Icon::TabSettings, "tab_settings", "\u{F0493}", "⚙"),
    (Icon::StatusDraft, "status_draft", "\u{F0766}", "○"),
    (Icon::StatusGroomed, "status_groomed", "\u{F0AA5}", "◔"),
    (Icon::StatusApproved, "status_approved", "\u{F0132}", "◑"),
    (Icon::StatusExecuting, "status_executing", "\u{F040A}", "▶"),
    (Icon::StatusReview, "status_review", "\u{F0208}", "◎"),
    (
        Icon::StatusRemediating,
        "status_remediating",
        "\u{F05B7}",
        "↻",
    ),
    (
        Icon::StatusNeedsHuman,
        "status_needs_human",
        "\u{F0026}",
        "!",
    ),
    (Icon::StatusDone, "status_done", "\u{F012C}", "✓"),
    (Icon::KindIdea, "kind_idea", "\u{F0335}", "✦"),
    (Icon::KindFeature, "kind_feature", "\u{F0415}", "+"),
    (Icon::KindBug, "kind_bug", "\u{F00E4}", "✗"),
    (
        Icon::RoleOrchestrator,
        "role_orchestrator",
        "\u{F06A9}",
        "◇",
    ),
    (Icon::RoleRunner, "role_runner", "\u{F05B7}", "▸"),
    (Icon::RoleReviewer, "role_reviewer", "\u{F0208}", "◎"),
    (Icon::RoleHuman, "role_human", "\u{F0004}", "☺"),
    (Icon::Blocked, "blocked", "\u{F033E}", "⊘"),
    (Icon::Finding, "finding", "\u{F015A}", "✗"),
    (Icon::Link, "link", "\u{F0337}", "→"),
    (Icon::Parent, "parent", "\u{F005D}", "↑"),
    (Icon::Clock, "clock", "\u{F0954}", "◷"),
    (Icon::Running, "running", "\u{F046E}", "●"),
    (Icon::Stopped, "stopped", "\u{F03E4}", "■"),
    (Icon::Config, "config", "\u{F0493}", "⚙"),
    (Icon::Next, "next", "\u{F044A}", "●"),
    (Icon::CursorBar, "cursor_bar", "\u{258C}", "▌"),
    (Icon::InUse, "in_use", "\u{F043E}", "●"),
    (Icon::TaskOpen, "task_open", "\u{F0131}", "○"),
    (Icon::AlertNote, "alert_note", "\u{F02FC}", "ℹ"),
    (Icon::AlertTip, "alert_tip", "\u{F06E9}", "✦"),
    (Icon::AlertImportant, "alert_important", "\u{F0362}", "!"),
    (Icon::AlertWarning, "alert_warning", "\u{F0026}", "⚠"),
    (Icon::AlertCaution, "alert_caution", "\u{F015C}", "✗"),
    (Icon::TabMarkerLeft, "tab_marker_left", "\u{2039}", "‹"),
    (Icon::TabMarkerRight, "tab_marker_right", "\u{203A}", "›"),
    (Icon::TabUnderline, "tab_underline", "\u{2594}", "▔"),
];

/// The table's row of an icon.
fn row(icon: Icon) -> (Icon, &'static str, &'static str, &'static str) {
    TABLE
        .into_iter()
        .find(|(row, _, _, _)| *row == icon)
        .expect("every icon has a row in the table")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{first_grapheme, text_width};
    use coding_crew_core::Status;

    /// The redesign spec: the source of the names and the glyphs.
    const REDESIGN: &str = include_str!("../../../docs/tui/redesign.md");

    /// The rows of the spec's icon table: the name, and the glyph of each set.
    fn spec_rows() -> Vec<(String, String, String)> {
        let section = REDESIGN
            .split_once("## The icons")
            .expect("the spec has an icons section")
            .1
            .split_once("\n## ")
            .expect("the icons section ends")
            .0;
        section
            .lines()
            .filter(|line| line.starts_with("| `"))
            .map(|line| {
                let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
                let name = cells[0].trim().trim_matches('`').to_string();
                let nerd = cells[1].trim().split('`').nth(1).expect("a nerd glyph");
                let unicode = cells[2].trim().trim_matches('`');
                (name, nerd.to_string(), unicode.to_string())
            })
            .collect()
    }

    #[test]
    fn the_table_is_the_spec_table() {
        // The names and both glyphs of every row, against the spec's rows.
        // The codepoint notes in the spec are not compared: two of them name
        // a codepoint other than the glyph written beside them.
        let rows = spec_rows();
        assert_eq!(rows.len(), Icon::ALL.len(), "the spec's rows");
        for (name, nerd, unicode) in rows {
            let icon = Icon::ALL
                .into_iter()
                .find(|icon| icon.name() == name)
                .unwrap_or_else(|| panic!("the spec names {name} and the table does not"));
            assert_eq!(icon.glyph(IconSet::Nerd), nerd, "{name}");
            assert_eq!(icon.glyph(IconSet::Unicode), unicode, "{name}");
        }
    }

    #[test]
    fn every_icon_has_one_row() {
        assert_eq!(TABLE.len(), Icon::ALL.len());
        for icon in Icon::ALL {
            let rows = TABLE.iter().filter(|(row, _, _, _)| *row == icon).count();
            assert_eq!(rows, 1, "{icon:?}");
        }
    }

    #[test]
    fn every_glyph_is_one_column_in_both_sets() {
        // The width the sweep and the column table give a glyph: one column,
        // one grapheme, in both sets.
        for icon in Icon::ALL {
            for set in IconSet::ALL {
                let glyph = icon.glyph(set);
                assert_eq!(text_width(glyph), 1, "{icon:?} in {}", set.name());
                assert_eq!(first_grapheme(glyph), glyph, "{icon:?} in {}", set.name());
            }
        }
    }

    #[test]
    fn the_nerd_glyphs_are_private_use() {
        // A nerd glyph sits in a private-use plane, where no font draws it
        // without the patch; a unicode glyph never does. A glyph both sets
        // share, such as the cursor bar, needs no patch and is not nerd.
        for icon in Icon::ALL {
            let glyph = icon.glyph(IconSet::Nerd);
            if glyph == icon.glyph(IconSet::Unicode) {
                continue;
            }
            let character = glyph.chars().next().expect("a glyph");
            let private = ('\u{E000}'..='\u{F8FF}').contains(&character)
                || ('\u{F0000}'..='\u{FFFFD}').contains(&character);
            assert!(private, "{icon:?}: {glyph:?} is not a private-use glyph");
            let unicode = icon
                .glyph(IconSet::Unicode)
                .chars()
                .next()
                .expect("a glyph");
            assert!(!('\u{E000}'..='\u{F8FF}').contains(&unicode), "{icon:?}");
        }
    }

    #[test]
    fn the_set_names_are_the_flag_names() {
        assert_eq!(IconSet::from_name("nerd"), Some(IconSet::Nerd));
        assert_eq!(IconSet::from_name("unicode"), Some(IconSet::Unicode));
        assert_eq!(IconSet::from_name("emoji"), None);
        assert_eq!(IconSet::from_name("Nerd"), None);
        for set in IconSet::ALL {
            assert_eq!(IconSet::from_name(set.name()), Some(set));
        }
    }

    #[test]
    fn the_flag_chooses_the_set_over_the_environment() {
        assert_eq!(
            IconSet::choose(Some("unicode"), Some("nerd")),
            Ok(IconSet::Unicode)
        );
        assert_eq!(IconSet::choose(Some("nerd"), None), Ok(IconSet::Nerd));
    }

    #[test]
    fn the_environment_chooses_the_set_without_a_flag() {
        assert_eq!(IconSet::choose(None, Some("unicode")), Ok(IconSet::Unicode));
    }

    #[test]
    fn nerd_is_the_default() {
        assert_eq!(IconSet::choose(None, None), Ok(IconSet::Nerd));
        assert_eq!(IconSet::choose(None, Some("")), Ok(IconSet::Nerd));
    }

    #[test]
    fn an_unknown_set_is_refused_and_named() {
        let flag = IconSet::choose(Some("emoji"), Some("nerd")).expect_err("a flag typo");
        assert!(flag.contains("--icons") && flag.contains("emoji"), "{flag}");
        assert!(flag.contains("nerd, unicode"), "{flag}");
        let env = IconSet::choose(None, Some("emoji")).expect_err("an environment typo");
        assert!(env.contains(ENV) && env.contains("emoji"), "{env}");
    }

    #[test]
    fn a_status_has_its_phase_icon() {
        let phases = [
            (Status::Draft, Icon::StatusDraft),
            (Status::Groomed, Icon::StatusGroomed),
            (Status::Approved, Icon::StatusApproved),
            (Status::Executing, Icon::StatusExecuting),
            (Status::CodeReview, Icon::StatusReview),
            (Status::Remediating, Icon::StatusRemediating),
            (Status::NeedsHuman, Icon::StatusNeedsHuman),
            (Status::Done, Icon::StatusDone),
        ];
        for (status, icon) in phases {
            assert_eq!(Icon::for_status(status.as_str()), Some(icon));
        }
        assert_eq!(Icon::for_status("archived"), None);
    }

    #[test]
    fn a_kind_has_its_icon() {
        assert_eq!(Icon::for_kind("idea"), Some(Icon::KindIdea));
        assert_eq!(Icon::for_kind("feature"), Some(Icon::KindFeature));
        assert_eq!(Icon::for_kind("bug"), Some(Icon::KindBug));
        assert_eq!(Icon::for_kind("chore"), None);
    }
}
