//! The order each tab keeps its rows in.
//!
//! The Settings tab's sort group and the tab's own ordering read the same
//! value: a tab that is sorted another way says so.

use std::cmp::Ordering;

use crate::path::Branch;
use crate::settings::GroupKind;
use crate::tabs::Tab;

/// The order a tab puts its rows in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// The Overview is a summary, not a list: it has no order.
    None,
    /// The longest wait first, then the id.
    LongestWait,
    /// The claimable tasks in dependency order, then the waiting branches.
    Dependency,
    /// The id, ascending, inside the tab's groups: ideas first.
    Id,
    /// The tab's groups, in the order the spec names them.
    Groups,
}

impl Sort {
    /// The words the Settings tab uses: what comes first, and what the
    /// order is.
    pub fn words(self) -> String {
        match self {
            Sort::None => "no sort: a summary".to_string(),
            Sort::LongestWait => "longest wait first".to_string(),
            Sort::Dependency => "claimable, then branches".to_string(),
            Sort::Id => "ideas first, ids ascending".to_string(),
            Sort::Groups => GroupKind::ALL
                .iter()
                .map(|group| group.label())
                .collect::<Vec<&str>>()
                .join(", "),
        }
    }

    /// The comparator of the longest-wait order: the longest wait first,
    /// and the id when two waits are equal.
    ///
    /// The order is asserted, not assumed: a release build that is handed
    /// another `Sort` stops here instead of sorting another way.
    pub fn compare_waits(self, one: (i64, &str), other: (i64, &str)) -> Ordering {
        assert_eq!(
            self,
            Sort::LongestWait,
            "compare_waits is not the longest-wait order"
        );
        other.0.cmp(&one.0).then_with(|| one.1.cmp(other.1))
    }

    /// The comparator of the id order.
    ///
    /// The order is asserted, not assumed: a release build that is handed
    /// another `Sort` stops here instead of sorting another way.
    pub fn compare_ids(self, one: &str, other: &str) -> Ordering {
        assert_eq!(self, Sort::Id, "compare_ids is not the id order");
        one.cmp(other)
    }

    /// The branches of a tree tab, in the order the tab draws them.
    pub fn branches(self) -> &'static [Branch] {
        match self {
            Sort::Dependency => &Branch::ALL,
            _ => &[],
        }
    }
}

impl Tab {
    /// The order this tab keeps its rows in.
    pub fn sort(self) -> Sort {
        match self {
            Tab::Overview => Sort::None,
            Tab::NeedsYou => Sort::LongestWait,
            Tab::Path => Sort::Dependency,
            Tab::Backlog => Sort::Id,
            Tab::Settings => Sort::Groups,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_tab_names_the_order_it_keeps() {
        assert_eq!(Tab::Overview.sort(), Sort::None);
        assert_eq!(Tab::NeedsYou.sort(), Sort::LongestWait);
        assert_eq!(Tab::Path.sort(), Sort::Dependency);
        assert_eq!(Tab::Backlog.sort(), Sort::Id);
        assert_eq!(Tab::Settings.sort(), Sort::Groups);
    }

    #[test]
    fn every_tab_says_something_different() {
        // A tab that is sorted another way says so: no two tabs share the
        // words of their order.
        let words: Vec<String> = Tab::ALL.iter().map(|tab| tab.sort().words()).collect();
        let unique: HashSet<&String> = words.iter().collect();
        assert_eq!(unique.len(), words.len(), "{words:?}");
        for word in &words {
            assert!(!word.is_empty(), "{words:?}");
            assert!(
                word.chars().count() <= 30,
                "{word:?} is too long for 40 columns"
            );
        }
    }

    #[test]
    fn the_overview_says_it_has_no_sort() {
        let words = Tab::Overview.sort().words();
        assert!(words.contains("no sort"), "{words}");
        assert!(words.contains("summary"), "{words}");
        assert!(Tab::Overview.sort().branches().is_empty());
    }

    #[test]
    fn the_settings_order_names_its_groups_in_order() {
        let words = Tab::Settings.sort().words();
        let expected = GroupKind::ALL
            .iter()
            .map(|group| group.label())
            .collect::<Vec<&str>>()
            .join(", ");
        assert_eq!(words, expected);
    }

    #[test]
    fn the_path_order_is_the_branch_order_the_tree_draws() {
        assert_eq!(Tab::Path.sort().branches(), &Branch::ALL);
        // The other sorts have no branches; a tab that gains one says so.
        for tab in [Tab::Overview, Tab::NeedsYou, Tab::Backlog, Tab::Settings] {
            assert!(tab.sort().branches().is_empty(), "{tab:?}");
        }
    }

    #[test]
    fn the_comparators_sort_the_way_their_words_say() {
        // The longest wait first, then the id.
        let sort = Tab::NeedsYou.sort();
        let mut rows = [("TSK02", 10i64), ("TSK01", 10), ("TSK03", 20)];
        rows.sort_by(|one, other| sort.compare_waits((one.1, one.0), (other.1, other.0)));
        assert_eq!(rows, [("TSK03", 20), ("TSK01", 10), ("TSK02", 10)]);
        // The id, ascending.
        let sort = Tab::Backlog.sort();
        let mut ids = ["ITM02", "ITM01", "TSK27"];
        ids.sort_by(|one, other| sort.compare_ids(one, other));
        assert_eq!(ids, ["ITM01", "ITM02", "TSK27"]);
    }

    #[test]
    fn the_id_row_says_what_comes_first_and_what_the_order_is() {
        // The Backlog's groups decide which rows come first; the id decides
        // the order inside a group. The row says both.
        let words = Tab::Backlog.sort().words();
        assert!(words.contains("ideas first"), "{words}");
        assert!(words.contains("ids ascending"), "{words}");
    }

    #[test]
    #[should_panic(expected = "compare_ids is not the id order")]
    fn the_id_comparator_refuses_another_sort() {
        // The order is a real assertion, not a debug one: a release build
        // stops here instead of mis-sorting in silence.
        Tab::Settings.sort().compare_ids("ITM01", "ITM02");
    }

    #[test]
    #[should_panic(expected = "compare_waits is not the longest-wait order")]
    fn the_wait_comparator_refuses_another_sort() {
        Tab::Backlog
            .sort()
            .compare_waits((1, "TSK01"), (2, "TSK02"));
    }
}
