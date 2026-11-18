//! The theme: the colours of raul.
//!
//! A slot names a role; a preset decides what the role looks like. The
//! domain knows nothing about this: `crates/core` never names a colour, and
//! a phase names a slot, never a colour.

use coding_lanes_core::Status;
use coding_lanes_engine::MarkdownTheme;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::data::Reason;

/// The role a colour plays.
///
/// The name of the slot is the name of the role, not of a colour: the same
/// slot is painted differently by every preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    /// The title line, the product name.
    ChromeTitle,
    /// The tab that is showing.
    ChromeTabActive,
    /// The other tabs.
    ChromeTabInactive,
    /// The `3/6` index.
    ChromeTabIndex,
    /// A block's border.
    ChromeBorder,
    /// The border of the focused block.
    ChromeBorderFocus,
    /// The footer's text.
    ChromeFooter,
    /// A key label in the footer and in the help overlay.
    ChromeKey,
    /// Ordinary text.
    TextNormal,
    /// Secondary text: the empty state, a marker, a hint under a line, a
    /// separator, a path, a tab that is not built yet.
    TextDim,
    /// A count, a title that matters.
    TextStrong,
    /// The selected row.
    TextSelected,
    /// A clean result, a finished task.
    SemanticOk,
    /// Something wants attention and is not broken.
    SemanticWarn,
    /// A failure: an unreadable file, a failed criterion, an escalation, no
    /// coding-lanes folder found.
    SemanticError,
    /// Neutral information.
    ///
    /// The colour carries no hue of its own: the spread between its
    /// highest and lowest channel is at most 48 of 255.
    SemanticInfo,
    /// The human lane: approve, review, and the facts that wait on it.
    SemanticHuman,
    /// The agent lane: claimable, running.
    SemanticAgent,
}

impl Slot {
    /// Every slot, in the order of the spec.
    ///
    /// The list and the names are the keys of the theme file format,
    /// which comes later: they are pinned by the tests, and nothing draws
    /// a slot by name yet.
    #[allow(dead_code)]
    pub const ALL: [Slot; 18] = [
        Slot::ChromeTitle,
        Slot::ChromeTabActive,
        Slot::ChromeTabInactive,
        Slot::ChromeTabIndex,
        Slot::ChromeBorder,
        Slot::ChromeBorderFocus,
        Slot::ChromeFooter,
        Slot::ChromeKey,
        Slot::TextNormal,
        Slot::TextDim,
        Slot::TextStrong,
        Slot::TextSelected,
        Slot::SemanticOk,
        Slot::SemanticWarn,
        Slot::SemanticError,
        Slot::SemanticInfo,
        Slot::SemanticHuman,
        Slot::SemanticAgent,
    ];

    /// The name of the slot, as the spec writes it.
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            Slot::ChromeTitle => "chrome.title",
            Slot::ChromeTabActive => "chrome.tab_active",
            Slot::ChromeTabInactive => "chrome.tab_inactive",
            Slot::ChromeTabIndex => "chrome.tab_index",
            Slot::ChromeBorder => "chrome.border",
            Slot::ChromeBorderFocus => "chrome.border_focus",
            Slot::ChromeFooter => "chrome.footer",
            Slot::ChromeKey => "chrome.key",
            Slot::TextNormal => "text.normal",
            Slot::TextDim => "text.dim",
            Slot::TextStrong => "text.strong",
            Slot::TextSelected => "text.selected",
            Slot::SemanticOk => "semantic.ok",
            Slot::SemanticWarn => "semantic.warn",
            Slot::SemanticError => "semantic.error",
            Slot::SemanticInfo => "semantic.info",
            Slot::SemanticHuman => "semantic.human",
            Slot::SemanticAgent => "semantic.agent",
        }
    }
}

/// The colours of the chrome: the frame around the content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chrome {
    pub title: Style,
    pub tab_active: Style,
    pub tab_inactive: Style,
    pub tab_index: Style,
    pub border: Style,
    pub border_focus: Style,
    pub footer: Style,
    pub key: Style,
}

/// The colours of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    pub normal: Style,
    pub dim: Style,
    pub strong: Style,
    pub selected: Style,
}

/// The colours that carry a meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Semantic {
    pub ok: Style,
    pub warn: Style,
    pub error: Style,
    pub info: Style,
    pub human: Style,
    pub agent: Style,
}

/// The colours of raul, one per slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub chrome: Chrome,
    pub text: Text,
    pub semantic: Semantic,
}

impl Theme {
    /// The theme of a preset.
    pub fn preset(preset: Preset) -> Self {
        let style = |slot: Slot| style_of(preset, slot);
        Theme {
            chrome: Chrome {
                title: style(Slot::ChromeTitle),
                tab_active: style(Slot::ChromeTabActive),
                tab_inactive: style(Slot::ChromeTabInactive),
                tab_index: style(Slot::ChromeTabIndex),
                border: style(Slot::ChromeBorder),
                border_focus: style(Slot::ChromeBorderFocus),
                footer: style(Slot::ChromeFooter),
                key: style(Slot::ChromeKey),
            },
            text: Text {
                normal: style(Slot::TextNormal),
                dim: style(Slot::TextDim),
                strong: style(Slot::TextStrong),
                selected: style(Slot::TextSelected),
            },
            semantic: Semantic {
                ok: style(Slot::SemanticOk),
                warn: style(Slot::SemanticWarn),
                error: style(Slot::SemanticError),
                info: style(Slot::SemanticInfo),
                human: style(Slot::SemanticHuman),
                agent: style(Slot::SemanticAgent),
            },
        }
    }

    /// The style of a slot.
    pub fn slot(&self, slot: Slot) -> Style {
        match slot {
            Slot::ChromeTitle => self.chrome.title,
            Slot::ChromeTabActive => self.chrome.tab_active,
            Slot::ChromeTabInactive => self.chrome.tab_inactive,
            Slot::ChromeTabIndex => self.chrome.tab_index,
            Slot::ChromeBorder => self.chrome.border,
            Slot::ChromeBorderFocus => self.chrome.border_focus,
            Slot::ChromeFooter => self.chrome.footer,
            Slot::ChromeKey => self.chrome.key,
            Slot::TextNormal => self.text.normal,
            Slot::TextDim => self.text.dim,
            Slot::TextStrong => self.text.strong,
            Slot::TextSelected => self.text.selected,
            Slot::SemanticOk => self.semantic.ok,
            Slot::SemanticWarn => self.semantic.warn,
            Slot::SemanticError => self.semantic.error,
            Slot::SemanticInfo => self.semantic.info,
            Slot::SemanticHuman => self.semantic.human,
            Slot::SemanticAgent => self.semantic.agent,
        }
    }

    /// The markdown engine's theme, mapped onto our slots.
    ///
    /// The engine has its own palettes; raul does not use them. Every field
    /// takes its slot's foreground, or the slot's background for a field that
    /// paints one. A slot with no colour in the preset becomes `Color::Reset`,
    /// so the terminal preset names no colour at all.
    ///
    /// The promise stops at `MarkdownTheme`: the engine paints code tokens
    /// with syntect's own theme, which this mapping does not reach. And
    /// `MarkdownTheme` carries colours only, so the terminal preset leaves
    /// every document field `Reset` and only the engine's own modifiers
    /// distinguish the roles.
    #[allow(dead_code)] // The detail pane's first caller.
    pub fn markdown_theme(&self) -> MarkdownTheme {
        let fg = |slot: Slot| self.slot(slot).fg.unwrap_or(Color::Reset);
        let bg = |slot: Slot| self.slot(slot).bg.unwrap_or(Color::Reset);
        MarkdownTheme {
            // A search hit takes the selection's background, but the engine
            // keeps the searched span's own foreground: the mapping cannot
            // make the hit readable, so search is not offered in the dark
            // document until a readable pairing exists.
            search_highlight_bg: bg(Slot::TextSelected),
            search_match_bg: bg(Slot::TextSelected),
            // Code: the gutter and the label are document words, so they
            // take the text grade; the frame is a border; inline code is
            // raised prose.
            code_gutter: fg(Slot::TextDim),
            code_frame: fg(Slot::ChromeBorder),
            code_label: fg(Slot::TextDim),
            inline_code_fg: fg(Slot::TextStrong),
            inline_code_bg: bg(Slot::ChromeBorder),
            // Quotes, rules, tables and blockquote markers are chrome.
            blockquote_marker: fg(Slot::ChromeBorder),
            blockquote_text: fg(Slot::TextDim),
            rule: fg(Slot::ChromeBorder),
            table_border: fg(Slot::ChromeBorder),
            table_separator: fg(Slot::ChromeBorder),
            table_header: fg(Slot::TextStrong),
            table_cell: fg(Slot::TextNormal),
            // Lists: a deliberate text hierarchy, one step dimmer each time,
            // but a document word never drops below the text grade.
            list_level_1: fg(Slot::TextNormal),
            list_level_2: fg(Slot::TextDim),
            list_level_3: fg(Slot::TextDim),
            ordered_list: fg(Slot::TextNormal),
            // Headings: the brightest title down to the dimmest step.
            heading_1: fg(Slot::ChromeTitle),
            heading_2: fg(Slot::TextStrong),
            heading_3: fg(Slot::TextNormal),
            heading_4: fg(Slot::TextDim),
            heading_other: fg(Slot::TextDim),
            heading_underline: fg(Slot::ChromeBorder),
            // Links are neutral information, never a lane.
            link_icon: fg(Slot::SemanticInfo),
            link_text: fg(Slot::SemanticInfo),
            link_hover: fg(Slot::SemanticInfo),
            // Ordinary and strong prose.
            text: fg(Slot::TextNormal),
            strong_text: fg(Slot::TextStrong),
            // LaTeX and Mermaid are prose with the code family's chrome.
            latex_inline_fg: fg(Slot::TextStrong),
            latex_inline_bg: bg(Slot::ChromeBorder),
            latex_block_fg: fg(Slot::TextStrong),
            mermaid_keyword: fg(Slot::TextStrong),
            mermaid_arrow: fg(Slot::ChromeBorder),
            mermaid_label: fg(Slot::TextNormal),
            mermaid_block_fg: fg(Slot::TextDim),
            // A mark is the selection: its background is the selection's
            // background and its foreground is the selection's own, the
            // pair the palette records.
            mark_fg: fg(Slot::TextSelected),
            mark_bg: bg(Slot::TextSelected),
            // Tasks.
            task_checked: fg(Slot::SemanticOk),
            task_unchecked: fg(Slot::TextDim),
            // Alerts, one semantic slot each. The warning and the failure
            // never share a slot; importance is information, not a lane.
            alert_note: fg(Slot::SemanticInfo),
            alert_tip: fg(Slot::SemanticOk),
            alert_important: fg(Slot::SemanticInfo),
            alert_warning: fg(Slot::SemanticWarn),
            alert_caution: fg(Slot::SemanticError),
            // Footnotes: the reference is information, the note is secondary.
            footnote_ref: fg(Slot::SemanticInfo),
            footnote_text: fg(Slot::TextDim),
        }
    }
}

/// The slot that paints a phase.
///
/// A phase never names a colour; it names a slot.
pub fn phase_slot(phase: Status) -> Slot {
    match phase {
        // Nothing has been said about it yet.
        Status::Draft => Slot::TextDim,
        // A human approves it.
        Status::Groomed => Slot::SemanticHuman,
        // An agent can take it.
        Status::Approved => Slot::SemanticAgent,
        // An agent is on it.
        Status::Executing => Slot::SemanticAgent,
        // A human reviews it.
        Status::CodeReview => Slot::SemanticHuman,
        // It is being fixed.
        Status::Remediating => Slot::SemanticWarn,
        // The review budget ran out; a person has to step in.
        Status::NeedsHuman => Slot::SemanticError,
        // It is finished.
        Status::Done => Slot::SemanticOk,
    }
}

/// The style of a phase in a theme.
pub fn phase_style(theme: &Theme, phase: Status) -> Style {
    theme.slot(phase_slot(phase))
}

/// The slot that paints a reason.
///
/// A reason is a phase in words: the human lane is approve and review, and
/// an escalation is a failure.
pub fn reason_slot(reason: Reason) -> Slot {
    match reason {
        // A human approves it.
        Reason::Approve => Slot::SemanticHuman,
        // A human reviews it.
        Reason::Review => Slot::SemanticHuman,
        // The review budget ran out; a person has to step in.
        Reason::Escalated => Slot::SemanticError,
    }
}

/// The style of a reason in a theme.
pub fn reason_style(theme: &Theme, reason: Reason) -> Style {
    theme.slot(reason_slot(reason))
}

/// The style of the title of a block.
///
/// The title of a block is `text.strong`: a title that matters. It sits on
/// the border, and the border's attributes bleed into it, so the dimming of
/// a dimmed border is taken away again: a title is never dimmed.
pub fn block_title_style(theme: &Theme) -> Style {
    theme.text.strong.remove_modifier(Modifier::DIM)
}

/// The line the cursor is on: every cell takes the selection's paint, and
/// the bar runs to the edge of the pane.
pub fn selected_line(line: Line<'static>, width: usize, theme: &Theme) -> Line<'static> {
    let used = line.width();
    let mut spans: Vec<Span<'static>> = line
        .spans
        .into_iter()
        .map(|span| Span::styled(span.content, theme.text.selected))
        .collect();
    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), theme.text.selected));
    }
    Line::from(spans)
}

/// The palettes raul ships with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// The dark palette. The default.
    Dark,
    /// The Nord palette: a dark one, from the family's own page.
    Nord,
    /// The Catppuccin palette: the family's mocha flavour.
    Catppuccin,
    /// The colours of the terminal itself.
    Terminal,
}

impl Preset {
    /// Every preset, in the order of the spec.
    pub const ALL: [Preset; 4] = [
        Preset::Dark,
        Preset::Nord,
        Preset::Catppuccin,
        Preset::Terminal,
    ];

    /// The name of the preset, as the flag writes it.
    pub fn name(&self) -> &'static str {
        match self {
            Preset::Dark => "dark",
            Preset::Nord => "nord",
            Preset::Catppuccin => "catppuccin",
            Preset::Terminal => "terminal",
        }
    }

    /// The preset of a name, if there is one.
    pub fn parse(name: &str) -> Option<Self> {
        Preset::ALL
            .iter()
            .copied()
            .find(|preset| preset.name() == name)
    }
}

/// The style of a slot in a preset.
///
/// The slot is looked up by the enum, never by its name: the name is for
/// the theme file, and a name that is spelled twice is a name that can
/// disagree with itself.
fn style_of(preset: Preset, slot: Slot) -> Style {
    match preset {
        Preset::Dark => dark_style(slot),
        Preset::Nord => nord_style(slot),
        Preset::Catppuccin => catppuccin_style(slot),
        Preset::Terminal => terminal_style(slot),
    }
}

/// The colours of the dark palette.
///
/// The palette is ours: the eighteen values are written down in
/// docs/tui/palettes.md, with the role each one plays. The background is
/// the terminal's, and the palette assumes a dark one.
mod dark {
    use ratatui::style::Color;

    /// The background the palette is drawn on.
    pub const BACKGROUND: Color = Color::Rgb(0x0d, 0x11, 0x17);
    /// The main foreground: ordinary text, the tab that is showing.
    pub const FOREGROUND: Color = Color::Rgb(0xc9, 0xd1, 0xd9);
    /// The second foreground: the tabs that are not showing.
    pub const SECOND: Color = Color::Rgb(0x9a, 0xa5, 0xb1);
    /// The muted foreground: secondary text.
    pub const MUTED: Color = Color::Rgb(0x8b, 0x94, 0x9e);
    /// The dimmed foreground: the footer, the index.
    pub const DIMMED: Color = Color::Rgb(0x7d, 0x85, 0x90);
    /// The raised foreground: a count, a title that matters.
    pub const RAISED: Color = Color::Rgb(0xe6, 0xed, 0xf3);
    /// The brightest foreground: the name of the tool.
    pub const BRIGHTEST: Color = Color::Rgb(0xff, 0xff, 0xff);
    /// The border colour.
    pub const BORDER: Color = Color::Rgb(0x3d, 0x44, 0x4d);
    /// The accent: the focus border, the selection, the human lane.
    pub const ACCENT: Color = Color::Rgb(0x79, 0xc0, 0xff);
    /// The second accent: the agent lane.
    pub const SECOND_ACCENT: Color = Color::Rgb(0xd2, 0xa8, 0xff);
    /// A clean result, a finished task.
    pub const OK: Color = Color::Rgb(0x7e, 0xc6, 0x99);
    /// Something wants attention and is not broken.
    pub const WARN: Color = Color::Rgb(0xff, 0x9e, 0x64);
    /// A failure.
    pub const ERROR: Color = Color::Rgb(0xe0, 0x6c, 0x75);
    /// Neutral information: a foreground without a hue of its own.
    pub const INFO: Color = Color::Rgb(0xb0, 0xb0, 0xb0);
    /// The attention colour: a key label.
    pub const ATTENTION: Color = Color::Rgb(0xff, 0xd1, 0x66);
}

/// The style of a slot in the dark palette.
///
/// The palette is ours: its eighteen values are written down in
/// docs/tui/palettes.md, with the role each one plays. It assumes a
/// terminal with a dark background.
fn dark_style(slot: Slot) -> Style {
    use dark::{
        ACCENT, ATTENTION, BACKGROUND, BORDER, BRIGHTEST, DIMMED, ERROR, FOREGROUND, INFO, MUTED,
        OK, RAISED, SECOND, SECOND_ACCENT, WARN,
    };
    match slot {
        // The brightest foreground: the name of the tool.
        Slot::ChromeTitle => Style::new().fg(BRIGHTEST).add_modifier(Modifier::BOLD),
        // The main foreground, raised: the tab that is showing.
        Slot::ChromeTabActive => Style::new().fg(FOREGROUND).add_modifier(Modifier::BOLD),
        // The second foreground: the other tabs.
        Slot::ChromeTabInactive => Style::new().fg(SECOND),
        // The dimmed foreground: the `3/6` index.
        Slot::ChromeTabIndex => Style::new().fg(DIMMED),
        // The border colour.
        Slot::ChromeBorder => Style::new().fg(BORDER),
        // The accent: the border of the focused block.
        Slot::ChromeBorderFocus => Style::new().fg(ACCENT),
        // The dimmed foreground: the footer's text.
        Slot::ChromeFooter => Style::new().fg(DIMMED),
        // The attention colour: a key label.
        Slot::ChromeKey => Style::new().fg(ATTENTION).add_modifier(Modifier::BOLD),
        // The main foreground: ordinary text.
        Slot::TextNormal => Style::new().fg(FOREGROUND),
        // The muted foreground: secondary text.
        Slot::TextDim => Style::new().fg(MUTED),
        // The raised foreground: a count, a title that matters.
        Slot::TextStrong => Style::new().fg(RAISED).add_modifier(Modifier::BOLD),
        // The selection is the accent, whole: a terminal has no alpha
        // compositing, so a translucent selection is not there to be had.
        Slot::TextSelected => Style::new().fg(BACKGROUND).bg(ACCENT),
        // A clean result, a finished task.
        Slot::SemanticOk => Style::new().fg(OK),
        // Something wants attention and is not broken.
        Slot::SemanticWarn => Style::new().fg(WARN),
        // A failure.
        Slot::SemanticError => Style::new().fg(ERROR),
        // Neutral information: it does not compete with the human lane.
        Slot::SemanticInfo => Style::new().fg(INFO),
        // The human lane takes the accent.
        Slot::SemanticHuman => Style::new().fg(ACCENT),
        // The agent lane takes the second accent.
        Slot::SemanticAgent => Style::new().fg(SECOND_ACCENT),
    }
}

/// The twelve Nord colours that the slots spend.
///
/// The family's palette has sixteen values in four groups: polar night
/// (nord0–3), snow storm (nord4–6), frost (nord7–10) and aurora (nord11–15).
/// The mapping in docs/tui/palettes.md says which role each of these twelve
/// plays, and why nord10 is kept for the record.
mod nord {
    use ratatui::style::Color;

    /// nord0: the base, and the selection's foreground.
    pub const NORD0: Color = Color::Rgb(0x2e, 0x34, 0x40);
    /// nord2: the dimmed chrome.
    pub const NORD2: Color = Color::Rgb(0x43, 0x4c, 0x5e);
    /// nord3: the border.
    pub const NORD3: Color = Color::Rgb(0x4c, 0x56, 0x6a);
    /// nord4: the muted text, and the footer's labels.
    pub const NORD4: Color = Color::Rgb(0xd8, 0xde, 0xe9);
    /// nord5: the main foreground.
    pub const NORD5: Color = Color::Rgb(0xe5, 0xe9, 0xf0);
    /// nord6: the raised and the brightest foreground.
    pub const NORD6: Color = Color::Rgb(0xec, 0xef, 0xf4);
    /// nord7: the second accent, the agent lane.
    pub const NORD7: Color = Color::Rgb(0x8f, 0xbc, 0xbb);
    /// nord8: the accent: the focus border, the selection, the human lane.
    pub const NORD8: Color = Color::Rgb(0x88, 0xc0, 0xd0);
    /// nord9: the second foreground, the frost.
    pub const NORD9: Color = Color::Rgb(0x81, 0xa1, 0xc1);
    /// nord11: the error colour.
    pub const NORD11: Color = Color::Rgb(0xbf, 0x61, 0x6a);
    /// nord13: the warning colour, and the attention colour.
    pub const NORD13: Color = Color::Rgb(0xeb, 0xcb, 0x8b);
    /// nord14: the success colour.
    pub const NORD14: Color = Color::Rgb(0xa3, 0xbe, 0x8c);
}

/// The style of a slot in the Nord palette.
///
/// The palette is the family's: its eighteen values are written down in
/// docs/tui/palettes.md, with the role each one plays. It is a dark preset:
/// it assumes a terminal with a dark background and paints none of its own.
fn nord_style(slot: Slot) -> Style {
    use nord::{
        NORD0, NORD2, NORD3, NORD4, NORD5, NORD6, NORD7, NORD8, NORD9, NORD11, NORD13, NORD14,
    };
    match slot {
        // The brightest foreground: the name of the tool.
        Slot::ChromeTitle => Style::new().fg(NORD6).add_modifier(Modifier::BOLD),
        // The main foreground: the tab that is showing.
        Slot::ChromeTabActive => Style::new().fg(NORD5).add_modifier(Modifier::BOLD),
        // The second foreground: the frost, the tabs that are not showing.
        Slot::ChromeTabInactive => Style::new().fg(NORD9),
        // The dimmed chrome: the `3/6` index.
        Slot::ChromeTabIndex => Style::new().fg(NORD2),
        // The border colour.
        Slot::ChromeBorder => Style::new().fg(NORD3),
        // The accent: the border of the focused block.
        Slot::ChromeBorderFocus => Style::new().fg(NORD8),
        // The footer's labels are words: the dimmed chrome is a border
        // grade, so they borrow the muted text.
        Slot::ChromeFooter => Style::new().fg(NORD4),
        // The attention colour: a key label.
        Slot::ChromeKey => Style::new().fg(NORD13).add_modifier(Modifier::BOLD),
        // The main foreground: ordinary text.
        Slot::TextNormal => Style::new().fg(NORD5),
        // The muted text: the dimmest snow storm grey.
        Slot::TextDim => Style::new().fg(NORD4),
        // The raised foreground: a count, a title that matters.
        Slot::TextStrong => Style::new().fg(NORD6).add_modifier(Modifier::BOLD),
        // The selection is the accent, whole: the base colour on it.
        Slot::TextSelected => Style::new().fg(NORD0).bg(NORD8),
        // A clean result, a finished task.
        Slot::SemanticOk => Style::new().fg(NORD14),
        // Something wants attention and is not broken.
        Slot::SemanticWarn => Style::new().fg(NORD13),
        // A failure: the family's own red.
        Slot::SemanticError => Style::new().fg(NORD11),
        // Neutral information: the snow storm greys all carry text, so the
        // information takes the main foreground.
        Slot::SemanticInfo => Style::new().fg(NORD5),
        // The human lane takes the accent.
        Slot::SemanticHuman => Style::new().fg(NORD8),
        // The agent lane takes the second accent.
        Slot::SemanticAgent => Style::new().fg(NORD7),
    }
}

/// The fifteen Catppuccin mocha values that the slots spend.
///
/// The family's page lists twenty-six values; the mapping in
/// docs/tui/palettes.md names which role each of these fifteen plays, and
/// why the rest are kept for the record. The flavour is mocha: the other
/// three wait with the light theme.
mod catppuccin {
    use ratatui::style::Color;

    /// base: the background the figures are measured against, and the
    /// selection's foreground.
    pub const BASE: Color = Color::Rgb(0x1e, 0x1e, 0x2e);
    /// rosewater: the raised and the brightest foreground.
    pub const ROSEWATER: Color = Color::Rgb(0xf5, 0xe0, 0xdc);
    /// text: the main foreground.
    pub const TEXT: Color = Color::Rgb(0xcd, 0xd6, 0xf4);
    /// subtext1: the muted foreground, and the footer's labels.
    pub const SUBTEXT1: Color = Color::Rgb(0xba, 0xc2, 0xde);
    /// subtext0: the neutral: the information colour.
    pub const SUBTEXT0: Color = Color::Rgb(0xa6, 0xad, 0xc8);
    /// overlay2: the second foreground, the tabs that are not showing.
    pub const OVERLAY2: Color = Color::Rgb(0x93, 0x99, 0xb2);
    /// overlay0: the dimmed chrome, the `3/6` index.
    pub const OVERLAY0: Color = Color::Rgb(0x6c, 0x70, 0x86);
    /// surface1: the border.
    pub const SURFACE1: Color = Color::Rgb(0x45, 0x47, 0x5a);
    /// mauve: the accent: the focus border, the selection, the human lane.
    pub const MAUVE: Color = Color::Rgb(0xcb, 0xa6, 0xf7);
    /// yellow: the attention colour.
    pub const YELLOW: Color = Color::Rgb(0xf9, 0xe2, 0xaf);
    /// green: the success colour.
    pub const GREEN: Color = Color::Rgb(0xa6, 0xe3, 0xa1);
    /// peach: the warning colour.
    pub const PEACH: Color = Color::Rgb(0xfa, 0xb3, 0x87);
    /// red: the error colour.
    pub const RED: Color = Color::Rgb(0xf3, 0x8b, 0xa8);
    /// teal: the second accent, the agent lane.
    pub const TEAL: Color = Color::Rgb(0x94, 0xe2, 0xd5);
}

/// The style of a slot in the Catppuccin palette.
///
/// The palette is the family's mocha flavour: its eighteen values are
/// written down in docs/tui/palettes.md, with the role each one plays. It
/// is a dark preset: it assumes a terminal with a dark background and
/// paints none of its own.
fn catppuccin_style(slot: Slot) -> Style {
    use catppuccin::{
        BASE, GREEN, MAUVE, OVERLAY0, OVERLAY2, PEACH, RED, ROSEWATER, SUBTEXT0, SUBTEXT1,
        SURFACE1, TEAL, TEXT, YELLOW,
    };
    match slot {
        // The brightest foreground: the name of the tool. The family names
        // no raised foreground, so the raised and the brightest borrow the
        // brightest colour it has.
        Slot::ChromeTitle => Style::new().fg(ROSEWATER).add_modifier(Modifier::BOLD),
        // The main foreground: the tab that is showing.
        Slot::ChromeTabActive => Style::new().fg(TEXT).add_modifier(Modifier::BOLD),
        // The second foreground: the brightest overlay, the tabs that are
        // not showing.
        Slot::ChromeTabInactive => Style::new().fg(OVERLAY2),
        // The dimmed chrome: the `3/6` index.
        Slot::ChromeTabIndex => Style::new().fg(OVERLAY0),
        // The border colour.
        Slot::ChromeBorder => Style::new().fg(SURFACE1),
        // The accent: the border of the focused block.
        Slot::ChromeBorderFocus => Style::new().fg(MAUVE),
        // The footer's labels are words: the dimmed chrome is a border
        // grade, so they borrow the muted foreground.
        Slot::ChromeFooter => Style::new().fg(SUBTEXT1),
        // The attention colour: a key label.
        Slot::ChromeKey => Style::new().fg(YELLOW).add_modifier(Modifier::BOLD),
        // The main foreground: ordinary text.
        Slot::TextNormal => Style::new().fg(TEXT),
        // The muted foreground: secondary text.
        Slot::TextDim => Style::new().fg(SUBTEXT1),
        // The raised foreground: a count, a title that matters.
        Slot::TextStrong => Style::new().fg(ROSEWATER).add_modifier(Modifier::BOLD),
        // The selection is the accent, whole: the base colour on it.
        Slot::TextSelected => Style::new().fg(BASE).bg(MAUVE),
        // A clean result, a finished task.
        Slot::SemanticOk => Style::new().fg(GREEN),
        // Something wants attention and is not broken.
        Slot::SemanticWarn => Style::new().fg(PEACH),
        // A failure: the family's own red.
        Slot::SemanticError => Style::new().fg(RED),
        // Neutral information: the word grey the other roles leave.
        Slot::SemanticInfo => Style::new().fg(SUBTEXT0),
        // The human lane takes the accent.
        Slot::SemanticHuman => Style::new().fg(MAUVE),
        // The agent lane takes the second accent.
        Slot::SemanticAgent => Style::new().fg(TEAL),
    }
}

/// The style of a slot in the colours of the terminal.
///
/// The colours are the terminal's; what is left are the attributes every
/// terminal has. A slot with no attribute is the terminal's own text.
///
/// The six semantic slots are pairwise distinct, and none of them paints
/// like ordinary text: the phases are told apart by the attributes alone.
fn terminal_style(slot: Slot) -> Style {
    match slot {
        Slot::ChromeTitle => Style::new().add_modifier(Modifier::BOLD),
        Slot::ChromeTabActive => Style::new()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::REVERSED),
        Slot::ChromeTabInactive => Style::new().add_modifier(Modifier::DIM),
        Slot::ChromeTabIndex => Style::new().add_modifier(Modifier::DIM),
        Slot::ChromeBorder => Style::new().add_modifier(Modifier::DIM),
        Slot::ChromeBorderFocus => Style::new().add_modifier(Modifier::BOLD),
        Slot::ChromeFooter => Style::new().add_modifier(Modifier::DIM),
        Slot::ChromeKey => Style::new()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::REVERSED),
        Slot::TextNormal => Style::new(),
        Slot::TextDim => Style::new().add_modifier(Modifier::DIM),
        Slot::TextStrong => Style::new().add_modifier(Modifier::BOLD),
        Slot::TextSelected => Style::new().add_modifier(Modifier::REVERSED),
        // The six semantic slots are pairwise distinct, and none of them
        // paints like ordinary text: the phases are told apart by the
        // attributes alone.
        Slot::SemanticOk => Style::new().add_modifier(Modifier::ITALIC),
        Slot::SemanticWarn => Style::new().add_modifier(Modifier::UNDERLINED),
        Slot::SemanticError => Style::new()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::REVERSED)
            .add_modifier(Modifier::UNDERLINED),
        Slot::SemanticInfo => Style::new()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::DIM),
        Slot::SemanticHuman => Style::new()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED),
        Slot::SemanticAgent => Style::new()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::ITALIC),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    /// The palettes doc: the source of truth for the dark palette.
    const PALETTES: &str = include_str!("../../../docs/tui/palettes.md");

    /// The theme doc: the source of truth for the engine mapping.
    const THEME: &str = include_str!("../../../docs/tui/theme.md");

    /// The engine's markdown sources: the authority for which fields it
    /// paints with a background.
    const ENGINE_SOURCES: [&str; 18] = [
        include_str!("../../../crates/engine/src/markdown/blocks.rs"),
        include_str!("../../../crates/engine/src/markdown/fences.rs"),
        include_str!("../../../crates/engine/src/markdown/footnotes.rs"),
        include_str!("../../../crates/engine/src/markdown/frontmatter.rs"),
        include_str!("../../../crates/engine/src/markdown/highlight.rs"),
        include_str!("../../../crates/engine/src/markdown/latex.rs"),
        include_str!("../../../crates/engine/src/markdown/links.rs"),
        include_str!("../../../crates/engine/src/markdown/lists.rs"),
        include_str!("../../../crates/engine/src/markdown/markers.rs"),
        include_str!("../../../crates/engine/src/markdown/mermaid.rs"),
        include_str!("../../../crates/engine/src/markdown/mod.rs"),
        include_str!("../../../crates/engine/src/markdown/spans.rs"),
        include_str!("../../../crates/engine/src/markdown/syntax.rs"),
        include_str!("../../../crates/engine/src/markdown/table_layout.rs"),
        include_str!("../../../crates/engine/src/markdown/tables.rs"),
        include_str!("../../../crates/engine/src/markdown/toc.rs"),
        include_str!("../../../crates/engine/src/markdown/width.rs"),
        include_str!("../../../crates/engine/src/markdown/wrapping.rs"),
    ];

    /// The tolerance of the neutral, in channels of 255.
    ///
    /// The doc states it; the test below holds the two together.
    const NEUTRAL_TOLERANCE: u8 = 48;

    /// The engine's theme fields, the slot each maps to, and whether the
    /// field paints a background.
    ///
    /// The engine exposes no reflection, so this list is the coverage of the
    /// 47 fields: a field added upstream has to be added here and in the doc.
    const ENGINE_SLOTS: [(&str, Slot, bool); 47] = [
        ("search_highlight_bg", Slot::TextSelected, true),
        ("search_match_bg", Slot::TextSelected, true),
        ("code_gutter", Slot::TextDim, false),
        ("blockquote_marker", Slot::ChromeBorder, false),
        ("list_level_1", Slot::TextNormal, false),
        ("list_level_2", Slot::TextDim, false),
        ("list_level_3", Slot::TextDim, false),
        ("ordered_list", Slot::TextNormal, false),
        ("table_border", Slot::ChromeBorder, false),
        ("table_separator", Slot::ChromeBorder, false),
        ("table_header", Slot::TextStrong, false),
        ("table_cell", Slot::TextNormal, false),
        ("heading_1", Slot::ChromeTitle, false),
        ("heading_2", Slot::TextStrong, false),
        ("heading_3", Slot::TextNormal, false),
        ("heading_4", Slot::TextDim, false),
        ("heading_other", Slot::TextDim, false),
        ("heading_underline", Slot::ChromeBorder, false),
        ("code_frame", Slot::ChromeBorder, false),
        ("code_label", Slot::TextDim, false),
        ("inline_code_fg", Slot::TextStrong, false),
        ("inline_code_bg", Slot::ChromeBorder, true),
        ("rule", Slot::ChromeBorder, false),
        ("link_icon", Slot::SemanticInfo, false),
        ("link_text", Slot::SemanticInfo, false),
        ("link_hover", Slot::SemanticInfo, false),
        ("blockquote_text", Slot::TextDim, false),
        ("text", Slot::TextNormal, false),
        ("strong_text", Slot::TextStrong, false),
        ("latex_inline_fg", Slot::TextStrong, false),
        ("latex_inline_bg", Slot::ChromeBorder, true),
        ("latex_block_fg", Slot::TextStrong, false),
        ("mermaid_keyword", Slot::TextStrong, false),
        ("mermaid_arrow", Slot::ChromeBorder, false),
        ("mermaid_label", Slot::TextNormal, false),
        ("mermaid_block_fg", Slot::TextDim, false),
        ("mark_fg", Slot::TextSelected, false),
        ("mark_bg", Slot::TextSelected, true),
        ("task_checked", Slot::SemanticOk, false),
        ("task_unchecked", Slot::TextDim, false),
        ("alert_note", Slot::SemanticInfo, false),
        ("alert_tip", Slot::SemanticOk, false),
        ("alert_important", Slot::SemanticInfo, false),
        ("alert_warning", Slot::SemanticWarn, false),
        ("alert_caution", Slot::SemanticError, false),
        ("footnote_ref", Slot::SemanticInfo, false),
        ("footnote_text", Slot::TextDim, false),
    ];

    /// The engine fields it consumes with `.bg()`.
    ///
    /// The vendored engine paints these five with a background, and every
    /// other field with a foreground: `markers.rs` and `spans.rs` pair
    /// `mark_fg` with `mark_bg` and `inline_code_fg`/`latex_inline_fg` with
    /// their `_bg`, `highlight.rs` paints `search_highlight_bg` and
    /// `search_match_bg`, and `table_layout.rs` paints the same three pairs.
    /// The test below derives the set from the engine's own sources, so it
    /// is a fact and not a copy of the mapping's assumption.
    const ENGINE_BACKGROUND_FIELDS: [&str; 5] = [
        "search_highlight_bg",
        "search_match_bg",
        "inline_code_bg",
        "latex_inline_bg",
        "mark_bg",
    ];

    /// Every field of an engine theme, with the flag that says whether it
    /// paints a background.
    fn engine_fields(theme: &MarkdownTheme) -> [(&'static str, Color, bool); 47] {
        [
            ("search_highlight_bg", theme.search_highlight_bg, true),
            ("search_match_bg", theme.search_match_bg, true),
            ("code_gutter", theme.code_gutter, false),
            ("blockquote_marker", theme.blockquote_marker, false),
            ("list_level_1", theme.list_level_1, false),
            ("list_level_2", theme.list_level_2, false),
            ("list_level_3", theme.list_level_3, false),
            ("ordered_list", theme.ordered_list, false),
            ("table_border", theme.table_border, false),
            ("table_separator", theme.table_separator, false),
            ("table_header", theme.table_header, false),
            ("table_cell", theme.table_cell, false),
            ("heading_1", theme.heading_1, false),
            ("heading_2", theme.heading_2, false),
            ("heading_3", theme.heading_3, false),
            ("heading_4", theme.heading_4, false),
            ("heading_other", theme.heading_other, false),
            ("heading_underline", theme.heading_underline, false),
            ("code_frame", theme.code_frame, false),
            ("code_label", theme.code_label, false),
            ("inline_code_fg", theme.inline_code_fg, false),
            ("inline_code_bg", theme.inline_code_bg, true),
            ("rule", theme.rule, false),
            ("link_icon", theme.link_icon, false),
            ("link_text", theme.link_text, false),
            ("link_hover", theme.link_hover, false),
            ("blockquote_text", theme.blockquote_text, false),
            ("text", theme.text, false),
            ("strong_text", theme.strong_text, false),
            ("latex_inline_fg", theme.latex_inline_fg, false),
            ("latex_inline_bg", theme.latex_inline_bg, true),
            ("latex_block_fg", theme.latex_block_fg, false),
            ("mermaid_keyword", theme.mermaid_keyword, false),
            ("mermaid_arrow", theme.mermaid_arrow, false),
            ("mermaid_label", theme.mermaid_label, false),
            ("mermaid_block_fg", theme.mermaid_block_fg, false),
            ("mark_fg", theme.mark_fg, false),
            ("mark_bg", theme.mark_bg, true),
            ("task_checked", theme.task_checked, false),
            ("task_unchecked", theme.task_unchecked, false),
            ("alert_note", theme.alert_note, false),
            ("alert_tip", theme.alert_tip, false),
            ("alert_important", theme.alert_important, false),
            ("alert_warning", theme.alert_warning, false),
            ("alert_caution", theme.alert_caution, false),
            ("footnote_ref", theme.footnote_ref, false),
            ("footnote_text", theme.footnote_text, false),
        ]
    }

    /// Every phase of the lifecycle and the slot the spec gives it.
    fn phases() -> [(Status, Slot); 8] {
        [
            (Status::Draft, Slot::TextDim),
            (Status::Groomed, Slot::SemanticHuman),
            (Status::Approved, Slot::SemanticAgent),
            (Status::Executing, Slot::SemanticAgent),
            (Status::CodeReview, Slot::SemanticHuman),
            (Status::Remediating, Slot::SemanticWarn),
            (Status::NeedsHuman, Slot::SemanticError),
            (Status::Done, Slot::SemanticOk),
        ]
    }

    #[test]
    fn every_phase_maps_to_a_slot() {
        for (phase, slot) in phases() {
            assert_eq!(phase_slot(phase), slot, "{phase:?} maps elsewhere");
        }
    }

    #[test]
    fn every_slot_is_named() {
        // The names are the keys of the theme file format, so they are
        // pinned here: a typo in one of them is a slot that cannot be set
        // from a theme file.
        let names: Vec<&str> = Slot::ALL.iter().map(|slot| slot.name()).collect();
        assert_eq!(
            names,
            vec![
                "chrome.title",
                "chrome.tab_active",
                "chrome.tab_inactive",
                "chrome.tab_index",
                "chrome.border",
                "chrome.border_focus",
                "chrome.footer",
                "chrome.key",
                "text.normal",
                "text.dim",
                "text.strong",
                "text.selected",
                "semantic.ok",
                "semantic.warn",
                "semantic.error",
                "semantic.info",
                "semantic.human",
                "semantic.agent",
            ]
        );
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), Slot::ALL.len(), "two slots share a name");
        for slot in Slot::ALL {
            assert!(slot.name().contains('.'), "{} is not named", slot.name());
        }
    }

    #[test]
    fn every_reason_maps_to_a_slot() {
        assert_eq!(reason_slot(Reason::Approve), Slot::SemanticHuman);
        assert_eq!(reason_slot(Reason::Review), Slot::SemanticHuman);
        assert_eq!(reason_slot(Reason::Escalated), Slot::SemanticError);
    }

    #[test]
    fn the_semantic_slots_of_the_terminal_preset_are_told_apart() {
        let theme = Theme::preset(Preset::Terminal);
        let semantic = [
            Slot::SemanticOk,
            Slot::SemanticWarn,
            Slot::SemanticError,
            Slot::SemanticInfo,
            Slot::SemanticHuman,
            Slot::SemanticAgent,
        ];
        let normal = theme.slot(Slot::TextNormal);
        for slot in semantic {
            let style = theme.slot(slot);
            assert_ne!(style, normal, "{} paints like ordinary text", slot.name());
            for other in semantic {
                if other == slot {
                    continue;
                }
                assert_ne!(
                    style,
                    theme.slot(other),
                    "{} and {} paint alike",
                    slot.name(),
                    other.name()
                );
            }
        }
    }

    #[test]
    fn the_human_and_the_agent_lane_are_different_colours() {
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            assert_ne!(
                theme.slot(Slot::SemanticHuman),
                theme.slot(Slot::SemanticAgent),
                "{} paints the two lanes alike",
                preset.name()
            );
        }
    }

    #[test]
    fn every_phase_looks_different_from_every_other_phase() {
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            for (phase, slot) in phases() {
                for (other, other_slot) in phases() {
                    if phase == other || slot == other_slot {
                        // Two phases may share a slot: the spec says so.
                        continue;
                    }
                    assert_ne!(
                        theme.slot(slot),
                        theme.slot(other_slot),
                        "{} paints {phase:?} and {other:?} alike",
                        preset.name()
                    );
                }
            }
        }
    }

    #[test]
    fn every_slot_is_painted_in_every_colour_preset() {
        // The terminal preset paints by attribute, and its ordinary text is
        // the terminal's own: only the colour presets paint all eighteen.
        for preset in Preset::ALL {
            if preset == Preset::Terminal {
                continue;
            }
            let theme = Theme::preset(preset);
            for slot in Slot::ALL {
                // A slot that paints nothing is a slot that was forgotten.
                let style = theme.slot(slot);
                assert!(
                    style.fg.is_some() || style.bg.is_some() || !style.add_modifier.is_empty(),
                    "{} paints nothing in the {} preset",
                    slot.name(),
                    preset.name()
                );
            }
        }
    }

    /// The relative luminance of a colour, by the WCAG formula.
    fn luminance(color: Color) -> f64 {
        let Color::Rgb(r, g, b) = color else {
            return 0.0;
        };
        let channel = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }

    /// The contrast ratio of two colours, from 1 to 21.
    fn contrast(one: Color, other: Color) -> f64 {
        let (one, other) = (luminance(one), luminance(other));
        let (light, dark) = if one > other {
            (one, other)
        } else {
            (other, one)
        };
        (light + 0.05) / (dark + 0.05)
    }

    #[test]
    fn the_dark_preset_reads_on_its_background() {
        let theme = Theme::preset(Preset::Dark);
        let background = Color::Rgb(0x0d, 0x11, 0x17);
        for slot in Slot::ALL {
            let style = theme.slot(slot);
            let Some(foreground) = style.fg else {
                continue;
            };
            // The border is a line, not a word: it is allowed to be faint.
            if slot == Slot::ChromeBorder {
                continue;
            }
            // The words are read on whatever is behind them.
            let behind = style.bg.unwrap_or(background);
            let ratio = contrast(foreground, behind);
            assert!(
                ratio >= 4.5,
                "{} reads at {ratio:.1}:1, and a word needs 4.5:1",
                slot.name()
            );
        }
    }

    #[test]
    fn the_information_colour_is_neutral() {
        // Information is not a lane: it takes no hue of its own, so it
        // does not compete with the human lane for attention.
        //
        // The test is the one the doc states: the spread between the
        // three channels, within a tolerance. A neutral need not have
        // three equal channels.
        for preset in Preset::ALL {
            if preset == Preset::Terminal {
                continue;
            }
            let theme = Theme::preset(preset);
            let Color::Rgb(r, g, b) = theme.semantic.info.fg.expect("info has a colour") else {
                panic!("info is not a colour of the {} preset", preset.name());
            };
            let apart = r.max(g).max(b) - r.min(g).min(b);
            assert!(
                apart <= NEUTRAL_TOLERANCE,
                "{}'s info is not neutral: its channels are {apart} apart, and the tolerance is {NEUTRAL_TOLERANCE}",
                preset.name()
            );
        }
    }

    #[test]
    fn the_tolerance_of_the_neutral_is_the_one_the_doc_states() {
        // The doc states the tolerance; the code holds the same one.
        assert!(
            PALETTES.contains(&format!("at most {NEUTRAL_TOLERANCE} of 255")),
            "the doc does not state the tolerance of {NEUTRAL_TOLERANCE} the code holds"
        );
    }

    /// The spread between the three channels of a colour.
    fn spread(color: Color) -> u8 {
        let Color::Rgb(r, g, b) = color else {
            return 255;
        };
        r.max(g).max(b) - r.min(g).min(b)
    }

    #[test]
    fn the_neutrals_of_the_families_are_neutral() {
        // Each family table names the neutral it derives; every one of
        // them must pass the test the code holds.
        let mut checked = 0;
        for line in PALETTES.lines() {
            if !line.contains("**the neutral**") {
                continue;
            }
            for hex in hexes_in(line) {
                assert!(
                    spread(hex) <= NEUTRAL_TOLERANCE,
                    "{hex:?} is named a neutral, and it is {0} apart",
                    spread(hex)
                );
                checked += 1;
            }
        }
        assert!(checked >= 3, "no family names a neutral: {checked} found");
    }

    /// The colours a line of the doc names, in the order it names them.
    fn hexes_in(line: &str) -> Vec<Color> {
        let mut colours = Vec::new();
        let mut rest = line;
        while let Some(start) = rest.find('#') {
            rest = &rest[start + 1..];
            if rest.len() < 6 {
                break;
            }
            let Some(end) = rest.find(|c: char| !c.is_ascii_hexdigit()) else {
                break;
            };
            if end != 6 {
                rest = &rest[end..];
                continue;
            }
            let digits = &rest[..6];
            let channel =
                |index: usize| u8::from_str_radix(&digits[index..index + 2], 16).expect("hex");
            colours.push(Color::Rgb(channel(0), channel(2), channel(4)));
            rest = &rest[6..];
        }
        colours
    }

    /// The rows of the dark palette table of the doc: the slot, the
    /// foreground, the background and whether it is bold.
    fn dark_table() -> Vec<(String, Option<Color>, Option<Color>, bool)> {
        let section = PALETTES
            .split_once("## Ours: the dark palette")
            .expect("the doc has a dark palette section")
            .1;
        let section = section
            .split_once("\n## ")
            .map_or(section, |(head, _)| head);
        let mut rows = Vec::new();
        for line in section.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // | slot | role | foreground | background | attributes |
            if cells.len() < 6 {
                continue;
            }
            let slot = cells[1].trim_matches('`');
            if !slot.contains('.') {
                continue;
            }
            let foreground = hexes_in(cells[3]).first().copied();
            let background = hexes_in(cells[4]).first().copied();
            let bold = cells[5] == "bold";
            rows.push((slot.to_string(), foreground, background, bold));
        }
        rows
    }

    #[test]
    fn the_dark_palette_is_the_one_the_doc_records() {
        // The doc is the source of truth: the code is checked against
        // its table, so the two cannot drift apart silently.
        let theme = Theme::preset(Preset::Dark);
        let rows = dark_table();
        assert_eq!(
            rows.len(),
            Slot::ALL.len(),
            "the doc's dark table does not hold the eighteen slots"
        );
        for (slot, foreground, background, bold) in rows {
            let style = theme.slot(
                *Slot::ALL
                    .iter()
                    .find(|known| known.name() == slot)
                    .unwrap_or_else(|| panic!("{slot} is not a slot")),
            );
            assert_eq!(
                style.fg, foreground,
                "{slot} paints a foreground the doc does not record"
            );
            assert_eq!(
                style.bg, background,
                "{slot} paints a background the doc does not record"
            );
            assert_eq!(
                style.add_modifier.contains(Modifier::BOLD),
                bold,
                "{slot} is bold in one place and not in the other"
            );
        }
    }

    /// The families, and the heading that names each of them.
    const FAMILIES: [&str; 3] = ["Catppuccin", "Dracula", "Nord"];

    /// The text of a section of the doc.
    fn section(doc: &str, heading: &str) -> String {
        let rest = doc
            .split_once(&format!("## {heading}"))
            .unwrap_or_else(|| panic!("the doc has no {heading} section"))
            .1;
        rest.split_once("\n## ")
            .map_or(rest, |(head, _)| head)
            .to_string()
    }

    /// The rows of a family's tables that name a slot: the slot and the
    /// cells after it — the role first, then the values or the ratios.
    fn family_rows(section: &str) -> Vec<(String, Vec<String>)> {
        let mut rows = Vec::new();
        for line in section.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if cells.len() < 4 || !cells[1].starts_with('`') {
                continue;
            }
            let slot = cells[1].trim_matches('`');
            if !slot.contains('.') {
                continue;
            }
            rows.push((
                slot.to_string(),
                cells[2..].iter().map(|cell| cell.to_string()).collect(),
            ));
        }
        rows
    }

    /// The values of a family's slots: the slot and the colour of each
    /// flavour.
    fn family_values(section: &str) -> Vec<(String, Vec<Color>)> {
        family_rows(section)
            .into_iter()
            .filter_map(|(slot, cells)| {
                let values: Vec<Color> = cells[1..]
                    .iter()
                    .filter_map(|cell| hexes_in(cell).first().copied())
                    .collect();
                (!values.is_empty()).then_some((slot, values))
            })
            .collect()
    }

    /// The contrast figures of a family's slots: the slot and the ratio of
    /// each flavour.
    fn family_ratios(section: &str) -> Vec<(String, Vec<f64>)> {
        family_rows(section)
            .into_iter()
            .filter_map(|(slot, cells)| {
                let ratios: Vec<f64> = cells.iter().filter_map(|cell| ratio_in(cell)).collect();
                (!ratios.is_empty()).then_some((slot, ratios))
            })
            .collect()
    }

    /// The ratio a cell states, if it states one.
    fn ratio_in(cell: &str) -> Option<f64> {
        let (number, _) = cell.split_once(":1")?;
        number.trim().parse().ok()
    }

    /// The base of a family: the background its figures are measured
    /// against, one for each flavour.
    fn base_of(section: &str) -> Vec<Color> {
        let line = section
            .lines()
            .find(|line| line.starts_with("The base is"))
            .unwrap_or_else(|| panic!("the section does not name a base"));
        let base = hexes_in(line);
        assert!(!base.is_empty(), "the base names no colour");
        base
    }

    /// The slots that hold a word. The chrome that draws lines is not one.
    fn is_a_word(slot: &str) -> bool {
        !matches!(slot, "chrome.border" | "chrome.tab_index")
    }

    #[test]
    fn every_family_table_names_every_slot() {
        // The mapping table promises every theme the same eighteen slots,
        // and a family that leaves one of them out has no value for it.
        for heading in FAMILIES {
            let section = section(PALETTES, heading);
            let named: Vec<String> = family_values(&section)
                .into_iter()
                .map(|(slot, _)| slot)
                .collect();
            for slot in Slot::ALL {
                assert!(
                    named.contains(&slot.name().to_string()),
                    "{heading} has no {} row",
                    slot.name()
                );
            }
            assert_eq!(
                named.len(),
                Slot::ALL.len(),
                "{heading} names {} rows, and the mapping promises {}",
                named.len(),
                Slot::ALL.len()
            );
        }
    }

    #[test]
    fn the_words_of_the_families_read_on_their_base() {
        // The rule the doc states: a dimmed word reads at 3:1 or better on
        // the background it sits on.
        assert!(
            PALETTES.contains("at 3:1 or better"),
            "the doc does not state the floor the code holds"
        );
        for heading in FAMILIES {
            let section = section(PALETTES, heading);
            let base = base_of(&section);
            for (slot, values) in family_values(&section) {
                if !is_a_word(&slot) {
                    continue;
                }
                assert_eq!(
                    values.len(),
                    base.len(),
                    "{heading} paints {slot} in {} flavours and has {} bases",
                    values.len(),
                    base.len()
                );
                for (colour, base) in values.iter().zip(base.iter()) {
                    let ratio = contrast(*colour, *base);
                    assert!(
                        ratio >= 3.0,
                        "{heading}'s {slot} reads at {ratio:.1}:1, and a word needs 3:1"
                    );
                }
            }
        }
    }

    #[test]
    fn the_contrast_figures_of_the_families_are_right() {
        // Every figure the doc states is recomputed from the values the
        // same table states.
        for heading in FAMILIES {
            let section = section(PALETTES, heading);
            let base = base_of(&section);
            let values = family_values(&section);
            for (slot, ratios) in family_ratios(&section) {
                let colours = values
                    .iter()
                    .find(|(name, _)| name == &slot)
                    .map(|(_, colours)| colours)
                    .unwrap_or_else(|| panic!("{heading} states a ratio for {slot} and no value"));
                assert_eq!(
                    ratios.len(),
                    colours.len(),
                    "{heading} states {} ratios for {slot} and {} values",
                    ratios.len(),
                    colours.len()
                );
                for (ratio, (colour, base)) in ratios.iter().zip(colours.iter().zip(base.iter())) {
                    let computed = contrast(*colour, *base);
                    assert!(
                        (computed - ratio).abs() < 0.05,
                        "{heading}'s {slot} reads at {computed:.2}:1, and the doc says {ratio}:1"
                    );
                }
            }
        }
    }

    #[test]
    fn the_contrast_figures_of_the_dark_palette_are_right() {
        // The same, for our own palette.
        let section = section(PALETTES, "Ours: the dark palette");
        let mut checked = 0;
        for line in section.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            for pair in cells.windows(2) {
                let Some(colour) = hexes_in(pair[0]).first().copied() else {
                    continue;
                };
                let Some(ratio) = ratio_in(pair[1]) else {
                    continue;
                };
                let computed = contrast(colour, dark::BACKGROUND);
                assert!(
                    (computed - ratio).abs() < 0.05,
                    "{} reads at {computed:.2}:1, and the doc says {ratio}:1",
                    pair[0]
                );
                checked += 1;
            }
        }
        assert!(checked >= 14, "the dark palette states {checked} figures");
    }

    /// The colour rows of the Nord table: the slot, the family's role, and
    /// the value.
    fn nord_rows() -> Vec<(String, String, Color)> {
        let section = section(PALETTES, "Nord");
        family_rows(&section)
            .into_iter()
            .filter_map(|(slot, cells)| {
                let colour = cells
                    .iter()
                    .find_map(|cell| hexes_in(cell).first().copied())?;
                Some((slot, cells[0].clone(), colour))
            })
            .collect()
    }

    /// The colour of a Nord role named by the doc, like `nord6`.
    fn nord_colour(role: &str) -> Color {
        match role {
            "nord0" => nord::NORD0,
            "nord2" => nord::NORD2,
            "nord3" => nord::NORD3,
            "nord4" => nord::NORD4,
            "nord5" => nord::NORD5,
            "nord6" => nord::NORD6,
            "nord7" => nord::NORD7,
            "nord8" => nord::NORD8,
            "nord9" => nord::NORD9,
            "nord11" => nord::NORD11,
            "nord13" => nord::NORD13,
            "nord14" => nord::NORD14,
            other => panic!("{other} is not a Nord colour the slots spend"),
        }
    }

    /// The slot a name the doc writes names.
    fn slot_named(name: &str) -> Slot {
        *Slot::ALL
            .iter()
            .find(|slot| slot.name() == name)
            .unwrap_or_else(|| panic!("{name} is not a slot"))
    }

    #[test]
    fn the_nord_palette_is_the_one_the_doc_records() {
        // The doc is the source of truth: the code is checked against its
        // table, so the two cannot drift apart silently.
        let theme = Theme::preset(Preset::Nord);
        let base = base_of(&section(PALETTES, "Nord"))[0];
        let rows = nord_rows();
        assert_eq!(
            rows.len(),
            Slot::ALL.len(),
            "the doc's Nord table does not hold the eighteen slots"
        );
        for (name, _, colour) in rows {
            let slot = slot_named(&name);
            let style = theme.slot(slot);
            if slot == Slot::TextSelected {
                // The doc records the accent; the selection puts the base
                // on it, the pair the palette records.
                assert_eq!(
                    style.bg,
                    Some(colour),
                    "{name} paints a background the doc does not record"
                );
                assert_eq!(
                    style.fg,
                    Some(base),
                    "{name} does not put the base on the accent"
                );
            } else {
                assert_eq!(
                    style.fg,
                    Some(colour),
                    "{name} paints a foreground the doc does not record"
                );
                assert_eq!(
                    style.bg, None,
                    "{name} paints a background the doc does not record"
                );
            }
        }
    }

    #[test]
    fn the_nord_roles_name_the_values_the_table_states() {
        // The value beside a role is the value the page gives that role:
        // the table's role cell and its value cell agree with each other.
        for (name, role, colour) in nord_rows() {
            let role = role.split_whitespace().next().expect("a role");
            assert_eq!(
                nord_colour(role),
                colour,
                "{name}'s role {role} holds another value"
            );
        }
    }

    #[test]
    fn the_contrast_figures_of_the_nord_preset_are_right() {
        // Every figure the doc states is recomputed from the code's own
        // palette, the way the dark preset's figures are.
        let theme = Theme::preset(Preset::Nord);
        let rows = family_ratios(&section(PALETTES, "Nord"));
        assert_eq!(
            rows.len(),
            Slot::ALL.len(),
            "the doc's Nord table states {} figures",
            rows.len()
        );
        let mut checked = 0;
        for (name, ratios) in rows {
            let slot = slot_named(&name);
            let style = theme.slot(slot);
            let foreground = style.fg.expect("a Nord slot has a foreground");
            let behind = style.bg.unwrap_or(nord::NORD0);
            let computed = contrast(foreground, behind);
            assert!(
                (computed - ratios[0]).abs() < 0.05,
                "{name} reads at {computed:.2}:1, and the doc says {}:1",
                ratios[0]
            );
            checked += 1;
        }
        assert_eq!(checked, Slot::ALL.len(), "Nord states {checked} figures");
    }

    #[test]
    fn the_nord_preset_reads_on_its_background() {
        // Nord's aurora is low-contrast on the polar night: the error reads
        // at 3.1:1 and keeps the family's own red. Every word still clears
        // the floor the doc states; only the border and the index are lines.
        let theme = Theme::preset(Preset::Nord);
        for slot in Slot::ALL {
            if matches!(slot, Slot::ChromeBorder | Slot::ChromeTabIndex) {
                continue;
            }
            let style = theme.slot(slot);
            let foreground = style.fg.expect("a Nord slot has a foreground");
            let behind = style.bg.unwrap_or(nord::NORD0);
            let ratio = contrast(foreground, behind);
            assert!(
                ratio >= 3.0,
                "{} reads at {ratio:.1}:1, and a word needs 3:1",
                slot.name()
            );
        }
    }

    #[test]
    fn the_nord_table_keeps_its_words_readable() {
        // The fourth review: Nord's border, its muted text and its second
        // foreground were merged into one border grade, and the muted text
        // read at 1.7:1 on the family's own base. The muted text is the
        // dimmest snow storm grey, and the second foreground is the frost.
        let section = section(PALETTES, "Nord");
        let base = base_of(&section);
        let values = family_values(&section);
        let value = |slot: &str| {
            values
                .iter()
                .find(|(name, _)| name == slot)
                .map(|(_, colours)| colours[0])
                .unwrap_or_else(|| panic!("the Nord table has no {slot} row"))
        };
        let border = value("chrome.border");
        for slot in ["text.dim", "chrome.tab_inactive"] {
            let ratio = contrast(value(slot), base[0]);
            assert!(
                ratio >= 4.5,
                "Nord's {slot} reads at {ratio:.1}:1 on its base, and a word needs 4.5:1"
            );
            assert_ne!(
                value(slot),
                border,
                "Nord's {slot} is the border's colour, and a word is not a line"
            );
        }
    }

    #[test]
    fn the_example_theme_file_matches_the_nord_table() {
        // The example in the doc is a Nord theme: every value in it is the
        // value the Nord table states.
        let section = section(PALETTES, "Nord");
        let values = family_values(&section);
        let block = PALETTES
            .split_once("```toml")
            .and_then(|(_, rest)| rest.split_once("```"))
            .map(|(block, _)| block)
            .expect("the doc has a theme file example");
        let mut checked = 0;
        for line in block.lines() {
            let Some((key, value)) = line.split_once(" = ") else {
                continue;
            };
            if !key.contains('.') {
                continue;
            }
            let Some(colour) = hexes_in(value).first().copied() else {
                continue;
            };
            let expected = values
                .iter()
                .find(|(slot, _)| slot == key)
                .map(|(_, colours)| colours[0])
                .unwrap_or_else(|| panic!("the Nord table has no {key} row"));
            assert_eq!(
                colour, expected,
                "the example's {key} is not the value the Nord table states"
            );
            checked += 1;
        }
        assert_eq!(
            checked,
            Slot::ALL.len(),
            "the example names {checked} slots"
        );
    }
    /// Mocha, as the family's page lists it: every value, in the page's
    /// order. The slots spend fifteen of the twenty-six; the rest are kept
    /// for the record, and the doc's page table is the page's list.
    const MOCHA_PAGE: [(&str, Color); 26] = [
        ("rosewater", Color::Rgb(0xf5, 0xe0, 0xdc)),
        ("flamingo", Color::Rgb(0xf2, 0xcd, 0xcd)),
        ("pink", Color::Rgb(0xf5, 0xc2, 0xe7)),
        ("mauve", Color::Rgb(0xcb, 0xa6, 0xf7)),
        ("red", Color::Rgb(0xf3, 0x8b, 0xa8)),
        ("maroon", Color::Rgb(0xeb, 0xa0, 0xac)),
        ("peach", Color::Rgb(0xfa, 0xb3, 0x87)),
        ("yellow", Color::Rgb(0xf9, 0xe2, 0xaf)),
        ("green", Color::Rgb(0xa6, 0xe3, 0xa1)),
        ("teal", Color::Rgb(0x94, 0xe2, 0xd5)),
        ("sky", Color::Rgb(0x89, 0xdc, 0xeb)),
        ("sapphire", Color::Rgb(0x74, 0xc7, 0xec)),
        ("blue", Color::Rgb(0x89, 0xb4, 0xfa)),
        ("lavender", Color::Rgb(0xb4, 0xbe, 0xfe)),
        ("text", Color::Rgb(0xcd, 0xd6, 0xf4)),
        ("subtext1", Color::Rgb(0xba, 0xc2, 0xde)),
        ("subtext0", Color::Rgb(0xa6, 0xad, 0xc8)),
        ("overlay2", Color::Rgb(0x93, 0x99, 0xb2)),
        ("overlay1", Color::Rgb(0x7f, 0x84, 0x9c)),
        ("overlay0", Color::Rgb(0x6c, 0x70, 0x86)),
        ("surface2", Color::Rgb(0x58, 0x5b, 0x70)),
        ("surface1", Color::Rgb(0x45, 0x47, 0x5a)),
        ("surface0", Color::Rgb(0x31, 0x32, 0x44)),
        ("base", Color::Rgb(0x1e, 0x1e, 0x2e)),
        ("mantle", Color::Rgb(0x18, 0x18, 0x25)),
        ("crust", Color::Rgb(0x11, 0x11, 0x1b)),
    ];

    /// The colour a Catppuccin role the doc names holds.
    fn catppuccin_colour(role: &str) -> Color {
        match role {
            "base" => catppuccin::BASE,
            "rosewater" => catppuccin::ROSEWATER,
            "text" => catppuccin::TEXT,
            "subtext1" => catppuccin::SUBTEXT1,
            "subtext0" => catppuccin::SUBTEXT0,
            "overlay2" => catppuccin::OVERLAY2,
            "overlay0" => catppuccin::OVERLAY0,
            "surface1" => catppuccin::SURFACE1,
            "mauve" => catppuccin::MAUVE,
            "yellow" => catppuccin::YELLOW,
            "green" => catppuccin::GREEN,
            "peach" => catppuccin::PEACH,
            "red" => catppuccin::RED,
            "teal" => catppuccin::TEAL,
            other => panic!("{other} is not a Catppuccin colour the slots spend"),
        }
    }

    /// The colour rows of the Catppuccin slot table: the slot, the family's
    /// role, and the mocha value.
    fn catppuccin_rows() -> Vec<(String, String, Color)> {
        let section = section(PALETTES, "Catppuccin");
        family_rows(&section)
            .into_iter()
            .filter_map(|(slot, cells)| {
                let role = cells.first()?.clone();
                // The row is the role, frappé, macchiato and mocha.
                let colour = cells
                    .iter()
                    .skip(1)
                    .filter_map(|cell| hexes_in(cell).first().copied())
                    .nth(2)?;
                Some((slot, role, colour))
            })
            .collect()
    }

    /// The rows of the page table: the role and the mocha value it lists.
    fn catppuccin_page_rows() -> Vec<(String, Color)> {
        let section = section(PALETTES, "Catppuccin");
        let table = section
            .split_once("### Mocha, as the page lists it")
            .expect("the Catppuccin section records the page's mocha list")
            .1;
        let table = table.split_once("\n### ").map_or(table, |(head, _)| head);
        let mut rows = Vec::new();
        for line in table.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if cells.len() < 4 {
                continue;
            }
            let Some(colour) = hexes_in(cells[2]).first().copied() else {
                continue;
            };
            rows.push((cells[1].to_string(), colour));
        }
        rows
    }

    #[test]
    fn the_mocha_page_table_records_every_value_the_page_lists() {
        // The Nord review found the family section short three values: it
        // recorded the ones the slots spent and not the family's palette.
        // The page's mocha list is recorded whole, in the page's order.
        let rows = catppuccin_page_rows();
        assert_eq!(
            rows.len(),
            MOCHA_PAGE.len(),
            "the doc records {} mocha values and the page lists {}",
            rows.len(),
            MOCHA_PAGE.len()
        );
        for ((role, colour), (expected, expected_colour)) in rows.iter().zip(MOCHA_PAGE.iter()) {
            assert_eq!(role, expected, "the page's order moved");
            assert_eq!(
                colour, expected_colour,
                "{role} holds a value the page does not list"
            );
        }
    }

    #[test]
    fn the_catppuccin_palette_is_the_one_the_doc_records() {
        // The doc is the source of truth: the code is checked against its
        // mocha column, so the two cannot drift apart silently.
        let theme = Theme::preset(Preset::Catppuccin);
        let rows = catppuccin_rows();
        assert_eq!(
            rows.len(),
            Slot::ALL.len(),
            "the doc's Catppuccin table does not hold the eighteen slots"
        );
        for (name, _, colour) in rows {
            let slot = slot_named(&name);
            let style = theme.slot(slot);
            if slot == Slot::TextSelected {
                // The doc records the accent; the selection puts the base
                // on it, the pair the palette records.
                assert_eq!(
                    style.bg,
                    Some(colour),
                    "{name} paints a background the doc does not record"
                );
                assert_eq!(
                    style.fg,
                    Some(catppuccin::BASE),
                    "{name} does not put the base on the accent"
                );
            } else {
                assert_eq!(
                    style.fg,
                    Some(colour),
                    "{name} paints a foreground the doc does not record"
                );
                assert_eq!(
                    style.bg, None,
                    "{name} paints a background the doc does not record"
                );
            }
        }
    }

    #[test]
    fn the_catppuccin_roles_name_the_values_the_table_states() {
        // The value beside a role is the value the page gives that role:
        // the table's role cell and its mocha cell agree with each other.
        for (name, role, colour) in catppuccin_rows() {
            let role = role.split_whitespace().next().expect("a role");
            assert_eq!(
                catppuccin_colour(role),
                colour,
                "{name}'s role {role} holds another value"
            );
        }
    }

    #[test]
    fn the_contrast_figures_of_the_catppuccin_preset_are_right() {
        // Every figure the doc states is recomputed from the code's own
        // palette, the way the dark and Nord presets' figures are. The
        // mocha column is the preset's; the other two flavours wait.
        let theme = Theme::preset(Preset::Catppuccin);
        let rows = family_ratios(&section(PALETTES, "Catppuccin"));
        assert_eq!(
            rows.len(),
            Slot::ALL.len(),
            "the doc's Catppuccin table states {} figures",
            rows.len()
        );
        let mut checked = 0;
        for (name, ratios) in rows {
            let slot = slot_named(&name);
            let style = theme.slot(slot);
            let foreground = style.fg.expect("a Catppuccin slot has a foreground");
            let behind = style.bg.unwrap_or(catppuccin::BASE);
            let computed = contrast(foreground, behind);
            assert!(
                (computed - ratios[2]).abs() < 0.05,
                "{name} reads at {computed:.2}:1, and the doc says {}:1",
                ratios[2]
            );
            checked += 1;
        }
        assert_eq!(
            checked,
            Slot::ALL.len(),
            "Catppuccin states {checked} figures"
        );
    }

    #[test]
    fn the_catppuccin_preset_reads_on_its_background() {
        // Every word clears the floor the doc states; only the border and
        // the index are lines.
        let theme = Theme::preset(Preset::Catppuccin);
        for slot in Slot::ALL {
            if matches!(slot, Slot::ChromeBorder | Slot::ChromeTabIndex) {
                continue;
            }
            let style = theme.slot(slot);
            let foreground = style.fg.expect("a Catppuccin slot has a foreground");
            let behind = style.bg.unwrap_or(catppuccin::BASE);
            let ratio = contrast(foreground, behind);
            assert!(
                ratio >= 3.0,
                "{} reads at {ratio:.1}:1, and a word needs 3:1",
                slot.name()
            );
        }
    }

    #[test]
    fn the_not_yet_section_does_not_list_a_shipped_preset() {
        // The Nord review's one-line drift: theme.md still listed Nord under
        // Not yet after it shipped, so a reader was told the preset did not
        // exist. Only a deferred variant may be listed there.
        let not_yet = THEME
            .split_once("## Not yet")
            .expect("the theme doc has a Not yet section")
            .1
            .to_lowercase();
        let mut rest = not_yet.clone();
        let deferred = [
            "nord-light",
            "latte",
            "frappé",
            "macchiato",
            "dracula",
            "alucard",
        ];
        for name in deferred {
            rest = rest.replace(name, " ");
        }
        for preset in Preset::ALL {
            if preset == Preset::Dark || preset == Preset::Terminal {
                continue;
            }
            assert!(
                !rest.contains(preset.name()),
                "theme.md lists the shipped {} preset under Not yet",
                preset.name()
            );
        }
        let named = deferred
            .iter()
            .filter(|name| not_yet.contains(*name))
            .count();
        assert!(
            named >= deferred.len(),
            "the Not yet section names {named} of the {} deferred variants",
            deferred.len()
        );
    }

    #[test]
    fn the_syntect_note_names_every_preset() {
        // The Nord review's second one-line drift: detail.rs said the code
        // token pairing held in both of raul's presets, and there were
        // three. The note names them the way theme.md does.
        const DETAIL: &str = include_str!("detail.rs");
        // The note is wrapped across doc-comment lines: compare its words,
        // not its line breaks or its comment markers.
        let words = DETAIL
            .replace("///", " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            words.contains("the same in every one of raul's presets"),
            "the syntect note does not name every preset"
        );
        assert!(
            !words.contains("both of raul's presets"),
            "the syntect note still counts raul's presets as two"
        );
    }

    #[test]
    fn the_presets_differ_in_paint() {
        // The presets are not the same palette under another name.
        for (index, one) in Preset::ALL.iter().enumerate() {
            for other in &Preset::ALL[index + 1..] {
                assert_ne!(
                    Theme::preset(*one),
                    Theme::preset(*other),
                    "{} and {} paint the same",
                    one.name(),
                    other.name()
                );
            }
        }
    }

    #[test]
    fn the_terminal_preset_is_unchanged() {
        // Nord is a dark preset and must not have moved the terminal one:
        // no colour in any slot, and the same five attributes as before.
        let theme = Theme::preset(Preset::Terminal);
        let expected = [
            (Slot::ChromeTitle, Modifier::BOLD),
            (Slot::ChromeTabActive, Modifier::BOLD | Modifier::REVERSED),
            (Slot::ChromeTabInactive, Modifier::DIM),
            (Slot::ChromeTabIndex, Modifier::DIM),
            (Slot::ChromeBorder, Modifier::DIM),
            (Slot::ChromeBorderFocus, Modifier::BOLD),
            (Slot::ChromeFooter, Modifier::DIM),
            (Slot::ChromeKey, Modifier::BOLD | Modifier::REVERSED),
            (Slot::TextNormal, Modifier::empty()),
            (Slot::TextDim, Modifier::DIM),
            (Slot::TextStrong, Modifier::BOLD),
            (Slot::TextSelected, Modifier::REVERSED),
            (Slot::SemanticOk, Modifier::ITALIC),
            (Slot::SemanticWarn, Modifier::UNDERLINED),
            (
                Slot::SemanticError,
                Modifier::BOLD | Modifier::REVERSED | Modifier::UNDERLINED,
            ),
            (Slot::SemanticInfo, Modifier::BOLD | Modifier::DIM),
            (Slot::SemanticHuman, Modifier::BOLD | Modifier::UNDERLINED),
            (Slot::SemanticAgent, Modifier::BOLD | Modifier::ITALIC),
        ];
        for (slot, attributes) in expected {
            let style = theme.slot(slot);
            assert_eq!(style.fg, None, "{} names a colour", slot.name());
            assert_eq!(style.bg, None, "{} names a colour", slot.name());
            assert_eq!(
                style.add_modifier,
                attributes,
                "{} paints different attributes in the terminal preset",
                slot.name()
            );
        }
    }

    #[test]
    fn the_terminal_preset_has_no_colour_of_its_own() {
        let theme = Theme::preset(Preset::Terminal);
        for slot in Slot::ALL {
            let style = theme.slot(slot);
            assert!(
                style.fg.is_none_or(|color| color == Color::Reset),
                "{} names a colour",
                slot.name()
            );
            assert!(
                style.bg.is_none_or(|color| color == Color::Reset),
                "{} names a colour",
                slot.name()
            );
        }
    }

    #[test]
    fn a_preset_is_parsed_by_its_name() {
        for preset in Preset::ALL {
            assert_eq!(Preset::parse(preset.name()), Some(preset));
        }
        assert_eq!(Preset::parse("nord"), Some(Preset::Nord));
        assert_eq!(Preset::parse("nord-light"), None);
        assert_eq!(Preset::parse("light"), None);
        assert_eq!(Preset::parse(""), None);
    }

    #[test]
    fn every_markdown_field_maps_to_its_slot() {
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let markdown = theme.markdown_theme();
            let fields = engine_fields(&markdown);
            assert_eq!(
                fields.len(),
                ENGINE_SLOTS.len(),
                "{} leaves a field outside the doc's list",
                preset.name()
            );
            for ((field, actual, background), (expected_field, slot, expected_background)) in
                fields.iter().zip(ENGINE_SLOTS.iter())
            {
                assert_eq!(field, expected_field, "the two field lists disagree");
                assert_eq!(
                    background, expected_background,
                    "{field} paints a side the doc does not name"
                );
                let expected = if *background {
                    theme.slot(*slot).bg.unwrap_or(Color::Reset)
                } else {
                    theme.slot(*slot).fg.unwrap_or(Color::Reset)
                };
                assert_eq!(
                    *actual,
                    expected,
                    "{} paints {field} from the wrong side of {}",
                    preset.name(),
                    slot.name()
                );
            }
        }
    }

    /// The slot a field maps to.
    fn slot_of(field: &str) -> Slot {
        ENGINE_SLOTS
            .iter()
            .find(|(name, _, _)| *name == field)
            .map(|(_, slot, _)| *slot)
            .unwrap_or_else(|| panic!("{field} is not in the mapping"))
    }

    /// The theme fields a line names, in order.
    fn theme_refs(line: &'static str) -> Vec<&'static str> {
        let mut refs = Vec::new();
        let mut rest = line;
        while let Some(i) = rest.find("theme.") {
            rest = &rest[i + 6..];
            let field = rest
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap_or("");
            if !field.is_empty() {
                refs.push(field);
            }
        }
        refs
    }

    /// The fields a `.bg(` argument is bound to: either `theme.field`
    /// itself, or a local binding the engine routes it through — a
    /// `let line_bg = theme.search_highlight_bg;`, or one place of a
    /// `let (fg, bg) = match ..` whose arms bind theme fields positionally.
    fn bound_fields(source: &'static str, ident: &str) -> Vec<&'static str> {
        let single = format!("let {ident} = theme.");
        if let Some(i) = source.find(&single) {
            let field = source[i + single.len()..]
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap_or("");
            assert!(!field.is_empty(), ".bg({ident}) is bound to no field");
            return vec![field];
        }
        let anchor = source
            .match_indices("let (")
            .map(|(i, _)| i)
            .find(|&i| {
                let close = source[i..].find(')').expect("a let-tuple closes");
                source[i + 5..i + close]
                    .split(',')
                    .any(|name| name.trim() == ident)
            })
            .unwrap_or_else(|| panic!(".bg({ident}) is bound nowhere"));
        let close = source[anchor..].find(')').expect("a let-tuple closes");
        let names: Vec<&str> = source[anchor + 5..anchor + close]
            .split(',')
            .map(str::trim)
            .collect();
        let place = names
            .iter()
            .position(|name| *name == ident)
            .expect("the tuple holds the identifier");
        let mut fields = Vec::new();
        for arm in source.lines().filter(|line| line.contains("=> (")) {
            let refs = theme_refs(arm);
            assert_eq!(
                refs.len(),
                names.len(),
                "an arm of the .bg({ident}) tuple binds {} fields",
                names.len()
            );
            fields.push(refs[place]);
        }
        assert!(!fields.is_empty(), ".bg({ident}) is bound to no field");
        fields
    }

    /// The fields the engine paints with `.bg(`, derived from its markdown
    /// sources.
    fn engine_bg_fields_from_sources() -> Vec<&'static str> {
        let mut fields = Vec::new();
        for source in ENGINE_SOURCES {
            for (i, _) in source.match_indices(".bg(") {
                let rest = &source[i + 4..];
                let close = rest.find(')').expect(".bg( closes");
                let arg = rest[..close].trim();
                if let Some(field) = arg.strip_prefix("theme.") {
                    fields.push(field);
                } else {
                    fields.extend(bound_fields(source, arg));
                }
            }
        }
        fields
    }

    #[test]
    fn only_the_engines_background_fields_take_a_slot_background() {
        // The engine paints five fields with `.bg()` and the other 42 with a
        // foreground. The five are named here, so a field moved to the wrong
        // side cannot pass by moving both of the mapping's lists with it —
        // and the five are derived from the engine's own sources, so the
        // list is not a copy of the same assumption either.
        let mut derived = engine_bg_fields_from_sources();
        derived.sort_unstable();
        derived.dedup();
        let mut from_sources = ENGINE_BACKGROUND_FIELDS.to_vec();
        from_sources.sort_unstable();
        assert_eq!(
            derived, from_sources,
            "the engine paints a background on fields other than the five"
        );
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let markdown = theme.markdown_theme();
            let mut backgrounds = Vec::new();
            for (field, color, paints_background) in engine_fields(&markdown) {
                let slot = slot_of(field);
                let expected = ENGINE_BACKGROUND_FIELDS.contains(&field);
                assert_eq!(
                    paints_background,
                    expected,
                    "{} paints {field} on a side the engine does not use",
                    preset.name()
                );
                let side = if expected {
                    theme.slot(slot).bg
                } else {
                    theme.slot(slot).fg
                };
                assert_eq!(
                    color,
                    side.unwrap_or(Color::Reset),
                    "{} paints {field} from the wrong side of {}",
                    preset.name(),
                    slot.name()
                );
                if expected {
                    backgrounds.push(field);
                }
            }
            assert_eq!(
                backgrounds,
                ENGINE_BACKGROUND_FIELDS.to_vec(),
                "{} does not paint exactly the five background fields with a background",
                preset.name()
            );
        }
        // The doc states a hit's ratio for every preset that has a
        // background; recompute each from that preset's palette, so a
        // changed foreground or selection leaves the figure stale here
        // rather than in the prose.
        let search = THEME
            .split_once("**The search is not offered")
            .expect("the theme doc records the search limitation")
            .1;
        let mut with_a_background = 0;
        let mut without_a_background = 0;
        for preset in Preset::ALL {
            let bullet = search
                .split("\n- ")
                .find(|bullet| bullet.contains(&format!("**{}**", preset.name())))
                .unwrap_or_else(|| {
                    panic!(
                        "the search paragraph does not name the {} preset",
                        preset.name()
                    )
                });
            let theme = Theme::preset(preset);
            let Some(background) = theme.slot(Slot::TextSelected).bg else {
                // The terminal preset paints no background; the doc says so.
                assert!(
                    bullet.contains("paints no background"),
                    "the search paragraph does not say why the {} preset has no hit",
                    preset.name()
                );
                without_a_background += 1;
                continue;
            };
            let figure = ratio_in(
                bullet
                    .split_once("reads at")
                    .unwrap_or_else(|| {
                        panic!("the {} search paragraph states no ratio", preset.name())
                    })
                    .1,
            )
            .unwrap_or_else(|| panic!("the {} search paragraph states no ratio", preset.name()));
            let computed = contrast(
                theme
                    .slot(Slot::TextNormal)
                    .fg
                    .expect("text.normal names a foreground"),
                background,
            );
            assert!(
                (computed - figure).abs() < 0.05,
                "{}'s search hit reads at {computed:.2}:1, and the doc says {figure}:1",
                preset.name()
            );
            with_a_background += 1;
        }
        assert!(
            with_a_background >= 2,
            "the search figure covers {with_a_background} presets"
        );
        assert_eq!(
            with_a_background + without_a_background,
            Preset::ALL.len(),
            "the search paragraph does not cover every preset"
        );
    }

    #[test]
    fn a_mark_is_the_selection_pair() {
        // The engine paints a mark as mark_fg on mark_bg, the only pair the
        // mapping splits across two slots. Both halves are text.selected: the
        // pair the palette records at 9.7:1.
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let selected = theme.slot(Slot::TextSelected);
            let markdown = theme.markdown_theme();
            assert_eq!(
                markdown.mark_fg,
                selected.fg.unwrap_or(Color::Reset),
                "{} paints a mark's foreground from another slot",
                preset.name()
            );
            assert_eq!(
                markdown.mark_bg,
                selected.bg.unwrap_or(Color::Reset),
                "{} paints a mark's background from another slot",
                preset.name()
            );
        }
        let theme = Theme::preset(Preset::Dark);
        let selected = theme.slot(Slot::TextSelected);
        let ratio = contrast(
            selected.fg.expect("the selection names a foreground"),
            selected.bg.expect("the selection names a background"),
        );
        assert!(
            ratio >= 4.5,
            "a marked word reads at {ratio:.1}:1, and a word needs 4.5:1"
        );
    }

    #[test]
    fn the_terminal_markdown_theme_names_no_colour() {
        // The terminal preset has no colour in any slot: every engine field
        // is Reset, which is the terminal's own paint.
        let theme = Theme::preset(Preset::Terminal);
        for (field, color, background) in engine_fields(&theme.markdown_theme()) {
            let slot = ENGINE_SLOTS
                .iter()
                .find(|(name, _, _)| *name == field)
                .map(|(_, slot, _)| *slot)
                .unwrap_or_else(|| panic!("{field} is not in the mapping"));
            if background {
                assert_eq!(
                    theme.slot(slot).bg,
                    None,
                    "{field}'s slot names a background in the terminal preset"
                );
            } else {
                assert_eq!(
                    theme.slot(slot).fg,
                    None,
                    "{field}'s slot names a foreground in the terminal preset"
                );
            }
            assert_eq!(
                color,
                Color::Reset,
                "{field} names a colour the terminal preset does not have"
            );
        }
    }

    #[test]
    fn every_colour_preset_keeps_the_markdown_theme_inside_its_slots() {
        // Every colour the engine is handed is a colour one of the eighteen
        // slots already paints with: no leaf or family palette leaks in.
        let mut painted_in_all = 0;
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let mut palette = Vec::new();
            for slot in Slot::ALL {
                let style = theme.slot(slot);
                palette.extend(style.fg);
                palette.extend(style.bg);
            }
            let mut painted = 0;
            for (field, color, _) in engine_fields(&theme.markdown_theme()) {
                if color == Color::Reset {
                    continue;
                }
                painted += 1;
                assert!(
                    palette.contains(&color),
                    "{} paints {field} with {color:?}, which is not one of the eighteen slots",
                    preset.name()
                );
            }
            if preset == Preset::Terminal {
                assert_eq!(painted, 0, "the terminal preset paints {painted} fields");
            } else {
                assert!(
                    painted >= 40,
                    "the {} preset paints only {painted} fields from the palette",
                    preset.name()
                );
                painted_in_all += painted;
            }
        }
        assert!(painted_in_all > 0, "no colour preset paints the engine");
    }

    /// The rows of the engine mapping table in the theme doc: the field, the
    /// slot and the rationale.
    fn engine_doc_rows() -> Vec<(String, String, String)> {
        let section = THEME
            .split_once("## The markdown engine")
            .expect("the theme doc has a markdown engine section")
            .1;
        let section = section
            .split_once("\n## ")
            .map_or(section, |(head, _)| head);
        let mut rows = Vec::new();
        for line in section.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if cells.len() < 4 || !cells[1].starts_with('`') || !cells[2].starts_with('`') {
                continue;
            }
            rows.push((
                cells[1].trim_matches('`').to_string(),
                cells[2].trim_matches('`').to_string(),
                cells[3].to_string(),
            ));
        }
        rows
    }

    #[test]
    fn the_engine_mapping_in_the_doc_is_the_one_in_the_code() {
        let rows = engine_doc_rows();
        assert_eq!(
            rows.len(),
            ENGINE_SLOTS.len(),
            "the doc lists {} fields and the engine has {}",
            rows.len(),
            ENGINE_SLOTS.len()
        );
        for (field, slot, _) in ENGINE_SLOTS {
            let (doc_slot, why) = rows
                .iter()
                .find(|(doc_field, _, _)| doc_field == field)
                .map(|(_, slot, why)| (slot, why))
                .unwrap_or_else(|| panic!("{field} is missing from the doc"));
            assert_eq!(
                doc_slot,
                slot.name(),
                "{field} maps to {doc_slot} in the doc and {} in the code",
                slot.name()
            );
            assert!(!why.is_empty(), "{field}'s row states no rationale");
        }
        for (field, _, _) in &rows {
            assert!(
                ENGINE_SLOTS.iter().any(|(known, _, _)| known == field),
                "{field} is in the doc and not in the code"
            );
        }
    }

    #[test]
    fn the_markdown_alerts_use_the_semantic_slots() {
        let slot = |field: &str| {
            ENGINE_SLOTS
                .iter()
                .find(|(name, _, _)| *name == field)
                .map(|(_, slot, _)| *slot)
                .unwrap_or_else(|| panic!("{field} is not in the mapping"))
        };
        assert_eq!(slot("alert_note"), Slot::SemanticInfo);
        assert_eq!(slot("alert_tip"), Slot::SemanticOk);
        assert_eq!(slot("alert_important"), Slot::SemanticInfo);
        assert_eq!(slot("alert_warning"), Slot::SemanticWarn);
        assert_eq!(slot("alert_caution"), Slot::SemanticError);
        // A warning and a failure are never painted alike.
        assert_ne!(slot("alert_warning"), slot("alert_caution"));
        assert_ne!(slot("alert_warning"), Slot::SemanticError);
        assert_ne!(slot("alert_caution"), Slot::SemanticWarn);
    }
}
