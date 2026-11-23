//! The theme: the colours raul and hugo paint with.
//!
//! A slot names a role; a preset decides what the role looks like. The
//! domain knows nothing about this: `crates/core` never names a colour, and
//! a phase names a slot, never a colour.

use crate::text::text_width;
use coding_crew_core::{FlowRole, Reason, Status};
use coding_crew_engine::MarkdownTheme;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

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
    /// The background of a sidebar: a step darker than the base.
    ChromeSurface,
    /// Ordinary text.
    TextNormal,
    /// Secondary text: the empty state, a marker, a hint under a line, a
    /// separator, a path, a tab that is not built yet.
    TextDim,
    /// A count, a title that matters.
    TextStrong,
    /// The mark in a document: the pair a search or a marked span is
    /// painted with, at full opacity.
    TextSelected,
    /// The background of the selected row: a subtle tint, so the row's text
    /// keeps its own colours.
    TextSelectedBg,
    /// A clean result, a finished task.
    SemanticOk,
    /// Something wants attention and is not broken.
    SemanticWarn,
    /// A failure: an unreadable file, a failed criterion, an escalation, no
    /// coding-crew folder found.
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
    /// The orchestrator's role: its icon and name in the crew and the sidebar.
    RoleOrchestrator,
    /// The runner's role, the builder that writes the code.
    RoleRunner,
    /// The reviewer's role.
    RoleReviewer,
}

impl Slot {
    /// Every slot, in the order of the spec.
    ///
    /// The list and the names are the keys of the theme file format,
    /// which comes later: they are pinned by the tests, and nothing draws
    /// a slot by name yet.
    #[allow(dead_code)]
    pub const ALL: [Slot; 23] = [
        Slot::ChromeTitle,
        Slot::ChromeTabActive,
        Slot::ChromeTabInactive,
        Slot::ChromeTabIndex,
        Slot::ChromeBorder,
        Slot::ChromeBorderFocus,
        Slot::ChromeFooter,
        Slot::ChromeKey,
        Slot::ChromeSurface,
        Slot::TextNormal,
        Slot::TextDim,
        Slot::TextStrong,
        Slot::TextSelected,
        Slot::TextSelectedBg,
        Slot::SemanticOk,
        Slot::SemanticWarn,
        Slot::SemanticError,
        Slot::SemanticInfo,
        Slot::SemanticHuman,
        Slot::SemanticAgent,
        Slot::RoleOrchestrator,
        Slot::RoleRunner,
        Slot::RoleReviewer,
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
            Slot::ChromeSurface => "chrome.surface",
            Slot::TextNormal => "text.normal",
            Slot::TextDim => "text.dim",
            Slot::TextStrong => "text.strong",
            Slot::TextSelected => "text.selected",
            Slot::TextSelectedBg => "text.selected_bg",
            Slot::SemanticOk => "semantic.ok",
            Slot::SemanticWarn => "semantic.warn",
            Slot::SemanticError => "semantic.error",
            Slot::SemanticInfo => "semantic.info",
            Slot::SemanticHuman => "semantic.human",
            Slot::SemanticAgent => "semantic.agent",
            Slot::RoleOrchestrator => "role.orchestrator",
            Slot::RoleRunner => "role.runner",
            Slot::RoleReviewer => "role.reviewer",
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
    pub surface: Style,
}

/// The colours of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    pub normal: Style,
    pub dim: Style,
    pub strong: Style,
    pub selected: Style,
    pub selected_bg: Style,
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

/// The colours of the roles: the icon and the name of each agent's role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roles {
    pub orchestrator: Style,
    pub runner: Style,
    pub reviewer: Style,
}

/// The colours raul and hugo paint with, one per slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The preset the colours came from: the theme in use.
    pub preset: Preset,
    pub chrome: Chrome,
    pub text: Text,
    pub semantic: Semantic,
    pub roles: Roles,
}

impl Theme {
    /// The theme of a preset.
    pub fn preset(preset: Preset) -> Self {
        let style = |slot: Slot| style_of(preset, slot);
        Theme {
            preset,
            chrome: Chrome {
                title: style(Slot::ChromeTitle),
                tab_active: style(Slot::ChromeTabActive),
                tab_inactive: style(Slot::ChromeTabInactive),
                tab_index: style(Slot::ChromeTabIndex),
                border: style(Slot::ChromeBorder),
                border_focus: style(Slot::ChromeBorderFocus),
                footer: style(Slot::ChromeFooter),
                key: style(Slot::ChromeKey),
                surface: style(Slot::ChromeSurface),
            },
            text: Text {
                normal: style(Slot::TextNormal),
                dim: style(Slot::TextDim),
                strong: style(Slot::TextStrong),
                selected: style(Slot::TextSelected),
                selected_bg: style(Slot::TextSelectedBg),
            },
            semantic: Semantic {
                ok: style(Slot::SemanticOk),
                warn: style(Slot::SemanticWarn),
                error: style(Slot::SemanticError),
                info: style(Slot::SemanticInfo),
                human: style(Slot::SemanticHuman),
                agent: style(Slot::SemanticAgent),
            },
            roles: Roles {
                orchestrator: style(Slot::RoleOrchestrator),
                runner: style(Slot::RoleRunner),
                reviewer: style(Slot::RoleReviewer),
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
            Slot::ChromeSurface => self.chrome.surface,
            Slot::TextNormal => self.text.normal,
            Slot::TextDim => self.text.dim,
            Slot::TextStrong => self.text.strong,
            Slot::TextSelected => self.text.selected,
            Slot::TextSelectedBg => self.text.selected_bg,
            Slot::SemanticOk => self.semantic.ok,
            Slot::SemanticWarn => self.semantic.warn,
            Slot::SemanticError => self.semantic.error,
            Slot::SemanticInfo => self.semantic.info,
            Slot::SemanticHuman => self.semantic.human,
            Slot::SemanticAgent => self.semantic.agent,
            Slot::RoleOrchestrator => self.roles.orchestrator,
            Slot::RoleRunner => self.roles.runner,
            Slot::RoleReviewer => self.roles.reviewer,
        }
    }

    /// The document's palette in this theme's preset.
    ///
    /// The document — the markdown pane — has a palette of its own: the
    /// twenty-seven [`DocRole`]s are not the chrome's [`Slot`]s. The two
    /// vocabularies sit side by side, and this theme's preset paints both from
    /// the same family palette.
    pub fn document(&self) -> DocPalette {
        document_palette(self.preset)
    }

    /// The style a document role paints outside the engine: its colour, and
    /// in the terminal preset the attributes that tell it apart from the
    /// other roles. The colour presets paint the colour alone.
    pub fn document_style(&self, role: DocRole) -> Style {
        let style = Style::new().fg(self.document().paint(role, DocSide::One));
        match self.preset {
            Preset::Terminal => style.add_modifier(terminal_attribute(role)),
            _ => style,
        }
    }

    /// The markdown engine's theme, painted from the document's roles.
    ///
    /// The engine has its own palettes; raul and hugo do not use them. Every
    /// field takes one side of one document role — a foreground for a field
    /// that paints a word or a line, a background for a field that paints one.
    /// The mapping is the `ENGINE_ROLES` table, and the test below holds the
    /// two together.
    ///
    /// The terminal preset paints every document role `Color::Reset`, so the
    /// engine names no colour at all and only its own modifiers distinguish
    /// the roles.
    ///
    /// The promise stops at `MarkdownTheme`: the engine paints code tokens
    /// with syntect's own theme, which this mapping does not reach.
    #[allow(dead_code)] // The detail pane's first caller.
    pub fn markdown_theme(&self) -> MarkdownTheme {
        let doc = self.document();
        let at = |role: DocRole, side: DocSide| doc.paint(role, side);
        MarkdownTheme {
            // The search: a hit takes the document's hit background, the
            // current match its stronger one. The engine keeps the searched
            // span's own foreground, which is why the figures in theme.md are
            // the prose on each background, and why the search is not
            // offered.
            search_highlight_bg: at(DocRole::Search, DocSide::One),
            search_match_bg: at(DocRole::Search, DocSide::Two),
            // The code's frame is the document's line, so the block stays
            // quiet around its tokens. Its label and its numbers are words, so
            // they take the quietest word grade, not a line. Inline code and
            // its chip are the code role.
            code_gutter: at(DocRole::ListDepth3, DocSide::One),
            code_frame: at(DocRole::Rule, DocSide::One),
            code_label: at(DocRole::ListDepth3, DocSide::One),
            inline_code_fg: at(DocRole::Code, DocSide::One),
            inline_code_bg: at(DocRole::Code, DocSide::Two),
            // A quote's bar and its text are the quote's colour. Rules and the
            // table's frame are lines; the table's header rule is the header's
            // own grade. A header is a chromatic heading, and a cell is the
            // prose.
            blockquote_marker: at(DocRole::Quote, DocSide::One),
            blockquote_text: at(DocRole::Quote, DocSide::One),
            rule: at(DocRole::Rule, DocSide::One),
            table_border: at(DocRole::Rule, DocSide::One),
            table_separator: at(DocRole::Rule, DocSide::Two),
            table_header: at(DocRole::HeadingItem, DocSide::One),
            table_cell: at(DocRole::Text, DocSide::One),
            // Three list depths, a step apart each time; an ordered marker
            // ranks with the first.
            list_level_1: at(DocRole::ListDepth1, DocSide::One),
            list_level_2: at(DocRole::ListDepth2, DocSide::One),
            list_level_3: at(DocRole::ListDepth3, DocSide::One),
            ordered_list: at(DocRole::ListDepth1, DocSide::One),
            // Three heading levels; the engine's deeper ones join the
            // deepest, because the document has no level below `###`.
            heading_1: at(DocRole::HeadingSection, DocSide::One),
            heading_2: at(DocRole::HeadingItem, DocSide::One),
            heading_3: at(DocRole::HeadingFinding, DocSide::One),
            heading_4: at(DocRole::HeadingFinding, DocSide::One),
            heading_other: at(DocRole::HeadingFinding, DocSide::One),
            heading_underline: at(DocRole::Rule, DocSide::One),
            // Links are one role: the marker, the text, the hover and a
            // footnote's reference.
            link_icon: at(DocRole::Link, DocSide::One),
            link_text: at(DocRole::Link, DocSide::One),
            link_hover: at(DocRole::Link, DocSide::One),
            // Ordinary and strong prose are the prose's two weights.
            text: at(DocRole::Text, DocSide::One),
            strong_text: at(DocRole::Text, DocSide::Two),
            // LaTeX and Mermaid are the code family's ink and chip.
            latex_inline_fg: at(DocRole::Code, DocSide::One),
            latex_inline_bg: at(DocRole::Code, DocSide::Two),
            latex_block_fg: at(DocRole::Code, DocSide::One),
            mermaid_keyword: at(DocRole::Code, DocSide::One),
            mermaid_arrow: at(DocRole::Rule, DocSide::One),
            mermaid_label: at(DocRole::Text, DocSide::One),
            mermaid_block_fg: at(DocRole::Code, DocSide::One),
            // A mark is the selection's pair: its foreground and its
            // background, the two sides the palette records.
            mark_fg: at(DocRole::Selection, DocSide::One),
            mark_bg: at(DocRole::Selection, DocSide::Two),
            // Tasks: a done box and an open one.
            task_checked: at(DocRole::Task, DocSide::One),
            task_unchecked: at(DocRole::Task, DocSide::Two),
            // Five alerts, five roles: note, tip, important, warning and
            // caution are five different facts, and no two share a role.
            alert_note: at(DocRole::AlertNote, DocSide::One),
            alert_tip: at(DocRole::AlertTip, DocSide::One),
            alert_important: at(DocRole::AlertImportant, DocSide::One),
            alert_warning: at(DocRole::AlertWarning, DocSide::One),
            alert_caution: at(DocRole::AlertCaution, DocSide::One),
            // A footnote's reference ranks with a link; its note is prose.
            footnote_ref: at(DocRole::Link, DocSide::One),
            footnote_text: at(DocRole::Text, DocSide::One),
            // The glyphs are not colours. The caller sets the task glyphs
            // from its icon set; the engine's defaults stand until it does.
            ..MarkdownTheme::default()
        }
    }
}

/// A role the document paints with.
///
/// The document — the markdown pane — has a palette of its own. These
/// twenty-seven roles are not the chrome's [`Slot`]s: the two vocabularies sit
/// side by side, and nothing in `crates/core` names either one. A role is a
/// job the document has, and each preset decides what that job looks like.
///
/// Six roles name a pair rather than a single colour, because their job has
/// two sides: [`DocRole::Text`] the prose and its emphasis, [`DocRole::Code`]
/// the code's ink and its chip, [`DocRole::Rule`] the document's line and the
/// table's header rule, [`DocRole::Selection`] a mark's foreground and
/// background, [`DocRole::Search`] a hit and the current match, and
/// [`DocRole::Task`] a done box and an open one. The other twenty-one name one
/// colour. Eight of the twenty-one are syntax roles: the code's tokens, painted
/// by the syntect theme the detail pane pairs with each preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocRole {
    /// The `#` line: one of the ten sections.
    HeadingSection,
    /// The `##` line: an item under a section.
    HeadingItem,
    /// The `###` line: a finding under its round. The engine's deeper
    /// headings join it, because the document has no level below `###`.
    HeadingFinding,
    /// The document's prose, and its emphasis.
    Text,
    /// The first list depth, and an ordered list's marker.
    ListDepth1,
    /// The second list depth.
    ListDepth2,
    /// The third list depth.
    ListDepth3,
    /// A block quote's bar and its text.
    Quote,
    /// A `[!NOTE]` alert.
    AlertNote,
    /// A `[!TIP]` alert.
    AlertTip,
    /// A `[!IMPORTANT]` alert.
    AlertImportant,
    /// A `[!WARNING]` alert.
    AlertWarning,
    /// A `[!CAUTION]` alert.
    AlertCaution,
    /// A link's marker, its text, its hover state, and a footnote's
    /// reference.
    Link,
    /// Code and its kin: the ink and the chip behind it.
    Code,
    /// The document's lines, and the table's header rule.
    Rule,
    /// A mark: the reader's highlight.
    Selection,
    /// The search: a hit and the current match.
    Search,
    /// A task list's box: done and open.
    Task,
    /// A keyword, and a storage keyword.
    SyntaxKeyword,
    /// A string, and a character escape inside one.
    SyntaxString,
    /// A number and a constant.
    SyntaxNumber,
    /// A function, a macro and a builtin function.
    SyntaxFunction,
    /// A type, a class and a namespace.
    SyntaxType,
    /// A comment, which is italic.
    SyntaxComment,
    /// A variable and a parameter: the code's plain ink, the document's prose.
    SyntaxVariable,
    /// A tag and an attribute name.
    SyntaxTag,
}

/// One side of a role's pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocSide {
    /// The role's first colour: its only colour, unless the role names a pair.
    One,
    /// The role's second colour: the other side of a pair.
    Two,
}

impl DocRole {
    /// Every document role, in the order of the palette doc.
    pub const ALL: [DocRole; 27] = [
        DocRole::HeadingSection,
        DocRole::HeadingItem,
        DocRole::HeadingFinding,
        DocRole::Text,
        DocRole::ListDepth1,
        DocRole::ListDepth2,
        DocRole::ListDepth3,
        DocRole::Quote,
        DocRole::AlertNote,
        DocRole::AlertTip,
        DocRole::AlertImportant,
        DocRole::AlertWarning,
        DocRole::AlertCaution,
        DocRole::Link,
        DocRole::Code,
        DocRole::Rule,
        DocRole::Selection,
        DocRole::Search,
        DocRole::Task,
        DocRole::SyntaxKeyword,
        DocRole::SyntaxString,
        DocRole::SyntaxNumber,
        DocRole::SyntaxFunction,
        DocRole::SyntaxType,
        DocRole::SyntaxComment,
        DocRole::SyntaxVariable,
        DocRole::SyntaxTag,
    ];

    /// The name of the role, as the palette doc writes it.
    pub fn name(&self) -> &'static str {
        match self {
            DocRole::HeadingSection => "document.heading.section",
            DocRole::HeadingItem => "document.heading.item",
            DocRole::HeadingFinding => "document.heading.finding",
            DocRole::Text => "document.text",
            DocRole::ListDepth1 => "document.list.1",
            DocRole::ListDepth2 => "document.list.2",
            DocRole::ListDepth3 => "document.list.3",
            DocRole::Quote => "document.quote",
            DocRole::AlertNote => "document.alert.note",
            DocRole::AlertTip => "document.alert.tip",
            DocRole::AlertImportant => "document.alert.important",
            DocRole::AlertWarning => "document.alert.warning",
            DocRole::AlertCaution => "document.alert.caution",
            DocRole::Link => "document.link",
            DocRole::Code => "document.code",
            DocRole::Rule => "document.rule",
            DocRole::Selection => "document.selection",
            DocRole::Search => "document.search",
            DocRole::Task => "document.task",
            DocRole::SyntaxKeyword => "document.syntax.keyword",
            DocRole::SyntaxString => "document.syntax.string",
            DocRole::SyntaxNumber => "document.syntax.number",
            DocRole::SyntaxFunction => "document.syntax.function",
            DocRole::SyntaxType => "document.syntax.type",
            DocRole::SyntaxComment => "document.syntax.comment",
            DocRole::SyntaxVariable => "document.syntax.variable",
            DocRole::SyntaxTag => "document.syntax.tag",
        }
    }

    /// Whether the role's job has two sides.
    pub fn carries_a_pair(&self) -> bool {
        matches!(
            self,
            DocRole::Text
                | DocRole::Code
                | DocRole::Rule
                | DocRole::Selection
                | DocRole::Search
                | DocRole::Task
        )
    }

    /// Whether the role paints a token of a code block. The engine never
    /// reads these: the detail pane's syntect theme does.
    pub fn is_syntax(&self) -> bool {
        matches!(
            self,
            DocRole::SyntaxKeyword
                | DocRole::SyntaxString
                | DocRole::SyntaxNumber
                | DocRole::SyntaxFunction
                | DocRole::SyntaxType
                | DocRole::SyntaxComment
                | DocRole::SyntaxVariable
                | DocRole::SyntaxTag
        )
    }

    /// The name of one side of the role, as the palette doc writes it.
    ///
    /// A role that names no pair has one colour, and its side is the colour
    /// itself; the doc writes that as an em dash.
    pub fn side_name(&self, side: DocSide) -> &'static str {
        match (self, side) {
            (DocRole::Text, DocSide::One) => "body",
            (DocRole::Text, DocSide::Two) => "emphasis",
            (DocRole::Code, DocSide::One) => "text",
            (DocRole::Code, DocSide::Two) => "chip",
            (DocRole::Rule, DocSide::One) => "line",
            (DocRole::Rule, DocSide::Two) => "header",
            (DocRole::Selection, DocSide::One) => "foreground",
            (DocRole::Selection, DocSide::Two) => "background",
            (DocRole::Search, DocSide::One) => "hit",
            (DocRole::Search, DocSide::Two) => "match",
            (DocRole::Task, DocSide::One) => "done",
            (DocRole::Task, DocSide::Two) => "open",
            _ => "—",
        }
    }
}

/// The prose's two weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPaint {
    pub body: Color,
    pub emphasis: Color,
}

/// The code's ink and the chip behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodePaint {
    pub text: Color,
    pub chip: Color,
}

/// The document's line and the table's header rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RulePaint {
    pub line: Color,
    pub header: Color,
}

/// A mark's foreground and background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionPaint {
    pub foreground: Color,
    pub background: Color,
}

/// The search's two backgrounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchPaint {
    pub hit: Color,
    pub current: Color,
}

/// A task's two boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskPaint {
    pub done: Color,
    pub open: Color,
}

/// The code's tokens, one colour per token class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxPaint {
    pub keyword: Color,
    pub string: Color,
    pub number: Color,
    pub function: Color,
    pub type_name: Color,
    pub comment: Color,
    pub variable: Color,
    pub tag: Color,
}

/// The document's palette: one paint per role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocPalette {
    pub heading_section: Color,
    pub heading_item: Color,
    pub heading_finding: Color,
    pub text: TextPaint,
    pub list_depth_1: Color,
    pub list_depth_2: Color,
    pub list_depth_3: Color,
    pub quote: Color,
    pub alert_note: Color,
    pub alert_tip: Color,
    pub alert_important: Color,
    pub alert_warning: Color,
    pub alert_caution: Color,
    pub link: Color,
    pub code: CodePaint,
    pub rule: RulePaint,
    pub selection: SelectionPaint,
    pub search: SearchPaint,
    pub task: TaskPaint,
    pub syntax: SyntaxPaint,
}

impl DocPalette {
    /// The colour a role's side paints.
    ///
    /// A role that names no pair has one colour, and either side is that
    /// colour: a single role cannot be asked for a colour it does not name.
    pub fn paint(&self, role: DocRole, side: DocSide) -> Color {
        match role {
            DocRole::HeadingSection => self.heading_section,
            DocRole::HeadingItem => self.heading_item,
            DocRole::HeadingFinding => self.heading_finding,
            DocRole::Text => match side {
                DocSide::One => self.text.body,
                DocSide::Two => self.text.emphasis,
            },
            DocRole::ListDepth1 => self.list_depth_1,
            DocRole::ListDepth2 => self.list_depth_2,
            DocRole::ListDepth3 => self.list_depth_3,
            DocRole::Quote => self.quote,
            DocRole::AlertNote => self.alert_note,
            DocRole::AlertTip => self.alert_tip,
            DocRole::AlertImportant => self.alert_important,
            DocRole::AlertWarning => self.alert_warning,
            DocRole::AlertCaution => self.alert_caution,
            DocRole::Link => self.link,
            DocRole::Code => match side {
                DocSide::One => self.code.text,
                DocSide::Two => self.code.chip,
            },
            DocRole::Rule => match side {
                DocSide::One => self.rule.line,
                DocSide::Two => self.rule.header,
            },
            DocRole::Selection => match side {
                DocSide::One => self.selection.foreground,
                DocSide::Two => self.selection.background,
            },
            DocRole::Search => match side {
                DocSide::One => self.search.hit,
                DocSide::Two => self.search.current,
            },
            DocRole::Task => match side {
                DocSide::One => self.task.done,
                DocSide::Two => self.task.open,
            },
            DocRole::SyntaxKeyword => self.syntax.keyword,
            DocRole::SyntaxString => self.syntax.string,
            DocRole::SyntaxNumber => self.syntax.number,
            DocRole::SyntaxFunction => self.syntax.function,
            DocRole::SyntaxType => self.syntax.type_name,
            DocRole::SyntaxComment => self.syntax.comment,
            DocRole::SyntaxVariable => self.syntax.variable,
            DocRole::SyntaxTag => self.syntax.tag,
        }
    }

    /// The palette the terminal preset paints: every role is `Reset`.
    pub const fn reset() -> Self {
        DocPalette {
            heading_section: Color::Reset,
            heading_item: Color::Reset,
            heading_finding: Color::Reset,
            text: TextPaint {
                body: Color::Reset,
                emphasis: Color::Reset,
            },
            list_depth_1: Color::Reset,
            list_depth_2: Color::Reset,
            list_depth_3: Color::Reset,
            quote: Color::Reset,
            alert_note: Color::Reset,
            alert_tip: Color::Reset,
            alert_important: Color::Reset,
            alert_warning: Color::Reset,
            alert_caution: Color::Reset,
            link: Color::Reset,
            code: CodePaint {
                text: Color::Reset,
                chip: Color::Reset,
            },
            rule: RulePaint {
                line: Color::Reset,
                header: Color::Reset,
            },
            selection: SelectionPaint {
                foreground: Color::Reset,
                background: Color::Reset,
            },
            search: SearchPaint {
                hit: Color::Reset,
                current: Color::Reset,
            },
            task: TaskPaint {
                done: Color::Reset,
                open: Color::Reset,
            },
            syntax: SyntaxPaint {
                keyword: Color::Reset,
                string: Color::Reset,
                number: Color::Reset,
                function: Color::Reset,
                type_name: Color::Reset,
                comment: Color::Reset,
                variable: Color::Reset,
                tag: Color::Reset,
            },
        }
    }
}

/// The document's palette in a preset.
///
/// The four colour presets paint the twenty-seven roles from the same family
/// palette the chrome draws from; the terminal preset paints none of them.
pub fn document_palette(preset: Preset) -> DocPalette {
    match preset {
        Preset::Dark => dark_document(),
        Preset::Nord => nord_document(),
        Preset::Catppuccin => catppuccin_document(),
        Preset::Dracula => dracula_document(),
        Preset::Terminal => DocPalette::reset(),
    }
}

/// The attributes a document role paints in the terminal preset, where every
/// colour is the terminal's own. A syntax role takes one, so its tokens read
/// apart from the body; the section heading takes the bold the engine gives a
/// heading. Every other role takes none.
fn terminal_attribute(role: DocRole) -> Modifier {
    match role {
        DocRole::HeadingSection | DocRole::SyntaxKeyword => Modifier::BOLD,
        DocRole::SyntaxString => Modifier::UNDERLINED,
        DocRole::SyntaxNumber => Modifier::ITALIC,
        DocRole::SyntaxFunction => Modifier::BOLD | Modifier::UNDERLINED,
        DocRole::SyntaxType => Modifier::ITALIC | Modifier::UNDERLINED,
        DocRole::SyntaxComment => Modifier::DIM,
        DocRole::SyntaxTag => Modifier::DIM | Modifier::ITALIC,
        _ => Modifier::empty(),
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

/// The slot that paints a role.
///
/// The creator grooms with a human, so it takes the human lane's slot, the
/// way its icon is the human's.
pub fn role_slot(role: FlowRole) -> Slot {
    match role {
        FlowRole::Orchestrator => Slot::RoleOrchestrator,
        FlowRole::Builder => Slot::RoleRunner,
        FlowRole::Reviewer => Slot::RoleReviewer,
        FlowRole::Creator => Slot::SemanticHuman,
    }
}

/// The style of a role in a theme.
pub fn role_style(theme: &Theme, role: FlowRole) -> Style {
    theme.slot(role_slot(role))
}

/// The share of black a surface takes off its base: the sidebar recedes.
const SURFACE_SHARE: f64 = 0.30;

/// The share of the accent a selected row takes over its surface.
const SELECTION_SHARE: f64 = 0.10;

/// `from` mixed toward `to` by `share`, channel by channel.
///
/// The two backgrounds of a palette are derived this way, from the family's
/// own base, text and accent, so the doc can state every figure.
pub fn mix(from: Color, to: Color, share: f64) -> Color {
    let (Color::Rgb(r, g, b), Color::Rgb(to_r, to_g, to_b)) = (from, to) else {
        return from;
    };
    let channel = |one: u8, other: u8| {
        (f64::from(one) + (f64::from(other) - f64::from(one)) * share).round() as u8
    };
    Color::Rgb(channel(r, to_r), channel(g, to_g), channel(b, to_b))
}

/// The surface of a palette: the base a step toward black.
fn surface_of(base: Color) -> Color {
    mix(base, Color::Rgb(0, 0, 0), SURFACE_SHARE)
}

/// The selected row's tint: the surface a step toward the accent.
fn selection_of(surface: Color, accent: Color) -> Color {
    mix(surface, accent, SELECTION_SHARE)
}

/// The style of the title of a block.
///
/// The title of a block is `text.strong`: a title that matters. It sits on
/// the border, and the border's attributes bleed into it, so the dimming of
/// a dimmed border is taken away again: a title is never dimmed.
pub fn block_title_style(theme: &Theme) -> Style {
    theme.text.strong.remove_modifier(Modifier::DIM)
}

/// The line the cursor is on: the tint runs under every cell, and the
/// bar runs to the edge of the pane. Each cell keeps its own colours, so a
/// status stays readable on the selected row.
pub fn selected_line(line: Line<'static>, width: usize, theme: &Theme) -> Line<'static> {
    let used: usize = line
        .spans
        .iter()
        .map(|span| text_width(&span.content))
        .sum();
    let mut spans: Vec<Span<'static>> = line
        .spans
        .into_iter()
        .map(|span| {
            let style = span.style.patch(theme.text.selected_bg);
            Span::styled(span.content, style)
        })
        .collect();
    if used < width {
        spans.push(Span::styled(
            " ".repeat(width - used),
            theme.text.selected_bg,
        ));
    }
    Line::from(spans)
}

/// What a preset's colours are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetKind {
    /// A palette the tool ships with.
    Palette,
    /// The colours of the terminal itself.
    Terminal,
}

impl PresetKind {
    /// The words a screen uses for the kind.
    pub fn label(&self) -> &'static str {
        match self {
            PresetKind::Palette => "palette",
            PresetKind::Terminal => "terminal colours",
        }
    }
}

/// The palettes raul and hugo ship with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// The dark palette. The default.
    Dark,
    /// The Nord palette: a dark one, from the family's own page.
    Nord,
    /// The Catppuccin palette: the family's mocha flavour.
    Catppuccin,
    /// The Dracula palette: the family's Dracula Classic variant.
    Dracula,
    /// The colours of the terminal itself.
    Terminal,
}

impl Preset {
    /// Every preset, in the order of the spec.
    pub const ALL: [Preset; 5] = [
        Preset::Dark,
        Preset::Nord,
        Preset::Catppuccin,
        Preset::Dracula,
        Preset::Terminal,
    ];

    /// The name of the preset, as the flag writes it.
    pub fn name(&self) -> &'static str {
        match self {
            Preset::Dark => "dark",
            Preset::Nord => "nord",
            Preset::Catppuccin => "catppuccin",
            Preset::Dracula => "dracula",
            Preset::Terminal => "terminal",
        }
    }

    /// The name of the preset, as a screen writes it.
    pub fn label(&self) -> &'static str {
        match self {
            Preset::Dark => "Dark",
            Preset::Nord => "Nord",
            Preset::Catppuccin => "Catppuccin",
            Preset::Dracula => "Dracula",
            Preset::Terminal => "Terminal",
        }
    }

    /// What the preset's colours are: a palette, or the terminal's own.
    pub fn kind(&self) -> PresetKind {
        match self {
            Preset::Terminal => PresetKind::Terminal,
            _ => PresetKind::Palette,
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
        Preset::Dracula => dracula_style(slot),
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
    /// The attention colour: a key label, and the orchestrator's role.
    pub const ATTENTION: Color = Color::Rgb(0xff, 0xd1, 0x66);
    /// The runner's role: a teal of the palette's own, apart from the agent
    /// lane's violet and the human lane's blue.
    pub const TEAL: Color = Color::Rgb(0x56, 0xd4, 0xdd);
    /// The reviewer's role: a pink, apart from the error's red.
    pub const PINK: Color = Color::Rgb(0xff, 0x9b, 0xd2);
}

/// The style of a slot in the dark palette.
///
/// The palette is ours: its eighteen values are written down in
/// docs/tui/palettes.md, with the role each one plays. It assumes a
/// terminal with a dark background.
fn dark_style(slot: Slot) -> Style {
    use dark::{
        ACCENT, ATTENTION, BACKGROUND, BORDER, BRIGHTEST, DIMMED, ERROR, FOREGROUND, INFO, MUTED,
        OK, PINK, RAISED, SECOND, SECOND_ACCENT, TEAL, WARN,
    };
    let surface = surface_of(BACKGROUND);
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
        // A step darker than the base, for the sidebar.
        Slot::ChromeSurface => Style::new().bg(surface),
        // The main foreground: ordinary text.
        Slot::TextNormal => Style::new().fg(FOREGROUND),
        // The muted foreground: secondary text.
        Slot::TextDim => Style::new().fg(MUTED),
        // The raised foreground: a count, a title that matters.
        Slot::TextStrong => Style::new().fg(RAISED).add_modifier(Modifier::BOLD),
        // The mark in a document: the accent whole, with the base on it. A
        // terminal has no alpha compositing, so the mark is not translucent.
        Slot::TextSelected => Style::new().fg(BACKGROUND).bg(ACCENT),
        // The selected row: a tint a step toward the accent from the surface.
        Slot::TextSelectedBg => Style::new().bg(selection_of(surface, ACCENT)),
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
        // The roles take hues of their own: the attention yellow, a teal and
        // a pink.
        Slot::RoleOrchestrator => Style::new().fg(ATTENTION),
        Slot::RoleRunner => Style::new().fg(TEAL),
        Slot::RoleReviewer => Style::new().fg(PINK),
    }
}

/// The document's colours in the dark palette.
///
/// The three headings take three chromatic slots — the accent, the teal and
/// the attention yellow — so no two levels share a hue. The first list depth
/// takes the success green and the second the accent blue, which share a hue
/// with an alert and a heading; the third stays the dimmed grey. The quote
/// takes the second accent, as the link does. The link takes the second
/// accent, apart from the accent the section heading takes. The alerts take
/// the success, the pink, the warning and the error, and the note the neutral.
/// The code's ink is the warning orange on the surface tint, the token classes
/// take the chromatic slots, and the comment the muted grey.
fn dark_document() -> DocPalette {
    use dark::{
        ACCENT, ATTENTION, BACKGROUND, BORDER, DIMMED, ERROR, FOREGROUND, INFO, MUTED, OK, PINK,
        RAISED, SECOND_ACCENT, TEAL, WARN,
    };
    // The chip is the selected row's tint, the one faint background the
    // palette paints.
    let tint = selection_of(surface_of(BACKGROUND), ACCENT);
    DocPalette {
        heading_section: ACCENT,
        heading_item: TEAL,
        heading_finding: ATTENTION,
        text: TextPaint {
            body: FOREGROUND,
            emphasis: RAISED,
        },
        list_depth_1: OK,
        list_depth_2: ACCENT,
        list_depth_3: DIMMED,
        quote: SECOND_ACCENT,
        alert_note: INFO,
        alert_tip: OK,
        alert_important: PINK,
        alert_warning: WARN,
        alert_caution: ERROR,
        link: SECOND_ACCENT,
        code: CodePaint {
            text: WARN,
            chip: tint,
        },
        rule: RulePaint {
            line: BORDER,
            header: RAISED,
        },
        selection: SelectionPaint {
            foreground: BACKGROUND,
            background: ACCENT,
        },
        search: SearchPaint {
            hit: BORDER,
            current: ACCENT,
        },
        task: TaskPaint {
            done: OK,
            open: MUTED,
        },
        syntax: SyntaxPaint {
            keyword: PINK,
            string: TEAL,
            number: ACCENT,
            function: SECOND_ACCENT,
            type_name: ATTENTION,
            comment: MUTED,
            variable: FOREGROUND,
            tag: OK,
        },
    }
}

/// The fourteen Nord colours that the slots spend.
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
    /// nord12: the runner's role, the aurora's orange.
    pub const NORD12: Color = Color::Rgb(0xd0, 0x87, 0x70);
    /// nord15: the orchestrator's role, the aurora's purple.
    pub const NORD15: Color = Color::Rgb(0xb4, 0x8e, 0xad);
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
        NORD0, NORD2, NORD3, NORD4, NORD5, NORD6, NORD7, NORD8, NORD9, NORD11, NORD12, NORD13,
        NORD14, NORD15,
    };
    let surface = surface_of(NORD0);
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
        // The mark in a document: the accent whole, with the base on it.
        Slot::TextSelected => Style::new().fg(NORD0).bg(NORD8),
        // The selected row: a tint a step toward the accent from the surface.
        Slot::TextSelectedBg => Style::new().bg(selection_of(surface, NORD8)),
        // A step darker than the base, for the sidebar.
        Slot::ChromeSurface => Style::new().bg(surface),
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
        // The roles take the aurora's purple and orange, and the frost.
        Slot::RoleOrchestrator => Style::new().fg(NORD15),
        Slot::RoleRunner => Style::new().fg(NORD12),
        Slot::RoleReviewer => Style::new().fg(NORD9),
    }
}

/// The document's colours in the Nord palette.
///
/// The headings take three chromatic slots — the accent, the teal and the
/// aurora's yellow — so no two levels share a hue, and the link takes the
/// frost, apart from the accent. The first list depth takes the aurora's green
/// and the second the accent blue, which may share a hue with an alert or a
/// heading; the third stays the frost. The quote takes the aurora's purple, as
/// the important alert does. The alerts take the snow storm's
/// neutral, the aurora's green, purple, orange and red. The code's ink is the
/// aurora's yellow on the surface tint, and the tokens take the frost, the
/// aurora and the snow storm, with the comment the muted snow storm grey.
fn nord_document() -> DocPalette {
    use nord::{
        NORD0, NORD3, NORD4, NORD5, NORD6, NORD7, NORD8, NORD9, NORD11, NORD12, NORD13, NORD14,
        NORD15,
    };
    let tint = selection_of(surface_of(NORD0), NORD8);
    DocPalette {
        heading_section: NORD8,
        heading_item: NORD7,
        heading_finding: NORD13,
        text: TextPaint {
            body: NORD4,
            emphasis: NORD6,
        },
        list_depth_1: NORD14,
        list_depth_2: NORD8,
        list_depth_3: NORD9,
        quote: NORD15,
        alert_note: NORD5,
        alert_tip: NORD14,
        alert_important: NORD15,
        alert_warning: NORD12,
        alert_caution: NORD11,
        link: NORD9,
        code: CodePaint {
            text: NORD13,
            chip: tint,
        },
        rule: RulePaint {
            line: NORD3,
            header: NORD6,
        },
        selection: SelectionPaint {
            foreground: NORD0,
            background: NORD8,
        },
        search: SearchPaint {
            hit: NORD3,
            current: NORD8,
        },
        task: TaskPaint {
            done: NORD14,
            open: NORD4,
        },
        syntax: SyntaxPaint {
            keyword: NORD9,
            string: NORD14,
            number: NORD15,
            function: NORD8,
            type_name: NORD7,
            comment: NORD4,
            variable: NORD5,
            tag: NORD12,
        },
    }
}

/// The seventeen Catppuccin mocha values that the slots spend.
///
/// The family's page lists twenty-six values; the mapping in
/// docs/tui/palettes.md names which role each of these seventeen plays, and
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
    /// pink: the orchestrator's role.
    pub const PINK: Color = Color::Rgb(0xf5, 0xc2, 0xe7);
    /// sapphire: the runner's role.
    pub const SAPPHIRE: Color = Color::Rgb(0x74, 0xc7, 0xec);
    /// lavender: the reviewer's role.
    pub const LAVENDER: Color = Color::Rgb(0xb4, 0xbe, 0xfe);
}

/// The style of a slot in the Catppuccin palette.
///
/// The palette is the family's mocha flavour: its eighteen values are
/// written down in docs/tui/palettes.md, with the role each one plays. It
/// is a dark preset: it assumes a terminal with a dark background and
/// paints none of its own.
fn catppuccin_style(slot: Slot) -> Style {
    use catppuccin::{
        BASE, GREEN, LAVENDER, MAUVE, OVERLAY0, OVERLAY2, PEACH, PINK, RED, ROSEWATER, SAPPHIRE,
        SUBTEXT0, SUBTEXT1, SURFACE1, TEAL, TEXT, YELLOW,
    };
    let surface = surface_of(BASE);
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
        // The mark in a document: the accent whole, with the base on it.
        Slot::TextSelected => Style::new().fg(BASE).bg(MAUVE),
        // The selected row: a tint a step toward the accent from the surface.
        Slot::TextSelectedBg => Style::new().bg(selection_of(surface, MAUVE)),
        // A step darker than the base, for the sidebar.
        Slot::ChromeSurface => Style::new().bg(surface),
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
        // The roles take pink, sapphire and lavender.
        Slot::RoleOrchestrator => Style::new().fg(PINK),
        Slot::RoleRunner => Style::new().fg(SAPPHIRE),
        Slot::RoleReviewer => Style::new().fg(LAVENDER),
    }
}

/// The document's colours in the Catppuccin mocha palette.
///
/// The headings take three chromatic slots — mauve, teal and yellow — so no
/// two levels share a hue, and the link takes the sapphire, apart from the
/// accent. The first list depth takes the green and the second the sapphire,
/// which may share a hue with the link; the third stays the subtext.
/// The quote takes the lavender, as the reviewer's role and the important
/// alert do. The alerts take the subtext, the green, the lavender, the peach
/// and the red; the note takes the subtext one step above the third depth, so
/// the two are apart. The code's ink is the peach on the surface tint, and the
/// tokens take the family's chromatic slots, with the overlay for a comment.
fn catppuccin_document() -> DocPalette {
    use catppuccin::{
        BASE, GREEN, LAVENDER, MAUVE, OVERLAY2, PEACH, PINK, RED, ROSEWATER, SAPPHIRE, SUBTEXT0,
        SUBTEXT1, SURFACE1, TEAL, TEXT, YELLOW,
    };
    let tint = selection_of(surface_of(BASE), MAUVE);
    DocPalette {
        heading_section: MAUVE,
        heading_item: TEAL,
        heading_finding: YELLOW,
        text: TextPaint {
            body: SUBTEXT1,
            emphasis: ROSEWATER,
        },
        list_depth_1: GREEN,
        list_depth_2: SAPPHIRE,
        list_depth_3: SUBTEXT0,
        quote: LAVENDER,
        alert_note: SUBTEXT1,
        alert_tip: GREEN,
        alert_important: LAVENDER,
        alert_warning: PEACH,
        alert_caution: RED,
        link: SAPPHIRE,
        code: CodePaint {
            text: PEACH,
            chip: tint,
        },
        rule: RulePaint {
            line: SURFACE1,
            header: ROSEWATER,
        },
        selection: SelectionPaint {
            foreground: BASE,
            background: MAUVE,
        },
        search: SearchPaint {
            hit: SURFACE1,
            current: MAUVE,
        },
        task: TaskPaint {
            done: GREEN,
            open: SUBTEXT1,
        },
        syntax: SyntaxPaint {
            keyword: MAUVE,
            string: GREEN,
            number: PINK,
            function: SAPPHIRE,
            type_name: YELLOW,
            comment: OVERLAY2,
            variable: TEXT,
            tag: RED,
        },
    }
}

/// The twelve Dracula values that the slots spend.
///
/// The family's page lists twelve values in its color palette and sixteen in
/// its ANSI one; the mapping in docs/tui/palettes.md names which role each of
/// these twelve plays, and why the rest are kept for the record. The variant
/// is Dracula Classic: alucard waits with the light theme.
mod dracula {
    use ratatui::style::Color;

    /// background: the base the figures are measured against, and the
    /// selection's foreground.
    pub const BACKGROUND: Color = Color::Rgb(0x28, 0x2a, 0x36);
    /// current line: the border.
    pub const CURRENT_LINE: Color = Color::Rgb(0x62, 0x72, 0xa4);
    /// foreground: the main foreground, and the neutral.
    pub const FOREGROUND: Color = Color::Rgb(0xf8, 0xf8, 0xf2);
    /// comment: the muted foreground, the dimmed chrome, the tabs that are
    /// not showing and the footer's labels.
    pub const COMMENT: Color = Color::Rgb(0x62, 0x72, 0xa4);
    /// red: the error colour.
    pub const RED: Color = Color::Rgb(0xff, 0x55, 0x55);
    /// orange: the warning colour.
    pub const ORANGE: Color = Color::Rgb(0xff, 0xb8, 0x6c);
    /// yellow: the attention colour.
    pub const YELLOW: Color = Color::Rgb(0xf1, 0xfa, 0x8c);
    /// green: the success colour.
    pub const GREEN: Color = Color::Rgb(0x50, 0xfa, 0x7b);
    /// cyan: the second accent, the agent lane.
    pub const CYAN: Color = Color::Rgb(0x8b, 0xe9, 0xfd);
    /// purple: the accent: the focus border, the selection, the human lane.
    pub const PURPLE: Color = Color::Rgb(0xbd, 0x93, 0xf9);
    /// ANSI bright white: the raised and the brightest foreground.
    pub const BRIGHT_WHITE: Color = Color::Rgb(0xff, 0xff, 0xff);
    /// pink: the orchestrator's role.
    pub const PINK: Color = Color::Rgb(0xff, 0x79, 0xc6);
}

/// The style of a slot in the Dracula palette.
///
/// The palette is the family's: its eighteen values are written down in
/// docs/tui/palettes.md, with the role each one plays. It is a dark preset:
/// it assumes a terminal with a dark background and paints none of its own.
fn dracula_style(slot: Slot) -> Style {
    use dracula::{
        BACKGROUND, BRIGHT_WHITE, COMMENT, CURRENT_LINE, CYAN, FOREGROUND, GREEN, ORANGE, PINK,
        PURPLE, RED, YELLOW,
    };
    let surface = surface_of(BACKGROUND);
    match slot {
        // The brightest foreground: the ANSI bright white, a step above the
        // plain foreground.
        Slot::ChromeTitle => Style::new().fg(BRIGHT_WHITE).add_modifier(Modifier::BOLD),
        // The main foreground: the tab that is showing.
        Slot::ChromeTabActive => Style::new().fg(FOREGROUND).add_modifier(Modifier::BOLD),
        // The second foreground: the family's one dimmed grey, the comment.
        Slot::ChromeTabInactive => Style::new().fg(COMMENT),
        // The dimmed chrome: the `3/6` index.
        Slot::ChromeTabIndex => Style::new().fg(COMMENT),
        // The border: the current line, which the family paints in the
        // comment's own grey on purpose.
        Slot::ChromeBorder => Style::new().fg(CURRENT_LINE),
        // The accent: the border of the focused block.
        Slot::ChromeBorderFocus => Style::new().fg(PURPLE),
        // The footer's labels take the comment too: at 3.0:1 it is the floor
        // a word needs, and the family has no other muted foreground.
        Slot::ChromeFooter => Style::new().fg(COMMENT),
        // The attention colour: a key label.
        Slot::ChromeKey => Style::new().fg(YELLOW).add_modifier(Modifier::BOLD),
        // The main foreground: ordinary text.
        Slot::TextNormal => Style::new().fg(FOREGROUND),
        // The muted foreground: the comment.
        Slot::TextDim => Style::new().fg(COMMENT),
        // The raised foreground: the ANSI bright white.
        Slot::TextStrong => Style::new().fg(BRIGHT_WHITE).add_modifier(Modifier::BOLD),
        // The mark in a document: the accent whole, with the base on it.
        Slot::TextSelected => Style::new().fg(BACKGROUND).bg(PURPLE),
        // The selected row: a tint a step toward the accent from the surface.
        Slot::TextSelectedBg => Style::new().bg(selection_of(surface, PURPLE)),
        // A step darker than the base, for the sidebar.
        Slot::ChromeSurface => Style::new().bg(surface),
        // A clean result, a finished task.
        Slot::SemanticOk => Style::new().fg(GREEN),
        // Something wants attention and is not broken.
        Slot::SemanticWarn => Style::new().fg(ORANGE),
        // A failure: the family's own red.
        Slot::SemanticError => Style::new().fg(RED),
        // Neutral information: both near-greys are spent, so the information
        // takes the main foreground.
        Slot::SemanticInfo => Style::new().fg(FOREGROUND),
        // The human lane takes the accent.
        Slot::SemanticHuman => Style::new().fg(PURPLE),
        // The agent lane takes the second accent.
        Slot::SemanticAgent => Style::new().fg(CYAN),
        // The roles take pink, and the two hues the family already spends
        // for the agent lane and the attention colour.
        Slot::RoleOrchestrator => Style::new().fg(PINK),
        Slot::RoleRunner => Style::new().fg(CYAN),
        Slot::RoleReviewer => Style::new().fg(YELLOW),
    }
}

/// The document's colours in the Dracula Classic palette.
///
/// Dracula names two whites and one blue-grey, and the chromatic colours are
/// the spec's. The headings take the purple, the cyan and the yellow, so no
/// two levels share a hue; the link takes the cyan, apart from the purple. The
/// first list depth takes the green. The spec names no blue of its own, so the
/// second takes the cyan, the nearest hue the spec has to one; both may share
/// a hue with a heading. The third stays the blue-grey, and the quote takes
/// the purple. The alerts take the foreground, the green, the pink, the orange
/// and the red. The code's ink is the orange on the selection's tint, and the
/// tokens are the spec's syntax colours, with the comment in its own blue-grey.
fn dracula_document() -> DocPalette {
    use dracula::{
        BACKGROUND, BRIGHT_WHITE, COMMENT, CURRENT_LINE, CYAN, FOREGROUND, GREEN, ORANGE, PINK,
        PURPLE, RED, YELLOW,
    };
    let tint = selection_of(surface_of(BACKGROUND), PURPLE);
    DocPalette {
        heading_section: PURPLE,
        heading_item: CYAN,
        heading_finding: YELLOW,
        text: TextPaint {
            body: FOREGROUND,
            emphasis: BRIGHT_WHITE,
        },
        list_depth_1: GREEN,
        list_depth_2: CYAN,
        list_depth_3: COMMENT,
        quote: PURPLE,
        alert_note: FOREGROUND,
        alert_tip: GREEN,
        alert_important: PINK,
        alert_warning: ORANGE,
        alert_caution: RED,
        link: CYAN,
        code: CodePaint {
            text: ORANGE,
            chip: tint,
        },
        rule: RulePaint {
            line: CURRENT_LINE,
            header: BRIGHT_WHITE,
        },
        selection: SelectionPaint {
            foreground: BACKGROUND,
            background: PURPLE,
        },
        search: SearchPaint {
            hit: CURRENT_LINE,
            current: PURPLE,
        },
        task: TaskPaint {
            done: GREEN,
            open: COMMENT,
        },
        syntax: SyntaxPaint {
            keyword: PINK,
            string: YELLOW,
            number: PURPLE,
            function: GREEN,
            type_name: CYAN,
            comment: COMMENT,
            variable: FOREGROUND,
            tag: RED,
        },
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
        // The terminal has no tint: the selected row is reversed, as the
        // selection always was.
        Slot::TextSelectedBg => Style::new().add_modifier(Modifier::REVERSED),
        // The terminal's own background is the surface: nothing is painted.
        Slot::ChromeSurface => Style::new(),
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
        // The roles are told apart by attributes no semantic slot holds.
        Slot::RoleOrchestrator => Style::new()
            .add_modifier(Modifier::ITALIC)
            .add_modifier(Modifier::UNDERLINED),
        Slot::RoleRunner => Style::new()
            .add_modifier(Modifier::DIM)
            .add_modifier(Modifier::ITALIC),
        Slot::RoleReviewer => Style::new()
            .add_modifier(Modifier::UNDERLINED)
            .add_modifier(Modifier::DIM),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use ratatui::style::Color;

    /// The palettes doc: the source of truth for the dark palette.
    const PALETTES: &str = include_str!("../../../docs/tui/palettes.md");

    /// The theme doc: the source of truth for the engine mapping.
    const THEME: &str = include_str!("../../../docs/tui/theme.md");

    /// The repository's agent rules: the crate list the workspace must hold.
    const AGENTS: &str = include_str!("../../../AGENTS.md");

    /// The workspace manifest: the members of the build.
    const WORKSPACE: &str = include_str!("../../../Cargo.toml");

    /// The crates `AGENTS.md` lists, by directory under `crates/`.
    fn documented_crates() -> Vec<&'static str> {
        AGENTS
            .lines()
            .filter_map(|line| line.strip_prefix("- `"))
            .filter_map(|line| line.split('`').next())
            .filter(|path| path.starts_with("crates/"))
            .collect()
    }

    /// The workspace members named in the root manifest.
    fn workspace_members() -> Vec<&'static str> {
        let members = WORKSPACE
            .lines()
            .find(|line| line.trim_start().starts_with("members ="))
            .expect("the members of the workspace");
        let inner = members
            .split_once('[')
            .and_then(|(_, rest)| rest.split_once(']'))
            .map(|(inner, _)| inner)
            .expect("a member list");
        inner
            .split(',')
            .map(|member| member.trim().trim_matches('"'))
            .filter(|member| !member.is_empty())
            .collect()
    }

    #[test]
    fn the_documented_crates_are_the_workspace_members() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let documented = documented_crates();
        let members = workspace_members();
        // Every member is a crate the rules name, and the tree holds it.
        for member in &members {
            assert!(
                documented.contains(member),
                "{member} is a workspace member but AGENTS.md does not list it"
            );
            assert!(
                root.join(member).join("Cargo.toml").is_file(),
                "{member} is a workspace member but has no manifest"
            );
        }
        // A documented crate that is on disk is a member: a plan may name a
        // crate before it exists, and it joins the workspace when it does.
        for documented in &documented {
            if root.join(documented).is_dir() {
                assert!(
                    members.contains(documented),
                    "{documented} is documented and on disk but not a workspace member"
                );
            }
        }
    }

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

    /// The engine's theme fields, the document role each maps to, the side of
    /// the role it paints, and whether the field paints a background.
    ///
    /// The engine exposes no reflection, so this list is the coverage of the
    /// 47 fields: a field added upstream has to be added here and in the doc.
    const ENGINE_ROLES: [(&str, DocRole, DocSide, bool); 47] = [
        ("search_highlight_bg", DocRole::Search, DocSide::One, true),
        ("search_match_bg", DocRole::Search, DocSide::Two, true),
        ("code_gutter", DocRole::ListDepth3, DocSide::One, false),
        ("blockquote_marker", DocRole::Quote, DocSide::One, false),
        ("list_level_1", DocRole::ListDepth1, DocSide::One, false),
        ("list_level_2", DocRole::ListDepth2, DocSide::One, false),
        ("list_level_3", DocRole::ListDepth3, DocSide::One, false),
        ("ordered_list", DocRole::ListDepth1, DocSide::One, false),
        ("table_border", DocRole::Rule, DocSide::One, false),
        ("table_separator", DocRole::Rule, DocSide::Two, false),
        ("table_header", DocRole::HeadingItem, DocSide::One, false),
        ("table_cell", DocRole::Text, DocSide::One, false),
        ("heading_1", DocRole::HeadingSection, DocSide::One, false),
        ("heading_2", DocRole::HeadingItem, DocSide::One, false),
        ("heading_3", DocRole::HeadingFinding, DocSide::One, false),
        ("heading_4", DocRole::HeadingFinding, DocSide::One, false),
        (
            "heading_other",
            DocRole::HeadingFinding,
            DocSide::One,
            false,
        ),
        ("heading_underline", DocRole::Rule, DocSide::One, false),
        ("code_frame", DocRole::Rule, DocSide::One, false),
        ("code_label", DocRole::ListDepth3, DocSide::One, false),
        ("inline_code_fg", DocRole::Code, DocSide::One, false),
        ("inline_code_bg", DocRole::Code, DocSide::Two, true),
        ("rule", DocRole::Rule, DocSide::One, false),
        ("link_icon", DocRole::Link, DocSide::One, false),
        ("link_text", DocRole::Link, DocSide::One, false),
        ("link_hover", DocRole::Link, DocSide::One, false),
        ("blockquote_text", DocRole::Quote, DocSide::One, false),
        ("text", DocRole::Text, DocSide::One, false),
        ("strong_text", DocRole::Text, DocSide::Two, false),
        ("latex_inline_fg", DocRole::Code, DocSide::One, false),
        ("latex_inline_bg", DocRole::Code, DocSide::Two, true),
        ("latex_block_fg", DocRole::Code, DocSide::One, false),
        ("mermaid_keyword", DocRole::Code, DocSide::One, false),
        ("mermaid_arrow", DocRole::Rule, DocSide::One, false),
        ("mermaid_label", DocRole::Text, DocSide::One, false),
        ("mermaid_block_fg", DocRole::Code, DocSide::One, false),
        ("mark_fg", DocRole::Selection, DocSide::One, false),
        ("mark_bg", DocRole::Selection, DocSide::Two, true),
        ("task_checked", DocRole::Task, DocSide::One, false),
        ("task_unchecked", DocRole::Task, DocSide::Two, false),
        ("alert_note", DocRole::AlertNote, DocSide::One, false),
        ("alert_tip", DocRole::AlertTip, DocSide::One, false),
        (
            "alert_important",
            DocRole::AlertImportant,
            DocSide::One,
            false,
        ),
        ("alert_warning", DocRole::AlertWarning, DocSide::One, false),
        ("alert_caution", DocRole::AlertCaution, DocSide::One, false),
        ("footnote_ref", DocRole::Link, DocSide::One, false),
        ("footnote_text", DocRole::Text, DocSide::One, false),
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
                "chrome.surface",
                "text.normal",
                "text.dim",
                "text.strong",
                "text.selected",
                "text.selected_bg",
                "semantic.ok",
                "semantic.warn",
                "semantic.error",
                "semantic.info",
                "semantic.human",
                "semantic.agent",
                "role.orchestrator",
                "role.runner",
                "role.reviewer",
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
        // the terminal's own: only the colour presets paint all of them.
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
    fn the_words_read_on_the_base_the_surface_and_the_selected_tint() {
        // The sidebar draws its words on the surface, and a selected row
        // draws them on the tint. Each word there is held to the floor the
        // doc states, 3:1, on every background it sits on.
        for preset in colour_presets() {
            let theme = Theme::preset(preset);
            let base = document_base(preset);
            let surface = theme.chrome.surface.bg.expect("the surface is a colour");
            let tint = theme.text.selected_bg.bg.expect("the tint is a colour");
            for slot in [
                Slot::ChromeTabActive,
                Slot::TextNormal,
                Slot::TextDim,
                Slot::TextStrong,
                Slot::SemanticOk,
                Slot::SemanticWarn,
                Slot::SemanticError,
                Slot::SemanticInfo,
                Slot::SemanticHuman,
                Slot::SemanticAgent,
                Slot::RoleOrchestrator,
                Slot::RoleRunner,
                Slot::RoleReviewer,
            ] {
                let foreground = theme.slot(slot).fg.expect("a word is a colour");
                for behind in [base, surface, tint] {
                    let ratio = contrast(foreground, behind);
                    assert!(
                        ratio >= 3.0,
                        "{} reads at {ratio:.1}:1 in the {} preset, and a word needs 3:1",
                        slot.name(),
                        preset.name()
                    );
                }
            }
        }
    }

    #[test]
    fn the_selected_tint_is_a_step_from_the_surface() {
        // The tint is subtle: it differs from the surface, and it is a step
        // from it, not a fill. The step is at least 1.15:1, so the tint is
        // visible where the base and the tint are close, and under 1.5:1.
        for preset in colour_presets() {
            let theme = Theme::preset(preset);
            let surface = theme.chrome.surface.bg.expect("the surface is a colour");
            let tint = theme.text.selected_bg.bg.expect("the tint is a colour");
            let ratio = contrast(surface, tint);
            assert!(
                (1.15..1.5).contains(&ratio),
                "the {} preset's tint is {ratio:.2}:1 from the surface, and a step is 1.15:1 to 1.5:1",
                preset.name()
            );
        }
    }

    #[test]
    fn the_surface_recedes_below_the_base() {
        // The sidebar is recessed: each colour preset's surface is darker
        // than its base.
        for preset in colour_presets() {
            let theme = Theme::preset(preset);
            let surface = theme.chrome.surface.bg.expect("the surface is a colour");
            assert!(
                luminance(surface) < luminance(document_base(preset)),
                "the {} preset's surface is not darker than its base",
                preset.name()
            );
        }
    }

    #[test]
    fn the_terminal_roles_are_told_apart() {
        // The roles name no hue in the terminal, so they are told apart by
        // attribute: each holds a set no other role or semantic slot holds.
        let theme = Theme::preset(Preset::Terminal);
        let roles = [Slot::RoleOrchestrator, Slot::RoleRunner, Slot::RoleReviewer];
        let others = [
            Slot::TextNormal,
            Slot::SemanticOk,
            Slot::SemanticWarn,
            Slot::SemanticError,
            Slot::SemanticInfo,
            Slot::SemanticHuman,
            Slot::SemanticAgent,
        ];
        for slot in roles {
            let style = theme.slot(slot);
            for other in roles.iter().chain(others.iter()) {
                if *other == slot {
                    continue;
                }
                assert_ne!(
                    style,
                    theme.slot(*other),
                    "{} paints like {}",
                    slot.name(),
                    other.name()
                );
            }
        }
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
            "the doc's dark table does not hold every slot"
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

    /// The ratio a table cell states after a colour, like `` `#ffffff` 18.9:1 ``.
    fn figure_in(cell: &str) -> Option<f64> {
        let (before, _) = cell.split_once(":1")?;
        before.split_whitespace().last()?.trim().parse().ok()
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
        !matches!(
            slot,
            "chrome.border" | "chrome.tab_index" | "chrome.surface" | "text.selected_bg"
        )
    }

    /// The slots that paint a background and no word: the surface and the
    /// selected row's tint.
    fn is_a_background(slot: Slot) -> bool {
        matches!(slot, Slot::ChromeSurface | Slot::TextSelectedBg)
    }

    /// The slots that hold a word: every slot but the backgrounds.
    fn words() -> usize {
        Slot::ALL
            .iter()
            .filter(|slot| !is_a_background(**slot))
            .count()
    }

    #[test]
    fn every_family_table_names_every_slot() {
        // The mapping table promises every theme the same twenty-three slots,
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

    /// Nord's page: its sixteen colours, in the page's order.
    const NORD_PAGE: [(&str, Color); 16] = [
        ("nord0", Color::Rgb(0x2e, 0x34, 0x40)),
        ("nord1", Color::Rgb(0x3b, 0x42, 0x52)),
        ("nord2", Color::Rgb(0x43, 0x4c, 0x5e)),
        ("nord3", Color::Rgb(0x4c, 0x56, 0x6a)),
        ("nord4", Color::Rgb(0xd8, 0xde, 0xe9)),
        ("nord5", Color::Rgb(0xe5, 0xe9, 0xf0)),
        ("nord6", Color::Rgb(0xec, 0xef, 0xf4)),
        ("nord7", Color::Rgb(0x8f, 0xbc, 0xbb)),
        ("nord8", Color::Rgb(0x88, 0xc0, 0xd0)),
        ("nord9", Color::Rgb(0x81, 0xa1, 0xc1)),
        ("nord10", Color::Rgb(0x5e, 0x81, 0xac)),
        ("nord11", Color::Rgb(0xbf, 0x61, 0x6a)),
        ("nord12", Color::Rgb(0xd0, 0x87, 0x70)),
        ("nord13", Color::Rgb(0xeb, 0xcb, 0x8b)),
        ("nord14", Color::Rgb(0xa3, 0xbe, 0x8c)),
        ("nord15", Color::Rgb(0xb4, 0x8e, 0xad)),
    ];

    /// The four named groups the page gives the palette.
    const NORD_GROUPS: [&str; 4] = ["polar night", "snow storm", "frost", "aurora"];

    /// The three snow storm greys that carry words.
    const NORD_WORD_GREYS: [Color; 3] = [nord::NORD4, nord::NORD5, nord::NORD6];

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
            "nord12" => nord::NORD12,
            "nord15" => nord::NORD15,
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
            "the doc's Nord table does not hold every slot"
        );
        for (name, _, colour) in rows {
            let slot = slot_named(&name);
            let style = theme.slot(slot);
            if is_a_background(slot) {
                assert_eq!(
                    style.fg, None,
                    "{name} paints a foreground the doc does not record"
                );
                assert_eq!(
                    style.bg,
                    Some(colour),
                    "{name} paints a background the doc does not record"
                );
            } else if slot == Slot::TextSelected {
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
            if is_a_background(slot_named(&name)) {
                continue;
            }
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
            words(),
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
        assert_eq!(checked, words(), "Nord states {checked} figures");
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
            let Some(foreground) = style.fg else {
                continue;
            };
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
    fn the_nord_values_kept_for_the_record_are_the_pages() {
        // The Nord section records four values no slot spends. They are
        // values, so the gate reads them: each is the page's, and none is
        // painted.
        let section = section(PALETTES, "Nord");
        let kept = ["nord1", "nord10"];
        let recorded = section
            .split_once("**nord1 and nord10 are kept for the record.**")
            .expect("the Nord section no longer records the kept values")
            .1;
        for role in kept {
            let expected = NORD_PAGE
                .iter()
                .find(|(name, _)| *name == role)
                .unwrap_or_else(|| panic!("{role} is not on the Nord page"))
                .1;
            let at = recorded
                .find(role)
                .unwrap_or_else(|| panic!("the Nord section no longer records {role}"));
            let colour = hexes_in(&recorded[at..])
                .first()
                .copied()
                .expect("a colour after the role");
            assert_eq!(
                colour, expected,
                "{role} holds a value the page does not list"
            );
        }
        let theme = Theme::preset(Preset::Nord);
        for role in kept {
            let colour = NORD_PAGE
                .iter()
                .find(|(name, _)| *name == role)
                .expect("a kept role")
                .1;
            for slot in Slot::ALL {
                let style = theme.slot(slot);
                assert_ne!(
                    style.fg,
                    Some(colour),
                    "{} spends the kept {role}",
                    slot.name()
                );
                assert_ne!(
                    style.bg,
                    Some(colour),
                    "{} spends the kept {role}",
                    slot.name()
                );
            }
        }
        for group in NORD_GROUPS {
            assert!(
                section.contains(group),
                "the Nord section does not name its {group} group"
            );
        }
    }

    #[test]
    fn every_value_a_family_section_records_in_prose_is_a_page_value() {
        // A value written in the prose is read by the gate too: a new colour
        // on the page's record has to be one the page lists, not one written
        // from memory. The tables are pinned separately, against the page's
        // order.
        for heading in FAMILIES {
            let section = section(PALETTES, heading);
            let known: Vec<Color> = match heading {
                "Catppuccin" => {
                    assert_eq!(
                        base_of(&section),
                        CATPPUCCIN_BASES.to_vec(),
                        "the doc's flavour bases are not the page's"
                    );
                    let mut known: Vec<Color> = CATPPUCCIN_BASES.to_vec();
                    known.extend(MOCHA_PAGE.iter().map(|(_, colour)| *colour));
                    known
                }
                "Dracula" => {
                    let mut known: Vec<Color> =
                        DRACULA_COLOURS.iter().map(|(_, colour)| *colour).collect();
                    known.extend(DRACULA_ANSI.iter().map(|(_, colour)| *colour));
                    known.extend(DRACULA_FUNCTIONAL.iter().map(|(_, colour)| *colour));
                    known.extend(DRACULA_UI_BACKGROUNDS.iter().map(|(_, colour)| *colour));
                    known.push(DRACULA_CURRENT_LINE_FALLBACK);
                    known
                }
                "Nord" => NORD_PAGE.iter().map(|(_, colour)| *colour).collect(),
                other => panic!("{other} is not a family"),
            };
            for line in section.lines() {
                let line = line.trim();
                if line.starts_with('|') || line.starts_with('#') {
                    continue;
                }
                for colour in hexes_in(line) {
                    assert!(
                        known.contains(&colour),
                        "{heading} records {colour:?} in its prose, and the page does not list it"
                    );
                }
            }
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
    /// order. The slots spend seventeen of the twenty-six; the rest are kept
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

    /// The bases of the three dark flavours, in the doc's order.
    const CATPPUCCIN_BASES: [Color; 3] = [
        Color::Rgb(0x30, 0x34, 0x46),
        Color::Rgb(0x24, 0x27, 0x3a),
        Color::Rgb(0x1e, 0x1e, 0x2e),
    ];

    /// The four Catppuccin flavours the page names. Latte is the light one;
    /// the other three are dark.
    const CATPPUCCIN_FLAVOURS: [&str; 4] = ["latte", "frappé", "macchiato", "mocha"];

    /// The flavour that waits with the light theme.
    const CATPPUCCIN_LIGHT_FLAVOURS: [&str; 1] = ["latte"];

    /// The word greys the family has: the ordinary text, the muted text and
    /// the information.
    const CATPPUCCIN_WORD_GREYS: [&str; 3] = ["text", "subtext1", "subtext0"];

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
            "pink" => catppuccin::PINK,
            "sapphire" => catppuccin::SAPPHIRE,
            "lavender" => catppuccin::LAVENDER,
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
            "the doc's Catppuccin table does not hold every slot"
        );
        for (name, _, colour) in rows {
            let slot = slot_named(&name);
            let style = theme.slot(slot);
            if is_a_background(slot) {
                assert_eq!(
                    style.fg, None,
                    "{name} paints a foreground the doc does not record"
                );
                assert_eq!(
                    style.bg,
                    Some(colour),
                    "{name} paints a background the doc does not record"
                );
            } else if slot == Slot::TextSelected {
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
            if is_a_background(slot_named(&name)) {
                continue;
            }
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
            words(),
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
        assert_eq!(checked, words(), "Catppuccin states {checked} figures");
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
            let Some(foreground) = style.fg else {
                continue;
            };
            let behind = style.bg.unwrap_or(catppuccin::BASE);
            let ratio = contrast(foreground, behind);
            assert!(
                ratio >= 3.0,
                "{} reads at {ratio:.1}:1, and a word needs 3:1",
                slot.name()
            );
        }
    }

    /// Dracula Classic, as the family's page lists it: the page's color
    /// palette, in the page's order. The slots spend ten of the twelve; the
    /// selection and the pink are kept for the record.
    const DRACULA_COLOURS: [(&str, Color); 12] = [
        ("background", Color::Rgb(0x28, 0x2a, 0x36)),
        ("current line", Color::Rgb(0x62, 0x72, 0xa4)),
        ("selection", Color::Rgb(0x44, 0x47, 0x5a)),
        ("foreground", Color::Rgb(0xf8, 0xf8, 0xf2)),
        ("comment", Color::Rgb(0x62, 0x72, 0xa4)),
        ("red", Color::Rgb(0xff, 0x55, 0x55)),
        ("orange", Color::Rgb(0xff, 0xb8, 0x6c)),
        ("yellow", Color::Rgb(0xf1, 0xfa, 0x8c)),
        ("green", Color::Rgb(0x50, 0xfa, 0x7b)),
        ("cyan", Color::Rgb(0x8b, 0xe9, 0xfd)),
        ("purple", Color::Rgb(0xbd, 0x93, 0xf9)),
        ("pink", Color::Rgb(0xff, 0x79, 0xc6)),
    ];

    /// The page's ANSI palette, for terminal applications: sixteen values,
    /// eight restating a colour above and eight its own. The theme takes one,
    /// the bright white.
    const DRACULA_ANSI: [(&str, Color); 16] = [
        ("AnsiBlack", Color::Rgb(0x21, 0x22, 0x2c)),
        ("AnsiRed", Color::Rgb(0xff, 0x55, 0x55)),
        ("AnsiGreen", Color::Rgb(0x50, 0xfa, 0x7b)),
        ("AnsiYellow", Color::Rgb(0xf1, 0xfa, 0x8c)),
        ("AnsiBlue", Color::Rgb(0xbd, 0x93, 0xf9)),
        ("AnsiMagenta", Color::Rgb(0xff, 0x79, 0xc6)),
        ("AnsiCyan", Color::Rgb(0x8b, 0xe9, 0xfd)),
        ("AnsiWhite", Color::Rgb(0xf8, 0xf8, 0xf2)),
        ("AnsiBrightBlack", Color::Rgb(0x62, 0x72, 0xa4)),
        ("AnsiBrightRed", Color::Rgb(0xff, 0x6e, 0x6e)),
        ("AnsiBrightGreen", Color::Rgb(0x69, 0xff, 0x94)),
        ("AnsiBrightYellow", Color::Rgb(0xff, 0xff, 0xa5)),
        ("AnsiBrightBlue", Color::Rgb(0xd6, 0xac, 0xff)),
        ("AnsiBrightMagenta", Color::Rgb(0xff, 0x92, 0xdf)),
        ("AnsiBrightCyan", Color::Rgb(0xa4, 0xff, 0xff)),
        ("AnsiBrightWhite", Color::Rgb(0xff, 0xff, 0xff)),
    ];

    /// The two Dracula variants the page names. Alucard waits with the light
    /// theme.
    const DRACULA_VARIANTS: [&str; 2] = ["Dracula Classic", "Alucard Classic"];

    /// The family's two near-grey word colours.
    const DRACULA_WORD_GREYS: [Color; 2] = [dracula::FOREGROUND, dracula::BRIGHT_WHITE];

    /// The family's one dimmed grey: the comment.
    const DRACULA_MUTED_GREYS: [Color; 1] = [dracula::COMMENT];

    /// The functional colours the page publishes for UI, and marks not for
    /// terminal applications. No slot spends one.
    const DRACULA_FUNCTIONAL: [(&str, Color); 5] = [
        ("Functional Red", Color::Rgb(0xde, 0x57, 0x35)),
        ("Functional Orange", Color::Rgb(0xa3, 0x95, 0x14)),
        ("Functional Green", Color::Rgb(0x08, 0x91, 0x08)),
        ("Functional Cyan", Color::Rgb(0x00, 0x81, 0xd6)),
        ("Functional Purple", Color::Rgb(0x81, 0x5c, 0xd6)),
    ];

    /// The five UI backgrounds the page lists, one of them twice. None is
    /// spent: the theme paints no background of its own.
    const DRACULA_UI_BACKGROUNDS: [(&str, Color); 5] = [
        (
            "Floating interactive elements",
            Color::Rgb(0x34, 0x37, 0x46),
        ),
        ("Background Lighter", Color::Rgb(0x42, 0x44, 0x50)),
        ("Background Light", Color::Rgb(0x34, 0x37, 0x46)),
        ("Background Dark", Color::Rgb(0x21, 0x22, 0x2c)),
        ("Background Darker", Color::Rgb(0x19, 0x1a, 0x21)),
    ];

    /// The page's opaque fallback for the current line: implementation
    /// guidance, not a palette colour.
    const DRACULA_CURRENT_LINE_FALLBACK: Color = Color::Rgb(0x35, 0x37, 0x47);

    /// The colour a Dracula role the doc names holds.
    fn dracula_colour(role: &str) -> Color {
        match role {
            "background" => dracula::BACKGROUND,
            "current line" => dracula::CURRENT_LINE,
            "foreground" => dracula::FOREGROUND,
            "comment" => dracula::COMMENT,
            "red" => dracula::RED,
            "orange" => dracula::ORANGE,
            "yellow" => dracula::YELLOW,
            "green" => dracula::GREEN,
            "cyan" => dracula::CYAN,
            "purple" => dracula::PURPLE,
            "ANSI bright white" => dracula::BRIGHT_WHITE,
            "pink" => dracula::PINK,
            other => panic!("{other} is not a Dracula colour the slots spend"),
        }
    }

    /// The colour rows of the Dracula slot table: the slot, the family's
    /// role, and the value.
    fn dracula_rows() -> Vec<(String, String, Color)> {
        let section = section(PALETTES, "Dracula");
        family_rows(&section)
            .into_iter()
            .filter_map(|(slot, cells)| {
                let role = cells
                    .first()?
                    .split(" —")
                    .next()
                    .expect("a role")
                    .trim()
                    .to_string();
                let colour = cells
                    .iter()
                    .skip(1)
                    .find_map(|cell| hexes_in(cell).first().copied())?;
                Some((slot, role, colour))
            })
            .collect()
    }

    /// The rows of a page table: the role the page names and the value it
    /// gives it.
    fn page_rows(table: &str) -> Vec<(String, Color)> {
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

    /// One `###` subsection of a family section, by its heading.
    fn subsection(section: &str, heading: &str) -> String {
        let rest = section
            .split_once(&format!("### {heading}"))
            .unwrap_or_else(|| panic!("the section has no {heading} subsection"))
            .1;
        rest.split_once("\n### ")
            .map_or(rest, |(head, _)| head)
            .to_string()
    }

    /// The number a word of the doc states, up to thirty-nine.
    fn number_word(word: &str) -> Option<usize> {
        const UNITS: [&str; 20] = [
            "zero",
            "one",
            "two",
            "three",
            "four",
            "five",
            "six",
            "seven",
            "eight",
            "nine",
            "ten",
            "eleven",
            "twelve",
            "thirteen",
            "fourteen",
            "fifteen",
            "sixteen",
            "seventeen",
            "eighteen",
            "nineteen",
        ];
        const TENS: [&str; 2] = ["twenty", "thirty"];
        if let Some(unit) = UNITS.iter().position(|name| *name == word) {
            return Some(unit);
        }
        if let Some(ten) = TENS.iter().position(|name| *name == word) {
            return Some((ten + 2) * 10);
        }
        let (ten, unit) = word.split_once('-')?;
        let ten = TENS.iter().position(|name| *name == ten)?;
        let unit = UNITS.iter().position(|name| *name == unit)?;
        Some((ten + 2) * 10 + unit)
    }

    /// The number words a sentence states, in order.
    fn number_words_in(sentence: &str) -> Vec<usize> {
        sentence
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .filter_map(|word| number_word(&word.to_lowercase()))
            .collect()
    }

    /// Push the sentences of one paragraph, split on `.`, `!` and `?`.
    ///
    /// The terminators are followed by markdown's closing markers and then a
    /// space, so the markers are skipped before the sentence is cut.
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

    /// The prose sentences of a section: tables and headings are data the
    /// table tests read, not count statements.
    fn prose_sentences(section: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut paragraph = String::new();
        for line in section.lines() {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with('|')
                || line.starts_with('#')
                || line.starts_with("```")
            {
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

    /// The sentences of the family sections that state a number and no count.
    /// Each is named by a phrase of its sentence. Three kinds: `one` as a
    /// pronoun, a channel gap between two colours, and a contrast ratio.
    const NOT_COUNTS: [&str; 21] = [
        // `one` as a pronoun.
        "stays one: it is not a candidate",
        "no slot spends one",
        "25 apart in latte, 41 in frappé",
        // Channel gaps.
        "its channels are 25 apart",
        "foreground's three channels are 6 apart",
        "It is 11 channels apart",
        "nord7 is 45 channels apart",
        "is a frost blue, 78 channels apart",
        // Contrast ratios.
        "2.9:1 in frappé and 3.4:1 in mocha",
        "5.6:1 in frappé and 8.1:1 in mocha",
        "14.2:1 on the background, a step above",
        "The comment reads at 3.0:1 on the background",
        "at 3.0:1 it is at the floor",
        "reads at 3.0:1 on the base, 3.6:1",
        "with the base colour on it, 5.9:1",
        "on the family's own base it is faint: nord3 reads at 1.7:1",
        "nord3 reads at 1.7:1 on the base, which is a border",
        "nord4, which reads at 9.2:1",
        "it reads at 4.6:1 on the base",
        "nord2 is a border grade — 1.4:1",
        "nord11 reads at 3.1:1",
    ];

    /// Whether a sentence states a count: a number word or a digit outside a
    /// code span, unless the sentence is listed in [`NOT_COUNTS`]. A sentence
    /// is a count by its number, not by a noun that sits near it.
    fn states_a_count(sentence: &str) -> bool {
        if NOT_COUNTS.iter().any(|phrase| sentence.contains(phrase)) {
            return false;
        }
        let prose = without_code_spans(sentence);
        let digit = prose
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| !word.is_empty() && word.chars().all(|c| c.is_ascii_digit()));
        digit || !number_words_in(&prose).is_empty()
    }

    /// The text of a sentence outside its code spans: a hex colour or a
    /// setting in backticks is not a count.
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

    /// The counts the family sections state, with the numbers each one should
    /// state, in the sentence's order.
    fn family_count_claims() -> Vec<(&'static str, &'static str, Vec<usize>)> {
        let catppuccin = section(PALETTES, "Catppuccin");
        let (mocha_spent, mocha_total) = spent_of_page(
            &subsection(&catppuccin, "Mocha, as the page lists it"),
            Preset::Catppuccin,
        );
        let dracula = section(PALETTES, "Dracula");
        let (colour_spent, colour_total) = spent_of_page(
            &subsection(&dracula, "Dracula Classic, as the page lists it"),
            Preset::Dracula,
        );
        let ansi_restating = DRACULA_ANSI
            .iter()
            .filter(|(_, colour)| DRACULA_COLOURS.iter().any(|(_, other)| other == colour))
            .count();
        let ansi_own = DRACULA_ANSI.len() - ansi_restating;
        let mut ansi_roles: Vec<String> = dracula_rows()
            .into_iter()
            .map(|(_, role, _)| role)
            .filter(|role| role.starts_with("ANSI"))
            .collect();
        ansi_roles.sort_unstable();
        ansi_roles.dedup();
        let ansi_spent = ansi_roles.len();
        let page_spent = colour_spent + ansi_spent;
        vec![
            (
                "Catppuccin",
                "Four flavours",
                vec![CATPPUCCIN_FLAVOURS.len()],
            ),
            (
                "Catppuccin",
                "Catppuccin's three word greys are spent",
                vec![CATPPUCCIN_WORD_GREYS.len()],
            ),
            (
                "Catppuccin",
                "records the three dark flavours",
                vec![CATPPUCCIN_FLAVOURS.len() - CATPPUCCIN_LIGHT_FLAVOURS.len()],
            ),
            (
                "Catppuccin",
                "The slots spend",
                vec![mocha_spent, mocha_total],
            ),
            ("Dracula", "Two variants", vec![DRACULA_VARIANTS.len()]),
            (
                "Dracula",
                "two near-grey word colours",
                vec![DRACULA_WORD_GREYS.len()],
            ),
            (
                "Dracula",
                "besides the two whites",
                vec![DRACULA_WORD_GREYS.len(), DRACULA_MUTED_GREYS.len()],
            ),
            (
                "Dracula",
                "The slots spend",
                vec![colour_spent, colour_total],
            ),
            (
                "Dracula",
                "The page publishes a second palette",
                vec![DRACULA_ANSI.len(), ansi_restating, ansi_own],
            ),
            (
                "Dracula",
                "The theme takes one of the sixteen",
                vec![
                    ansi_spent,
                    DRACULA_ANSI.len(),
                    DRACULA_ANSI.len() - ansi_spent,
                ],
            ),
            (
                "Dracula",
                "With the color palette's eleven",
                vec![colour_spent, page_spent],
            ),
            (
                "Nord",
                "The palette has sixteen colours",
                vec![NORD_PAGE.len(), NORD_GROUPS.len()],
            ),
            (
                "Nord",
                "The snow storm's three greys",
                vec![NORD_WORD_GREYS.len()],
            ),
        ]
    }

    /// How many of a page table's values a preset spends, and how many the
    /// table lists.
    fn spent_of_page(table: &str, preset: Preset) -> (usize, usize) {
        let rows = page_rows(table);
        let theme = Theme::preset(preset);
        let mut palette = Vec::new();
        for slot in Slot::ALL {
            let style = theme.slot(slot);
            palette.extend(style.fg);
            palette.extend(style.bg);
        }
        let spent = rows
            .iter()
            .filter(|(_, colour)| palette.contains(colour))
            .count();
        (spent, rows.len())
    }

    #[test]
    fn the_dracula_page_tables_record_every_value_the_page_lists() {
        // The Catppuccin unit's mocha table records the page whole; Dracula's
        // page publishes two palettes plus its functional colours and UI
        // backgrounds, and all four are recorded in the page's order, so a
        // reader can count them.
        let section = section(PALETTES, "Dracula");
        let tables: [(&str, &[(&str, Color)]); 4] = [
            ("Dracula Classic, as the page lists it", &DRACULA_COLOURS),
            ("The ANSI palette, as the page lists it", &DRACULA_ANSI),
            (
                "The functional colours, as the page lists them",
                &DRACULA_FUNCTIONAL,
            ),
            (
                "The UI backgrounds, as the page lists them",
                &DRACULA_UI_BACKGROUNDS,
            ),
        ];
        for (heading, expected) in tables {
            let rows = page_rows(&subsection(&section, heading));
            assert_eq!(
                rows.len(),
                expected.len(),
                "the doc records {} values for {heading} and the page lists {}",
                rows.len(),
                expected.len()
            );
            for ((role, colour), (expected_role, expected_colour)) in
                rows.iter().zip(expected.iter())
            {
                assert_eq!(role, expected_role, "{heading}: the page's order moved");
                assert_eq!(
                    colour, expected_colour,
                    "{role} holds a value the page does not list"
                );
            }
        }
    }

    #[test]
    fn the_dracula_functional_colours_and_backgrounds_are_not_spent() {
        // The page marks the functional colours not for terminal
        // applications, and the UI backgrounds are backgrounds: neither is a
        // palette colour, and no slot may spend one.
        let theme = Theme::preset(Preset::Dracula);
        for (role, colour) in DRACULA_FUNCTIONAL
            .iter()
            .chain(DRACULA_UI_BACKGROUNDS.iter())
        {
            for slot in Slot::ALL {
                let style = theme.slot(slot);
                assert_ne!(
                    style.fg,
                    Some(*colour),
                    "{} spends the out-of-palette {role}",
                    slot.name()
                );
                assert_ne!(
                    style.bg,
                    Some(*colour),
                    "{} spends the out-of-palette {role}",
                    slot.name()
                );
            }
        }
    }

    #[test]
    fn the_dracula_current_line_fallback_is_guidance_not_a_palette_colour() {
        // The page lists an opaque fallback for the current line and says it
        // is implementation guidance, not a token. The doc records it as
        // such; it is not in any page table, and no slot spends it.
        let section = section(PALETTES, "Dracula");
        let fallback = subsection(&section, "The current-line fallback, as the page lists it");
        let paragraph = fallback
            .split_once("The page names an opaque fallback")
            .expect("the doc no longer records the fallback's sentence")
            .1;
        let paragraph = paragraph
            .split("\n\n")
            .next()
            .expect("the fallback sentence has a paragraph")
            .replace('\n', " ");
        assert_eq!(
            hexes_in(&paragraph),
            vec![DRACULA_CURRENT_LINE_FALLBACK],
            "the doc does not record the page's current-line fallback"
        );
        assert!(
            paragraph.contains("out-of-palette implementation guidance"),
            "the doc does not mark the fallback as out-of-palette guidance"
        );
        let listed = [
            "Dracula Classic, as the page lists it",
            "The ANSI palette, as the page lists it",
            "The functional colours, as the page lists them",
            "The UI backgrounds, as the page lists them",
        ];
        for heading in listed {
            for (role, colour) in page_rows(&subsection(&section, heading)) {
                assert_ne!(
                    colour, DRACULA_CURRENT_LINE_FALLBACK,
                    "{role} records the fallback as a palette colour"
                );
            }
        }
        let theme = Theme::preset(Preset::Dracula);
        for slot in Slot::ALL {
            let style = theme.slot(slot);
            assert_ne!(
                style.fg,
                Some(DRACULA_CURRENT_LINE_FALLBACK),
                "{} spends the current-line fallback",
                slot.name()
            );
            assert_ne!(
                style.bg,
                Some(DRACULA_CURRENT_LINE_FALLBACK),
                "{} spends the current-line fallback",
                slot.name()
            );
        }
    }

    #[test]
    fn the_dracula_palette_is_the_one_the_doc_records() {
        // The doc is the source of truth: the code is checked against its
        // table, so the two cannot drift apart silently.
        let theme = Theme::preset(Preset::Dracula);
        let rows = dracula_rows();
        assert_eq!(
            rows.len(),
            Slot::ALL.len(),
            "the doc's Dracula table does not hold every slot"
        );
        for (name, _, colour) in rows {
            let slot = slot_named(&name);
            let style = theme.slot(slot);
            if is_a_background(slot) {
                assert_eq!(
                    style.fg, None,
                    "{name} paints a foreground the doc does not record"
                );
                assert_eq!(
                    style.bg,
                    Some(colour),
                    "{name} paints a background the doc does not record"
                );
            } else if slot == Slot::TextSelected {
                // The doc records the accent; the selection puts the base
                // on it, the pair the palette records.
                assert_eq!(
                    style.bg,
                    Some(colour),
                    "{name} paints a background the doc does not record"
                );
                assert_eq!(
                    style.fg,
                    Some(dracula::BACKGROUND),
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
    fn the_dracula_roles_name_the_values_the_table_states() {
        // The value beside a role is the value the page gives that role:
        // the table's role cell and its value cell agree with each other.
        let rows = dracula_rows();
        for (name, role, colour) in &rows {
            if is_a_background(slot_named(name)) {
                continue;
            }
            assert_eq!(
                dracula_colour(role),
                *colour,
                "{name}'s role {role} holds another value"
            );
        }
        // The ANSI palette's sixteen values: the theme takes one, the bright
        // white, and the doc says so. A second ANSI role would make the count
        // it states wrong.
        let mut ansi: Vec<&str> = rows
            .iter()
            .map(|(_, role, _)| role.as_str())
            .filter(|role| role.starts_with("ANSI"))
            .collect();
        ansi.sort_unstable();
        ansi.dedup();
        assert_eq!(
            ansi,
            ["ANSI bright white"],
            "the slots take more than one value of the ANSI palette"
        );
    }

    #[test]
    fn the_contrast_figures_of_the_dracula_preset_are_right() {
        // Every figure the doc states is recomputed from the code's own
        // palette, the way the dark, Nord and Catppuccin presets' are.
        let theme = Theme::preset(Preset::Dracula);
        let rows = family_ratios(&section(PALETTES, "Dracula"));
        assert_eq!(
            rows.len(),
            words(),
            "the doc's Dracula table states {} figures",
            rows.len()
        );
        let mut checked = 0;
        for (name, ratios) in rows {
            let slot = slot_named(&name);
            let style = theme.slot(slot);
            let foreground = style.fg.expect("a Dracula slot has a foreground");
            let behind = style.bg.unwrap_or(dracula::BACKGROUND);
            let computed = contrast(foreground, behind);
            assert!(
                (computed - ratios[0]).abs() < 0.05,
                "{name} reads at {computed:.2}:1, and the doc says {}:1",
                ratios[0]
            );
            checked += 1;
        }
        assert_eq!(checked, words(), "Dracula states {checked} figures");
    }

    #[test]
    fn the_dracula_preset_reads_on_its_background() {
        // Every word clears the floor the doc states; only the border and
        // the index are lines. The comment is the family's one dimmed grey,
        // and it reads at 3.0:1 on the base — the floor, not below it.
        let theme = Theme::preset(Preset::Dracula);
        for slot in Slot::ALL {
            if matches!(slot, Slot::ChromeBorder | Slot::ChromeTabIndex) {
                continue;
            }
            let style = theme.slot(slot);
            let Some(foreground) = style.fg else {
                continue;
            };
            let behind = style.bg.unwrap_or(dracula::BACKGROUND);
            let ratio = contrast(foreground, behind);
            assert!(
                ratio >= 3.0,
                "{} reads at {ratio:.1}:1, and a word needs 3:1",
                slot.name()
            );
        }
    }

    #[test]
    fn the_dracula_preset_invents_no_colour() {
        // Rule 9: a theme is a palette, not a mood. Every colour the preset
        // paints is a value the page lists, in one of its two palettes, or
        // one of the two backgrounds the mapping derives from the page's own
        // values, so a slot cannot be filled from memory.
        let section = section(PALETTES, "Dracula");
        let colours = page_rows(&subsection(
            &section,
            "Dracula Classic, as the page lists it",
        ));
        let ansi = page_rows(&subsection(
            &section,
            "The ANSI palette, as the page lists it",
        ));
        let page: Vec<Color> = colours
            .iter()
            .chain(ansi.iter())
            .map(|(_, colour)| *colour)
            .collect();
        assert_eq!(
            page.len(),
            DRACULA_COLOURS.len() + DRACULA_ANSI.len(),
            "the doc does not record both of the page's palettes"
        );
        let value = |role: &str| {
            colours
                .iter()
                .find(|(name, _)| name == role)
                .unwrap_or_else(|| panic!("the page has no {role}"))
                .1
        };
        let surface = surface_of(value("background"));
        let tint = selection_of(surface, value("purple"));
        let theme = Theme::preset(Preset::Dracula);
        for slot in Slot::ALL {
            let style = theme.slot(slot);
            let derived = match slot {
                Slot::ChromeSurface => Some(surface),
                Slot::TextSelectedBg => Some(tint),
                _ => None,
            };
            if let Some(derived) = derived {
                assert_eq!(
                    style.bg,
                    Some(derived),
                    "{} is not the background the mapping derives",
                    slot.name()
                );
                continue;
            }
            for colour in [style.fg, style.bg].into_iter().flatten() {
                assert!(
                    page.contains(&colour),
                    "{} paints {colour:?}, which the page does not list",
                    slot.name()
                );
            }
        }
    }

    #[test]
    fn a_stated_number_is_a_count_unless_it_is_exempt() {
        // The old guard needed a count noun beside the number, so "Nine hues"
        // passed. A number is a count by itself; a pronoun or a figure is
        // exempt by name, and a code span is not read.
        assert!(states_a_count("Nine hues sit in the family."));
        assert!(states_a_count("The family states 9 things."));
        assert!(!states_a_count(
            "The overlay stays one: it is not a candidate."
        ));
        assert!(!states_a_count("The notes read `#1e1e2e` in the base."));
    }

    #[test]
    fn every_count_a_family_section_states_is_recomputed() {
        // The Catppuccin unit's low finding, and then Dracula's: a count the
        // doc states and no test reads is drift waiting to happen. Every
        // count sentence in every family section is recomputed here, and a
        // count sentence with no claim fails the gate — that is what keeps
        // the class from recurring.
        let claims = family_count_claims();
        let mut covered = Vec::new();
        for (heading, anchor, expected) in &claims {
            let section = section(PALETTES, heading);
            let sentence = prose_sentences(&section)
                .into_iter()
                .find(|sentence| sentence.contains(anchor))
                .unwrap_or_else(|| panic!("{heading} no longer states {anchor}"));
            let stated = number_words_in(&sentence);
            assert_eq!(
                &stated, expected,
                "{heading} says {stated:?} in \"{sentence}\", and the recompute is {expected:?}"
            );
            covered.push((heading.to_string(), sentence));
        }
        let mut checked = 0;
        for heading in FAMILIES {
            for sentence in prose_sentences(&section(PALETTES, heading)) {
                if !states_a_count(&sentence) {
                    continue;
                }
                assert!(
                    covered
                        .iter()
                        .any(|(covered_heading, covered)| covered_heading == heading
                            && covered == &sentence),
                    "{heading} states a count no test recomputes: \"{sentence}\""
                );
                checked += 1;
            }
        }
        assert_eq!(
            checked,
            claims.len(),
            "the guard reads {checked} count sentences and the claims name {}",
            claims.len()
        );
    }

    #[test]
    fn the_code_comments_count_the_catppuccin_values_right() {
        // The count that drifted was written in two comments as well as the
        // doc. The doc's count is recomputed from the page table by the test
        // above; these two are read from the source and held to the same
        // number, so the two cannot disagree with the page either.
        //
        // The anchors are assembled from parts: a phrase written whole here
        // would be found in this test's own source, and the check would pass
        // on its own text.
        let (spent, _) = spent_of_page(
            &subsection(
                &section(PALETTES, "Catppuccin"),
                "Mocha, as the page lists it",
            ),
            Preset::Catppuccin,
        );
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/theme.rs"))
            .expect("the theme's own source");
        for anchor in [
            [" Catppuccin mocha", " values that the slots spend"].concat(),
            [" of the twenty-six;", " the rest are kept"].concat(),
        ] {
            let at = source
                .find(&anchor)
                .unwrap_or_else(|| panic!("the source no longer says {anchor}"));
            let word = source[..at]
                .split_whitespace()
                .last()
                .expect("a number word before the anchor");
            assert_eq!(
                number_word(word),
                Some(spent),
                "a comment says {word}, and the page table says the slots spend {spent}"
            );
        }
    }

    #[test]
    fn the_code_comments_count_the_dracula_values_right() {
        // Dracula's mod comment states the same counts as its doc section:
        // the values the slots spend, and the two page palettes. They are
        // read from the source and held to the recompute, so a comment
        // cannot drift from the page.
        //
        // The anchors are assembled from parts: a phrase written whole here
        // would be found in this test's own source.
        let section = section(PALETTES, "Dracula");
        let (colour_spent, colour_total) = spent_of_page(
            &subsection(&section, "Dracula Classic, as the page lists it"),
            Preset::Dracula,
        );
        let mut ansi_roles: Vec<String> = dracula_rows()
            .into_iter()
            .map(|(_, role, _)| role)
            .filter(|role| role.starts_with("ANSI"))
            .collect();
        ansi_roles.sort_unstable();
        ansi_roles.dedup();
        let spent = colour_spent + ansi_roles.len();
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/theme.rs"))
            .expect("the theme's own source");
        let words = source
            .replace("///", " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let checks = [
            ([" Dracula values that", " the slots spend"].concat(), spent),
            (
                [" values in its color palette", " and sixteen in"].concat(),
                colour_total,
            ),
            (
                [" in its ANSI one;", " the mapping in"].concat(),
                DRACULA_ANSI.len(),
            ),
            (
                [
                    " plays, and why the rest",
                    " are kept for the record. The variant",
                ]
                .concat(),
                spent,
            ),
        ];
        for (anchor, expected) in checks {
            let at = words
                .find(&anchor)
                .unwrap_or_else(|| panic!("the source no longer says {anchor}"));
            let word = words[..at]
                .split_whitespace()
                .last()
                .expect("a number word before the anchor");
            assert_eq!(
                number_word(word),
                Some(expected),
                "a comment says {word}, and the page says {expected}"
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
        let deferred = ["nord-light", "latte", "frappé", "macchiato", "alucard"];
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
    fn the_presets_differ_in_paint() {
        // The presets are not the same palette under another name. The whole
        // theme is not compared: its `preset` field always differs, so the
        // paint is compared slot by slot and role by role.
        let paint = |preset: Preset| {
            let theme = Theme::preset(preset);
            (Slot::ALL.map(|slot| theme.slot(slot)), theme.document())
        };
        for (index, one) in Preset::ALL.iter().enumerate() {
            for other in &Preset::ALL[index + 1..] {
                assert_ne!(
                    paint(*one),
                    paint(*other),
                    "{} and {} paint the same",
                    one.name(),
                    other.name()
                );
            }
        }
    }

    #[test]
    fn a_selected_line_fills_the_pane_by_the_columns_it_draws() {
        // A tab draws no column, so the whole-string width would leave the
        // bar one column short.
        let theme = Theme::preset(Preset::Dark);
        let line = selected_line(Line::from("a\tb"), 4, &theme);
        let drawn: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(text_width(&drawn), 4);
    }

    #[test]
    fn every_preset_names_itself_and_its_kind() {
        let labels: Vec<&str> = Preset::ALL.iter().map(|preset| preset.label()).collect();
        assert_eq!(
            labels,
            ["Dark", "Nord", "Catppuccin", "Dracula", "Terminal"]
        );
        for preset in Preset::ALL {
            assert_eq!(Theme::preset(preset).preset, preset, "{}", preset.name());
            let kind = match preset {
                Preset::Terminal => PresetKind::Terminal,
                _ => PresetKind::Palette,
            };
            assert_eq!(preset.kind(), kind, "{}", preset.name());
            assert!(!kind.label().is_empty(), "{}", preset.name());
        }
        // The two kinds say different things.
        assert_ne!(PresetKind::Palette.label(), PresetKind::Terminal.label());
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
            (Slot::ChromeSurface, Modifier::empty()),
            (Slot::TextNormal, Modifier::empty()),
            (Slot::TextDim, Modifier::DIM),
            (Slot::TextStrong, Modifier::BOLD),
            (Slot::TextSelected, Modifier::REVERSED),
            (Slot::TextSelectedBg, Modifier::REVERSED),
            (Slot::SemanticOk, Modifier::ITALIC),
            (Slot::SemanticWarn, Modifier::UNDERLINED),
            (
                Slot::SemanticError,
                Modifier::BOLD | Modifier::REVERSED | Modifier::UNDERLINED,
            ),
            (Slot::SemanticInfo, Modifier::BOLD | Modifier::DIM),
            (Slot::SemanticHuman, Modifier::BOLD | Modifier::UNDERLINED),
            (Slot::SemanticAgent, Modifier::BOLD | Modifier::ITALIC),
            (
                Slot::RoleOrchestrator,
                Modifier::ITALIC | Modifier::UNDERLINED,
            ),
            (Slot::RoleRunner, Modifier::DIM | Modifier::ITALIC),
            (Slot::RoleReviewer, Modifier::UNDERLINED | Modifier::DIM),
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

    /// The colour a document role's side paints in a preset.
    fn document(preset: Preset, role: DocRole, side: DocSide) -> Color {
        document_palette(preset).paint(role, side)
    }

    /// The presets that paint colour: every one but the terminal.
    fn colour_presets() -> Vec<Preset> {
        Preset::ALL
            .into_iter()
            .filter(|preset| *preset != Preset::Terminal)
            .collect()
    }

    /// The background a preset's document figures are measured against.
    fn document_base(preset: Preset) -> Color {
        match preset {
            Preset::Dark => dark::BACKGROUND,
            Preset::Nord => nord::NORD0,
            Preset::Catppuccin => catppuccin::BASE,
            Preset::Dracula => dracula::BACKGROUND,
            Preset::Terminal => Color::Reset,
        }
    }

    #[test]
    fn every_markdown_field_paints_its_document_role() {
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let markdown = theme.markdown_theme();
            let fields = engine_fields(&markdown);
            assert_eq!(
                fields.len(),
                ENGINE_ROLES.len(),
                "{} leaves a field outside the doc's list",
                preset.name()
            );
            for ((field, actual, background), (expected_field, role, side, expected_background)) in
                fields.iter().zip(ENGINE_ROLES.iter())
            {
                assert_eq!(field, expected_field, "the two field lists disagree");
                assert_eq!(
                    background, expected_background,
                    "{field} paints a side the doc does not name"
                );
                assert_eq!(
                    *actual,
                    document(preset, *role, *side),
                    "{} paints {field} from another role than {} {}",
                    preset.name(),
                    role.name(),
                    role.side_name(*side)
                );
            }
        }
    }

    #[test]
    fn the_document_has_twenty_seven_roles() {
        let names: Vec<&str> = DocRole::ALL.iter().map(|role| role.name()).collect();
        assert_eq!(
            names,
            vec![
                "document.heading.section",
                "document.heading.item",
                "document.heading.finding",
                "document.text",
                "document.list.1",
                "document.list.2",
                "document.list.3",
                "document.quote",
                "document.alert.note",
                "document.alert.tip",
                "document.alert.important",
                "document.alert.warning",
                "document.alert.caution",
                "document.link",
                "document.code",
                "document.rule",
                "document.selection",
                "document.search",
                "document.task",
                "document.syntax.keyword",
                "document.syntax.string",
                "document.syntax.number",
                "document.syntax.function",
                "document.syntax.type",
                "document.syntax.comment",
                "document.syntax.variable",
                "document.syntax.tag",
            ]
        );
        assert_eq!(
            DocRole::ALL.len(),
            27,
            "the document has twenty-seven roles"
        );
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "two document roles share a name");
        for role in DocRole::ALL {
            assert!(
                role.name().starts_with("document."),
                "{} is not a document role name",
                role.name()
            );
        }
    }

    #[test]
    fn every_document_role_is_painted_by_a_field() {
        // A role that names no pair has one colour, and a field that took its
        // second side would paint the first and pass every other pin
        // unobserved: `paint` ignores the side, the dead-side loop below skips
        // it, and the doc's side column says the same either way.
        for (field, role, side, _) in ENGINE_ROLES {
            assert!(
                !role.is_syntax(),
                "{field} paints {}, which the engine never reads",
                role.name()
            );
            assert!(
                role.carries_a_pair() || side == DocSide::One,
                "{field} takes the {side:?} side of {}, which names no pair",
                role.name()
            );
        }
        // The syntax roles are the code's tokens, which the engine paints with
        // syntect's theme: the detail pane pairs it, and no engine field names
        // them.
        for role in DocRole::ALL.iter().filter(|role| !role.is_syntax()) {
            assert!(
                ENGINE_ROLES.iter().any(|(_, used, _, _)| used == role),
                "{} is a role no engine field uses",
                role.name()
            );
            for side in [DocSide::One, DocSide::Two] {
                if side == DocSide::Two && !role.carries_a_pair() {
                    continue;
                }
                assert!(
                    ENGINE_ROLES
                        .iter()
                        .any(|(_, used, used_side, _)| used == role && *used_side == side),
                    "{}'s {} side is a colour no engine field uses",
                    role.name(),
                    role.side_name(side)
                );
            }
        }
    }

    #[test]
    fn the_six_pair_roles_are_the_ones_the_doc_names() {
        // A role that names a pair carries two colours; a role that names one
        // carries the same colour on both sides.
        let pairs: Vec<&str> = DocRole::ALL
            .iter()
            .filter(|role| role.carries_a_pair())
            .map(|role| role.name())
            .collect();
        assert_eq!(
            pairs,
            vec![
                "document.text",
                "document.code",
                "document.rule",
                "document.selection",
                "document.search",
                "document.task",
            ]
        );
        for preset in colour_presets() {
            for role in DocRole::ALL {
                let one = document(preset, role, DocSide::One);
                let two = document(preset, role, DocSide::Two);
                if role.carries_a_pair() {
                    assert_ne!(
                        one,
                        two,
                        "{}'s {} names a pair and paints one colour",
                        preset.name(),
                        role.name()
                    );
                } else {
                    assert_eq!(
                        one,
                        two,
                        "{}'s {} names no pair and paints two colours",
                        preset.name(),
                        role.name()
                    );
                }
            }
        }
    }

    #[test]
    fn the_document_headings_are_three_and_apart() {
        // The three levels the projection writes: `#`, `##` and `###`.
        for preset in colour_presets() {
            let palette = document_palette(preset);
            let headings = [
                palette.heading_section,
                palette.heading_item,
                palette.heading_finding,
            ];
            for (index, one) in headings.iter().enumerate() {
                for other in &headings[index + 1..] {
                    assert_ne!(
                        one,
                        other,
                        "{} paints two heading levels alike",
                        preset.name()
                    );
                }
            }
            // The report's own defect: the third level was the prose's colour.
            assert_ne!(
                palette.heading_finding,
                palette.text.body,
                "{} paints the finding heading as the prose",
                preset.name()
            );
            assert_ne!(
                palette.heading_section,
                palette.text.body,
                "{} paints the section heading as the prose",
                preset.name()
            );
        }
    }

    #[test]
    fn the_document_list_depths_are_apart() {
        for preset in colour_presets() {
            let palette = document_palette(preset);
            let depths = [
                palette.list_depth_1,
                palette.list_depth_2,
                palette.list_depth_3,
            ];
            for (index, one) in depths.iter().enumerate() {
                for other in &depths[index + 1..] {
                    assert_ne!(one, other, "{} paints two list depths alike", preset.name());
                }
            }
        }
    }

    #[test]
    fn the_document_alerts_keep_their_five() {
        for preset in colour_presets() {
            let palette = document_palette(preset);
            let alerts = [
                palette.alert_note,
                palette.alert_tip,
                palette.alert_important,
                palette.alert_warning,
                palette.alert_caution,
            ];
            for (index, one) in alerts.iter().enumerate() {
                for other in &alerts[index + 1..] {
                    assert_ne!(
                        one,
                        other,
                        "{} paints two alert severities alike",
                        preset.name()
                    );
                }
            }
            for alert in alerts {
                assert_ne!(
                    alert,
                    palette.link,
                    "{} paints an alert as a link",
                    preset.name()
                );
            }
        }
    }

    #[test]
    fn the_document_table_parts_are_apart() {
        for preset in colour_presets() {
            let palette = document_palette(preset);
            // The report's own defect: the border and the header rule were
            // the same colour.
            assert_ne!(
                palette.rule.line,
                palette.rule.header,
                "{} paints a table's border and its header rule alike",
                preset.name()
            );
            assert_ne!(
                palette.text.emphasis,
                palette.text.body,
                "{} paints a table's header and its cell alike",
                preset.name()
            );
            assert_ne!(
                palette.rule.line,
                palette.text.body,
                "{} paints a table's line as a word",
                preset.name()
            );
        }
    }

    /// The least channel gap two chromatic document roles may have.
    ///
    /// The gap is the largest difference of the two colours' channels, in
    /// channels of 255. The nearest pair the palette holds is Catppuccin's
    /// section heading and its important alert, 24 apart.
    const APART: u8 = 20;

    /// The largest difference between two colours' channels.
    fn channel_gap(one: Color, other: Color) -> u8 {
        let (Color::Rgb(r, g, b), Color::Rgb(x, y, z)) = (one, other) else {
            panic!("{one:?} or {other:?} is not an RGB colour");
        };
        r.abs_diff(x).max(g.abs_diff(y)).max(b.abs_diff(z))
    }

    /// The first pair of a palette's headings and alerts, and its link and
    /// section heading, that are nearer than [`APART`]: the two roles, and
    /// the channel gap between them.
    fn near_pair(palette: &DocPalette) -> Option<(&'static str, &'static str, u8)> {
        let headings = [
            ("section heading", palette.heading_section),
            ("item heading", palette.heading_item),
            ("finding heading", palette.heading_finding),
        ];
        let alerts = [
            ("note", palette.alert_note),
            ("tip", palette.alert_tip),
            ("important", palette.alert_important),
            ("warning", palette.alert_warning),
            ("caution", palette.alert_caution),
        ];
        for (heading, colour) in headings {
            for (alert, other) in alerts {
                let gap = channel_gap(colour, other);
                if gap < APART {
                    return Some((heading, alert, gap));
                }
            }
        }
        let gap = channel_gap(palette.link, palette.heading_section);
        (gap < APART).then_some(("link", "section heading", gap))
    }

    #[test]
    fn the_headings_and_the_alerts_are_apart() {
        // A heading level and an alert severity are different jobs, so the
        // palette paints them apart, and the section heading and the link are
        // apart too: a link is not the heading it sits under.
        for preset in colour_presets() {
            let palette = document_palette(preset);
            if let Some((one, other, gap)) = near_pair(&palette) {
                panic!(
                    "{} paints a {one} and a {other} {gap} channels apart",
                    preset.name()
                );
            }
        }
        // The check fails a near-duplicate: an alert note shifted by five
        // channels is the heading it sits beside, and the check names it.
        let mut palette = document_palette(Preset::Dark);
        let Color::Rgb(r, g, b) = palette.alert_note else {
            panic!("the dark note is not an RGB colour");
        };
        palette.heading_item = Color::Rgb(r.saturating_add(5), g, b);
        assert_eq!(
            near_pair(&palette),
            Some(("item heading", "note", 5)),
            "the check passed a heading five channels from a note"
        );
    }

    #[test]
    fn the_third_list_depth_is_apart_from_the_headings_and_the_alerts() {
        // The no-shared-hue rule binds the headings, the alerts and the third
        // list depth. The first and second depths are exempt, ruled on
        // 2026-11-22: the list's markers are coloured, and they may share a
        // hue with a heading or an alert.
        for preset in colour_presets() {
            let palette = document_palette(preset);
            let headings = [
                palette.heading_section,
                palette.heading_item,
                palette.heading_finding,
            ];
            let alerts = [
                palette.alert_note,
                palette.alert_tip,
                palette.alert_important,
                palette.alert_warning,
                palette.alert_caution,
            ];
            for other in headings.into_iter().chain(alerts) {
                assert!(
                    channel_gap(palette.list_depth_3, other) >= APART,
                    "{} paints the third list depth {} channels from a heading or an alert",
                    preset.name(),
                    channel_gap(palette.list_depth_3, other)
                );
            }
        }
    }

    #[test]
    fn the_syntax_roles_are_apart() {
        // Eight token classes, eight colours: a fixture's keyword, string,
        // number, function, type, comment, variable and tag never read alike.
        // The plain ink of a code block is the code role's, and no token
        // class takes it, so an unmatched scope is the only one that does.
        for preset in colour_presets() {
            let palette = document_palette(preset);
            let syntax = [
                palette.syntax.keyword,
                palette.syntax.string,
                palette.syntax.number,
                palette.syntax.function,
                palette.syntax.type_name,
                palette.syntax.comment,
                palette.syntax.variable,
                palette.syntax.tag,
            ];
            for (index, one) in syntax.iter().enumerate() {
                assert_ne!(
                    *one,
                    palette.code.text,
                    "{} paints a token class as the plain code ink",
                    preset.name()
                );
                for other in &syntax[index + 1..] {
                    assert_ne!(
                        one,
                        other,
                        "{} paints two token classes alike",
                        preset.name()
                    );
                }
            }
        }
    }

    /// The role sides that paint a word: the headings, the prose, the lists,
    /// the alerts, the link, the code's ink, the task's boxes and the tokens.
    fn document_word_sides() -> Vec<(DocRole, DocSide)> {
        vec![
            (DocRole::HeadingSection, DocSide::One),
            (DocRole::HeadingItem, DocSide::One),
            (DocRole::HeadingFinding, DocSide::One),
            (DocRole::Text, DocSide::One),
            (DocRole::Text, DocSide::Two),
            (DocRole::ListDepth1, DocSide::One),
            (DocRole::ListDepth2, DocSide::One),
            (DocRole::ListDepth3, DocSide::One),
            (DocRole::Quote, DocSide::One),
            (DocRole::AlertNote, DocSide::One),
            (DocRole::AlertTip, DocSide::One),
            (DocRole::AlertImportant, DocSide::One),
            (DocRole::AlertWarning, DocSide::One),
            (DocRole::AlertCaution, DocSide::One),
            (DocRole::Link, DocSide::One),
            (DocRole::Code, DocSide::One),
            (DocRole::Task, DocSide::One),
            (DocRole::Task, DocSide::Two),
            (DocRole::SyntaxKeyword, DocSide::One),
            (DocRole::SyntaxString, DocSide::One),
            (DocRole::SyntaxNumber, DocSide::One),
            (DocRole::SyntaxFunction, DocSide::One),
            (DocRole::SyntaxType, DocSide::One),
            (DocRole::SyntaxComment, DocSide::One),
            (DocRole::SyntaxVariable, DocSide::One),
            (DocRole::SyntaxTag, DocSide::One),
        ]
    }

    /// The contrast a document word needs on its background.
    ///
    /// Rule 3 of the palette doc, applied to the document's roles. The dark
    /// palette is ours and holds its words to 4.5:1, the way it holds the
    /// chrome's.
    fn word_floor(preset: Preset) -> f64 {
        if preset == Preset::Dark { 4.5 } else { 3.0 }
    }

    #[test]
    fn every_document_word_reads_on_its_background() {
        for preset in colour_presets() {
            let base = document_base(preset);
            let floor = word_floor(preset);
            for (role, side) in document_word_sides() {
                let colour = document(preset, role, side);
                let ratio = contrast(colour, base);
                assert!(
                    ratio >= floor,
                    "{}'s {} {} reads at {ratio:.1}:1, and a word needs {floor}:1",
                    preset.name(),
                    role.name(),
                    role.side_name(side)
                );
            }
        }
    }

    #[test]
    fn the_code_label_and_gutter_read_as_words() {
        // The label is the language's name and the gutter its numbers, so
        // they are words, not the frame's line: each reads at the word floor.
        for preset in colour_presets() {
            let markdown = Theme::preset(preset).markdown_theme();
            let base = document_base(preset);
            let floor = word_floor(preset);
            for (field, colour) in [
                ("code_label", markdown.code_label),
                ("code_gutter", markdown.code_gutter),
            ] {
                let ratio = contrast(colour, base);
                assert!(
                    ratio >= floor,
                    "{}'s {field} reads at {ratio:.1}:1, and a word needs {floor}:1",
                    preset.name()
                );
            }
        }
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

    /// The five fields the engine paints with a background, and the role
    /// sides they take: the search's two, the code's chip and the mark's
    /// background.
    const BACKGROUND_FIELD_ROLES: [(&str, DocRole, DocSide); 5] = [
        ("search_highlight_bg", DocRole::Search, DocSide::One),
        ("search_match_bg", DocRole::Search, DocSide::Two),
        ("inline_code_bg", DocRole::Code, DocSide::Two),
        ("latex_inline_bg", DocRole::Code, DocSide::Two),
        ("mark_bg", DocRole::Selection, DocSide::Two),
    ];

    #[test]
    fn only_the_engines_background_fields_take_a_role_background() {
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
            let mut backgrounds = Vec::new();
            for (field, colour, paints_background) in engine_fields(&theme.markdown_theme()) {
                let (_, role, side, _) = ENGINE_ROLES
                    .iter()
                    .find(|(name, _, _, _)| *name == field)
                    .unwrap_or_else(|| panic!("{field} is not in the mapping"));
                let expected = BACKGROUND_FIELD_ROLES.iter().any(
                    |(name, background_role, background_side)| {
                        *name == field && background_role == role && background_side == side
                    },
                );
                assert_eq!(
                    paints_background,
                    expected,
                    "{} paints {field} on a side the engine does not use",
                    preset.name()
                );
                assert_eq!(
                    colour,
                    document(preset, *role, *side),
                    "{} paints {field} from the wrong side of {}",
                    preset.name(),
                    role.name()
                );
                if paints_background {
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
    }

    #[test]
    fn the_search_figures_in_the_doc_are_recomputed_from_the_palettes() {
        // The doc states a hit's figure and the current match's for every
        // preset that has a background; recompute each from the document
        // palette, so a changed background leaves the figure stale here
        // rather than in the prose.
        let search = THEME
            .split_once("**The search is not offered")
            .expect("the theme doc records the search limitation")
            .1;
        assert!(
            THEME.contains("**The search is not offered in any preset.**"),
            "the theme doc no longer says the search is not offered"
        );
        let mut with_a_background = 0;
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
            if preset == Preset::Terminal {
                // The terminal preset paints no background; the doc says so.
                assert!(
                    bullet.contains("paints no background"),
                    "the search paragraph does not say why the {} preset has no hit",
                    preset.name()
                );
                continue;
            }
            let figures: Vec<f64> = bullet
                .split("reads at ")
                .skip(1)
                .filter_map(ratio_in)
                .collect();
            assert_eq!(
                figures.len(),
                2,
                "the {} search bullet states {} figures, and the doc needs the hit's and the match's",
                preset.name(),
                figures.len()
            );
            let palette = document_palette(preset);
            let body = palette.text.body;
            let hit = contrast(body, palette.search.hit);
            let current = contrast(body, palette.search.current);
            assert!(
                (hit - figures[0]).abs() < 0.05,
                "{}'s search hit reads at {hit:.2}:1, and the doc says {}:1",
                preset.name(),
                figures[0]
            );
            assert!(
                (current - figures[1]).abs() < 0.05,
                "{}'s current match reads at {current:.2}:1, and the doc says {}:1",
                preset.name(),
                figures[1]
            );
            // The ruling: the current match is the background no word reads
            // on, so the search stays un-offered.
            assert!(
                current < 3.0,
                "{}'s current match reads at {current:.2}:1, so the search could be offered",
                preset.name()
            );
            with_a_background += 1;
        }
        assert_eq!(
            with_a_background,
            colour_presets().len(),
            "the search figure covers {with_a_background} colour presets"
        );
    }

    #[test]
    fn a_mark_is_the_selection_pair() {
        // The engine paints a mark as mark_fg on mark_bg: the two sides of
        // the selection role, the pair the palette records.
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let palette = theme.document();
            let markdown = theme.markdown_theme();
            assert_eq!(
                markdown.mark_fg,
                palette.selection.foreground,
                "{} paints a mark's foreground from another role",
                preset.name()
            );
            assert_eq!(
                markdown.mark_bg,
                palette.selection.background,
                "{} paints a mark's background from another role",
                preset.name()
            );
        }
        for preset in colour_presets() {
            let palette = document_palette(preset);
            let ratio = contrast(palette.selection.foreground, palette.selection.background);
            assert!(
                ratio >= 4.5,
                "{}'s mark reads at {ratio:.1}:1, and a word needs 4.5:1",
                preset.name()
            );
        }
    }

    #[test]
    fn the_inline_code_chip_reads() {
        // The code role's pair: the ink on the chip behind it.
        for preset in colour_presets() {
            let palette = document_palette(preset);
            let ratio = contrast(palette.code.text, palette.code.chip);
            assert!(
                ratio >= 4.5,
                "{}'s inline code reads at {ratio:.1}:1 on its chip, and a word needs 4.5:1",
                preset.name()
            );
        }
    }

    #[test]
    fn the_terminal_markdown_theme_names_no_colour() {
        // The terminal preset has no colour in any role: every engine field
        // is Reset, which is the terminal's own paint.
        let theme = Theme::preset(Preset::Terminal);
        let palette = theme.document();
        for role in DocRole::ALL {
            for side in [DocSide::One, DocSide::Two] {
                assert_eq!(
                    palette.paint(role, side),
                    Color::Reset,
                    "{}'s {} names a colour in the terminal preset",
                    role.name(),
                    role.side_name(side)
                );
            }
        }
        for (field, colour, _) in engine_fields(&theme.markdown_theme()) {
            assert_eq!(
                colour,
                Color::Reset,
                "{field} names a colour the terminal preset does not have"
            );
        }
    }

    #[test]
    fn every_colour_preset_keeps_the_document_inside_its_slots() {
        // Every colour a document role paints is a colour one of the
        // preset's twenty-three slots already paints with: no leaf or family
        // palette leaks in, and the document and the chrome are one palette.
        let mut painted_in_all = 0;
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let mut palette = Vec::new();
            for slot in Slot::ALL {
                let style = theme.slot(slot);
                palette.extend(style.fg);
                palette.extend(style.bg);
            }
            let document = theme.document();
            let mut painted = 0;
            for role in DocRole::ALL {
                let sides: &[DocSide] = if role.carries_a_pair() {
                    &[DocSide::One, DocSide::Two]
                } else {
                    &[DocSide::One]
                };
                for side in sides {
                    let colour = document.paint(role, *side);
                    if preset == Preset::Terminal {
                        assert_eq!(
                            colour,
                            Color::Reset,
                            "{}'s {} names a colour in the terminal preset",
                            role.name(),
                            role.side_name(*side)
                        );
                        continue;
                    }
                    painted += 1;
                    assert!(
                        palette.contains(&colour),
                        "{} paints {} {} with {colour:?}, which is not one of its slots",
                        preset.name(),
                        role.name(),
                        role.side_name(*side)
                    );
                }
            }
            if preset == Preset::Terminal {
                assert_eq!(painted, 0, "the terminal preset paints {painted} roles");
            } else {
                assert_eq!(
                    painted,
                    DocRole::ALL.len() + 6,
                    "{} paints {painted} role sides and the doc names {}",
                    preset.name(),
                    DocRole::ALL.len() + 6
                );
                painted_in_all += painted;
            }
        }
        assert_eq!(
            painted_in_all,
            (DocRole::ALL.len() + 6) * colour_presets().len(),
            "no colour preset paints the document"
        );
    }

    /// The document palette's section of the palette doc.
    fn document_section() -> String {
        section(PALETTES, "The document palette")
    }

    /// The rows of the doc's document-role table: the role and the sides it
    /// names.
    fn document_role_rows() -> Vec<(DocRole, String)> {
        let mut rows = Vec::new();
        for line in document_section().lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // | role | job | sides |
            if cells.len() != 5 || !cells[1].starts_with('`') {
                continue;
            }
            let name = cells[1].trim_matches('`');
            let Some(role) = DocRole::ALL
                .iter()
                .copied()
                .find(|role| role.name() == name)
            else {
                continue;
            };
            rows.push((role, cells[3].to_string()));
        }
        rows
    }

    /// The side a name the doc writes names.
    fn doc_side(role: DocRole, name: &str) -> DocSide {
        if role.carries_a_pair() {
            [DocSide::One, DocSide::Two]
                .into_iter()
                .find(|side| role.side_name(*side) == name)
                .unwrap_or_else(|| panic!("{} has no side {name}", role.name()))
        } else {
            assert_eq!(
                name,
                "—",
                "{} names no pair and the doc says {name}",
                role.name()
            );
            DocSide::One
        }
    }

    /// One row of the doc's document-value table: the role, its side, and
    /// each colour preset's colour and figure.
    type DocumentRow = (DocRole, DocSide, Vec<(Color, f64)>);

    /// The rows of the doc's document-value table, in the doc's order.
    fn document_rows() -> Vec<DocumentRow> {
        let mut rows = Vec::new();
        for line in document_section().lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // | role | side | dark | nord | catppuccin | dracula |
            if cells.len() < 7 || !cells[1].starts_with('`') {
                continue;
            }
            let name = cells[1].trim_matches('`');
            let Some(role) = DocRole::ALL
                .iter()
                .copied()
                .find(|role| role.name() == name)
            else {
                continue;
            };
            let side = doc_side(role, cells[2]);
            let mut values = Vec::new();
            for cell in &cells[3..7] {
                let colour = hexes_in(cell)
                    .first()
                    .copied()
                    .unwrap_or_else(|| panic!("{} states no colour", role.name()));
                let ratio =
                    figure_in(cell).unwrap_or_else(|| panic!("{} states no figure", role.name()));
                values.push((colour, ratio));
            }
            rows.push((role, side, values));
        }
        rows
    }

    #[test]
    fn the_document_palette_records_every_role_and_side() {
        // The doc is the source of truth: the code is checked against its
        // tables, so the two cannot drift apart silently.
        let roles = document_role_rows();
        assert_eq!(
            roles.len(),
            DocRole::ALL.len(),
            "the doc's role table holds {} roles and the code has {}",
            roles.len(),
            DocRole::ALL.len()
        );
        for (role, sides) in &roles {
            let expected = if role.carries_a_pair() {
                format!(
                    "`{}`, `{}`",
                    role.side_name(DocSide::One),
                    role.side_name(DocSide::Two)
                )
            } else {
                "one colour".to_string()
            };
            assert_eq!(
                sides,
                &expected,
                "{}'s sides are not the ones the code names",
                role.name()
            );
        }
        let rows = document_rows();
        assert_eq!(
            rows.len(),
            DocRole::ALL.len() + 6,
            "the doc's value table holds {} rows and the code names {} role sides",
            rows.len(),
            DocRole::ALL.len() + 6
        );
        for role in DocRole::ALL {
            assert!(
                rows.iter().any(|(row, _, _)| *row == role),
                "{} has no row in the doc's value table",
                role.name()
            );
        }
        for (role, side, values) in rows {
            for (index, preset) in colour_presets().iter().enumerate() {
                assert_eq!(
                    values[index].0,
                    document(*preset, role, side),
                    "{}'s {} {} is not the value the doc records",
                    preset.name(),
                    role.name(),
                    role.side_name(side)
                );
            }
        }
    }

    /// The figure the doc states for a role side: a word's or a line's
    /// contrast against the base, the code's ink on its chip, the mark's
    /// pair, or the prose on a search background.
    fn document_figure(preset: Preset, role: DocRole, side: DocSide) -> f64 {
        let palette = document_palette(preset);
        let base = document_base(preset);
        match role {
            DocRole::Code => match side {
                DocSide::One => contrast(palette.code.text, palette.code.chip),
                DocSide::Two => contrast(palette.code.chip, base),
            },
            DocRole::Selection => match side {
                DocSide::One => {
                    contrast(palette.selection.foreground, palette.selection.background)
                }
                DocSide::Two => contrast(palette.selection.background, base),
            },
            DocRole::Search => contrast(palette.text.body, palette.paint(role, side)),
            _ => contrast(palette.paint(role, side), base),
        }
    }

    #[test]
    fn the_document_palette_counts_are_recomputed() {
        // The section's counts are read, the way the family sections' are: a
        // count the doc states and no test reads is drift waiting to happen.
        let section = document_section();
        let headings = DocRole::ALL
            .iter()
            .filter(|role| {
                matches!(
                    role,
                    DocRole::HeadingSection | DocRole::HeadingItem | DocRole::HeadingFinding
                )
            })
            .count();
        let alerts = DocRole::ALL
            .iter()
            .filter(|role| {
                matches!(
                    role,
                    DocRole::AlertNote
                        | DocRole::AlertTip
                        | DocRole::AlertImportant
                        | DocRole::AlertWarning
                        | DocRole::AlertCaution
                )
            })
            .count();
        let depths = DocRole::ALL
            .iter()
            .filter(|role| {
                matches!(
                    role,
                    DocRole::ListDepth1 | DocRole::ListDepth2 | DocRole::ListDepth3
                )
            })
            .count();
        let fixed = headings + alerts + depths;
        let rest = DocRole::ALL.len() - fixed;
        let pairs = DocRole::ALL
            .iter()
            .filter(|role| role.carries_a_pair())
            .count();
        let sentences = prose_sentences(&section);
        let sentence = |anchor: &str| {
            sentences
                .iter()
                .find(|sentence| sentence.contains(anchor))
                .unwrap_or_else(|| panic!("the document section no longer states {anchor}"))
        };
        assert_eq!(
            number_words_in(sentence("is the human's number")),
            vec![DocRole::ALL.len()],
            "the section no longer states the twenty-seven roles the code has"
        );
        assert_eq!(
            number_words_in(sentence("are fixed by the constraints")),
            vec![fixed, headings, alerts, depths],
            "the section's fixed-role counts are not the roles the code names"
        );
        assert_eq!(
            number_words_in(sentence("are the rest of the document's work")),
            vec![rest],
            "the section's count of the other roles is not the code's"
        );
        let pair_sentence = number_words_in(sentence("name a **pair**"));
        assert_eq!(
            &pair_sentence[..2],
            &[pairs, rest],
            "the section's pair count is not the code's"
        );
        assert_eq!(
            number_words_in(sentence("roles name one colour")),
            vec![DocRole::ALL.len() - pairs, 1],
            "the section's single-colour count is not the code's"
        );
    }

    #[test]
    fn the_document_contrast_figures_of_the_palettes_are_right() {
        // Every figure the doc states is recomputed from the values the same
        // table states, the way the chrome palette's figures are.
        for (role, side, values) in document_rows() {
            for (index, preset) in colour_presets().iter().enumerate() {
                let computed = document_figure(*preset, role, side);
                let stated = values[index].1;
                assert!(
                    (computed - stated).abs() < 0.05,
                    "{}'s {} {} reads at {computed:.2}:1, and the doc says {stated}:1",
                    preset.name(),
                    role.name(),
                    role.side_name(side)
                );
            }
        }
    }

    /// The rows of the engine mapping table in the theme doc: the field, the
    /// role, the side and the rationale.
    fn engine_doc_rows() -> Vec<(String, String, String, String)> {
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
            if cells.len() < 5 || !cells[1].starts_with('`') || !cells[2].starts_with('`') {
                continue;
            }
            rows.push((
                cells[1].trim_matches('`').to_string(),
                cells[2].trim_matches('`').to_string(),
                cells[3].to_string(),
                cells[4].to_string(),
            ));
        }
        rows
    }

    #[test]
    fn the_engine_mapping_in_the_doc_is_the_one_in_the_code() {
        let rows = engine_doc_rows();
        assert_eq!(
            rows.len(),
            ENGINE_ROLES.len(),
            "the doc lists {} fields and the engine has {}",
            rows.len(),
            ENGINE_ROLES.len()
        );
        for (field, role, side, _) in ENGINE_ROLES {
            let (doc_role, doc_side_name, why) = rows
                .iter()
                .find(|(doc_field, _, _, _)| doc_field == field)
                .map(|(_, role, side, why)| (role, side, why))
                .unwrap_or_else(|| panic!("{field} is missing from the doc"));
            assert_eq!(
                doc_role,
                role.name(),
                "{field} maps to {doc_role} in the doc and {} in the code",
                role.name()
            );
            assert_eq!(
                doc_side_name,
                role.side_name(side),
                "{field} takes {} in the doc and {} in the code",
                doc_side_name,
                role.side_name(side)
            );
            assert!(!why.is_empty(), "{field}'s row states no rationale");
        }
        for (field, _, _, _) in &rows {
            assert!(
                ENGINE_ROLES.iter().any(|(known, _, _, _)| known == field),
                "{field} is in the doc and not in the code"
            );
        }
    }

    #[test]
    fn the_markdown_alerts_use_the_five_alert_roles() {
        let role = |field: &str| {
            ENGINE_ROLES
                .iter()
                .find(|(name, _, _, _)| *name == field)
                .map(|(_, role, _, _)| *role)
                .unwrap_or_else(|| panic!("{field} is not in the mapping"))
        };
        assert_eq!(role("alert_note"), DocRole::AlertNote);
        assert_eq!(role("alert_tip"), DocRole::AlertTip);
        assert_eq!(role("alert_important"), DocRole::AlertImportant);
        assert_eq!(role("alert_warning"), DocRole::AlertWarning);
        assert_eq!(role("alert_caution"), DocRole::AlertCaution);
        // Five facts, five roles: no two share one, and none is the link.
        let alerts = [
            role("alert_note"),
            role("alert_tip"),
            role("alert_important"),
            role("alert_warning"),
            role("alert_caution"),
        ];
        for (index, one) in alerts.iter().enumerate() {
            for other in &alerts[index + 1..] {
                assert_ne!(one, other, "two alerts share a role");
            }
            assert_ne!(*one, role("link_text"), "an alert shares the link's role");
        }
    }
}
