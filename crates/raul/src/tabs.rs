//! The five tabs of the window.

/// A tab of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    NeedsYou,
    Path,
    Backlog,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Overview,
        Tab::NeedsYou,
        Tab::Path,
        Tab::Backlog,
        Tab::Settings,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::NeedsYou => "Needs you",
            Tab::Path => "Path",
            Tab::Backlog => "Backlog",
            Tab::Settings => "Settings",
        }
    }

    pub fn index(&self) -> usize {
        Tab::ALL
            .iter()
            .position(|tab| tab == self)
            .unwrap_or_default()
    }

    pub fn from_index(index: usize) -> Tab {
        Tab::ALL[index % Tab::ALL.len()]
    }

    pub fn next(&self) -> Tab {
        Tab::from_index(self.index() + 1)
    }

    pub fn previous(&self) -> Tab {
        Tab::from_index(self.index() + Tab::ALL.len() - 1)
    }

    /// The name of the tab on the command line.
    pub fn name(&self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::NeedsYou => "needs-you",
            Tab::Path => "path",
            Tab::Backlog => "backlog",
            Tab::Settings => "settings",
        }
    }

    /// The tab of a command-line name.
    pub fn from_name(name: &str) -> Option<Tab> {
        Tab::ALL.into_iter().find(|tab| tab.name() == name)
    }

    /// Whether enter opens a row on this tab.
    ///
    /// The Settings tab is a list too, but it reads: enter opens nothing.
    pub fn opens(&self) -> bool {
        match self {
            Tab::Overview | Tab::Settings => false,
            Tab::NeedsYou | Tab::Path | Tab::Backlog => true,
        }
    }

    /// Whether the tab is a list of rows with a cursor.
    pub fn lists(&self) -> bool {
        !matches!(self, Tab::Overview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABELS: [&str; 5] = ["Overview", "Needs you", "Path", "Backlog", "Settings"];
    const NAMES: [&str; 5] = ["overview", "needs-you", "path", "backlog", "settings"];

    #[test]
    fn the_window_has_five_tabs_in_order() {
        assert_eq!(Tab::ALL.len(), 5);
        assert_eq!(Tab::ALL.map(|tab| tab.label()), LABELS);
        assert_eq!(Tab::ALL.map(|tab| tab.name()), NAMES);
        for (index, tab) in Tab::ALL.iter().enumerate() {
            assert_eq!(tab.index(), index);
            assert_eq!(Tab::from_index(index), *tab);
        }
        assert!(Tab::from_name("autopilot").is_none(), "a sixth tab remains");
    }

    #[test]
    fn only_the_tabs_with_tasks_open_a_row() {
        // The Overview is a summary and the Settings tab reads: enter opens
        // a record on the three tabs whose rows are tasks, and nowhere else.
        assert_eq!(
            Tab::ALL.map(|tab| tab.opens()),
            [false, true, true, true, false]
        );
        assert_eq!(
            Tab::ALL.map(|tab| tab.lists()),
            [false, true, true, true, true]
        );
    }

    /// The window sketch's tab bar, with the border and padding removed.
    fn sketch_bar(doc: &str) -> String {
        let sketch = doc.split("```").nth(1).expect("the window sketch");
        let line = sketch
            .lines()
            .find(|line| line.contains("Overview"))
            .expect("the sketch's tab bar");
        line.trim()
            .trim_start_matches('│')
            .trim_end_matches('│')
            .trim()
            .to_string()
    }

    /// The labels in the screens file's tab table, in order.
    fn table_labels(doc: &str) -> Vec<String> {
        doc.lines()
            .skip_while(|line| !line.starts_with("| tab |"))
            .skip(2) // the header row and the separator
            .take_while(|line| line.starts_with("| "))
            .map(|line| {
                line.split('|')
                    .nth(1)
                    .expect("a first cell")
                    .trim()
                    .to_string()
            })
            .collect()
    }

    /// The targets of the markdown links in a document.
    fn link_targets(doc: &str) -> Vec<String> {
        let mut targets = Vec::new();
        let mut rest = doc;
        while let Some(start) = rest.find("](") {
            rest = &rest[start + 2..];
            let Some(end) = rest.find(')') else {
                break;
            };
            let target = rest[..end].split_whitespace().next().unwrap_or("");
            targets.push(target.to_string());
            rest = &rest[end..];
        }
        targets
    }

    /// Whether the doc links the automation at hugo's design record.
    fn points_at_hugo(doc: &str) -> bool {
        link_targets(doc)
            .iter()
            .any(|target| target == "../hugo/design-record.md")
    }

    #[test]
    fn the_screens_doc_sketch_and_tab_table_agree_with_the_code() {
        let doc = include_str!("../../../docs/tui/screens.md");
        let labels: Vec<&str> = Tab::ALL.iter().map(|tab| tab.label()).collect();

        // The sketch draws the same tabs in the same order, three spaces
        // apart, as the bar does.
        assert_eq!(
            sketch_bar(doc),
            labels.join("   "),
            "the window sketch's tab bar disagrees with the code"
        );

        // The tab table has one row per tab, in the same order.
        assert_eq!(
            table_labels(doc),
            labels,
            "the tab table disagrees with the code"
        );

        // The tab bar names the keys the code answers: 1 to the tab count.
        let tab_bar_row = doc
            .lines()
            .find(|line| line.starts_with("| tab bar |"))
            .expect("the tab bar row");
        assert!(
            tab_bar_row.contains(&format!("`1`–`{}`", Tab::ALL.len())),
            "the tab bar row does not name keys 1-{}: {tab_bar_row}",
            Tab::ALL.len()
        );

        // The render flag knows the same names as the enum.
        let names: Vec<String> = Tab::ALL
            .iter()
            .map(|tab| format!("`{}`", tab.name()))
            .collect();
        let doc_flat = doc.split_whitespace().collect::<Vec<&str>>().join(" ");
        assert!(
            doc_flat.contains(&names.join(", ")),
            "the --tab names disagree with the code: {}",
            names.join(", ")
        );

        // The automation's questions are hugo's, not a raul tab's.
        assert!(
            !doc.to_lowercase().contains("autopilot"),
            "the screens file still describes the dropped tab"
        );
        assert!(
            points_at_hugo(doc),
            "the screens file does not point the automation at hugo"
        );
    }

    #[test]
    fn the_hugo_pointer_is_matched_by_its_target_not_its_text() {
        let doc = include_str!("../../../docs/tui/screens.md");
        assert!(points_at_hugo(doc), "the real doc links hugo");
        let dead = doc.replace("../hugo/design-record.md", "../hugo/missing.md");
        assert!(
            !points_at_hugo(&dead),
            "a dead target with the same link text passes the hugo pointer"
        );
    }
}
