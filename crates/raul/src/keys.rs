//! The keys raul answers, in one table.
//!
//! The window's input handling and the Settings tab's keys group both read
//! this table: a key that is handled but not listed, or listed but not
//! handled, is one the gate can see.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tabs::Tab;

/// Where a binding applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The window: the tab bar, the help and quitting.
    Window,
    /// The rows of a tab that lists them.
    Rows,
    /// The Tasks tab's rows: the order keys.
    Tasks,
    /// The detail pane, while it covers the window.
    Detail,
}

impl Scope {
    /// The name of the scope, as the keys group writes it.
    pub fn label(&self) -> &'static str {
        match self {
            Scope::Window => "window",
            Scope::Rows => "rows",
            Scope::Tasks => "tasks",
            Scope::Detail => "detail",
        }
    }
}

/// What a key asks the window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Leave raul.
    Quit,
    /// Open the help overlay.
    Help,
    /// The next tab, in the order of the bar.
    NextTab,
    /// The previous tab, in the order of the bar.
    PreviousTab,
    /// The tab of the digit that was pressed.
    JumpTab,
    /// Move the cursor by rows, down on a positive count.
    Move(isize),
    /// Move by pages, down on a positive count.
    Page(isize),
    /// The first row, or the top of the document.
    Top,
    /// The last row, or the end of the document.
    Bottom,
    /// Open the cursor's row as the detail pane.
    Open,
    /// The next key the Tasks tab sorts by.
    NextSort,
    /// The Tasks tab's order, the other way.
    ReverseSort,
    /// Return from the detail pane to the tab it came from.
    Back,
}

/// A key, or the keys that share one binding: `Control` says the key is
/// pressed with ctrl, and the other bindings take the key as the terminal
/// sends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chord {
    /// A key code, with ctrl when the flag says so.
    Key(KeyCode, bool),
    /// The digits that name a tab.
    Digits,
}

impl Chord {
    /// Whether the key press is this chord.
    fn matches(&self, key: &KeyEvent) -> bool {
        match self {
            Chord::Key(code, control) => {
                key.code == *code && (!*control || key.modifiers.contains(KeyModifiers::CONTROL))
            }
            Chord::Digits => {
                let last = char::from(b'0' + Tab::ALL.len() as u8);
                matches!(key.code, KeyCode::Char(digit) if ('1'..=last).contains(&digit))
            }
        }
    }
}

/// One row of the table, and one row of the Settings tab.
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    /// The scopes the binding works in.
    pub scopes: &'static [Scope],
    /// The keys that do this.
    pub chords: &'static [Chord],
    /// The keys, as a screen writes them.
    pub key: &'static str,
    /// What the key does.
    pub says: &'static str,
    /// What the key asks for.
    pub act: Act,
}

impl Binding {
    /// Whether the binding works in a scope.
    pub fn applies_to(&self, scope: Scope) -> bool {
        self.scopes.contains(&scope)
    }

    /// Whether the key press is one of the binding's keys.
    pub fn matches(&self, key: &KeyEvent) -> bool {
        self.chords.iter().any(|chord| chord.matches(key))
    }

    /// The scopes, as a screen writes them.
    pub fn scope_names(&self) -> String {
        self.scopes
            .iter()
            .map(Scope::label)
            .collect::<Vec<&str>>()
            .join(", ")
    }
}

/// The keys of the window, in the order the groups read them: the window
/// first, then the rows, then the detail pane.
pub const BINDINGS: [Binding; 25] = [
    Binding {
        scopes: &[Scope::Window, Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('q'), false)],
        key: "q",
        says: "quit",
        act: Act::Quit,
    },
    Binding {
        scopes: &[Scope::Window, Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('c'), true)],
        key: "ctrl-c",
        says: "quit",
        act: Act::Quit,
    },
    Binding {
        // The window's quit key, as the screens doc names it: in the detail
        // pane the same key returns to the tab instead.
        scopes: &[Scope::Window],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        key: "esc",
        says: "quit",
        act: Act::Quit,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[Chord::Key(KeyCode::Char('?'), false)],
        key: "?",
        says: "help",
        act: Act::Help,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[Chord::Digits],
        key: "1-5",
        says: "tab",
        act: Act::JumpTab,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[
            Chord::Key(KeyCode::Right, false),
            Chord::Key(KeyCode::Char('l'), false),
        ],
        key: "→ l",
        says: "next tab",
        act: Act::NextTab,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[
            Chord::Key(KeyCode::Left, false),
            Chord::Key(KeyCode::Char('h'), false),
        ],
        key: "← h",
        says: "previous tab",
        act: Act::PreviousTab,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[Chord::Key(KeyCode::Tab, false)],
        key: "tab",
        says: "next tab",
        act: Act::NextTab,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[Chord::Key(KeyCode::BackTab, false)],
        key: "shift-tab",
        says: "previous tab",
        act: Act::PreviousTab,
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[
            Chord::Key(KeyCode::Up, false),
            Chord::Key(KeyCode::Char('k'), false),
        ],
        key: "↑ k",
        says: "move up",
        act: Act::Move(-1),
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[
            Chord::Key(KeyCode::Down, false),
            Chord::Key(KeyCode::Char('j'), false),
        ],
        key: "↓ j",
        says: "move down",
        act: Act::Move(1),
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::PageUp, false)],
        key: "pgup",
        says: "move a page up",
        act: Act::Page(-1),
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::PageDown, false)],
        key: "pgdn",
        says: "move a page down",
        act: Act::Page(1),
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::Char('g'), false)],
        key: "g",
        says: "first row",
        act: Act::Top,
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::Char('G'), false)],
        key: "G",
        says: "last row",
        act: Act::Bottom,
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::Enter, false)],
        key: "enter",
        says: "open",
        act: Act::Open,
    },
    Binding {
        scopes: &[Scope::Tasks],
        chords: &[Chord::Key(KeyCode::Char('o'), false)],
        key: "o",
        says: "next sort key",
        act: Act::NextSort,
    },
    Binding {
        scopes: &[Scope::Tasks],
        chords: &[Chord::Key(KeyCode::Char('O'), false)],
        key: "O",
        says: "reverse sort",
        act: Act::ReverseSort,
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        key: "esc",
        says: "back",
        act: Act::Back,
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[
            Chord::Key(KeyCode::Up, false),
            Chord::Key(KeyCode::Char('k'), false),
        ],
        key: "↑ k",
        says: "scroll up",
        act: Act::Move(-1),
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[
            Chord::Key(KeyCode::Down, false),
            Chord::Key(KeyCode::Char('j'), false),
        ],
        key: "↓ j",
        says: "scroll down",
        act: Act::Move(1),
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::PageUp, false)],
        key: "pgup",
        says: "scroll a page up",
        act: Act::Page(-1),
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::PageDown, false)],
        key: "pgdn",
        says: "scroll a page down",
        act: Act::Page(1),
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('g'), false)],
        key: "g",
        says: "top of the document",
        act: Act::Top,
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('G'), false)],
        key: "G",
        says: "end of the document",
        act: Act::Bottom,
    },
];

/// The binding of a key press in a scope, if the table has one.
pub fn binding(scope: Scope, key: &KeyEvent) -> Option<&'static Binding> {
    BINDINGS
        .iter()
        .find(|binding| binding.applies_to(scope) && binding.matches(key))
}

/// Whether the key leaves an overlay.
///
/// The help overlay closes on any key; the key that returns from the detail
/// pane closes it too, so `esc` in the help does not leave raul.
pub fn leaves_overlay(key: &KeyEvent) -> bool {
    binding(Scope::Detail, key).is_some_and(|binding| binding.act == Act::Back)
}

/// The digit a key press names, when it names one.
pub fn digit(key: &KeyEvent) -> Option<usize> {
    match key.code {
        KeyCode::Char(digit) => digit.to_digit(10).map(|digit| digit as usize),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn every_binding_has_a_key_a_word_and_a_scope() {
        let mut seen = HashSet::new();
        for binding in BINDINGS {
            assert!(!binding.key.is_empty(), "{binding:?} has no key");
            assert!(!binding.says.is_empty(), "{} says nothing", binding.key);
            assert!(!binding.scopes.is_empty(), "{} has no scope", binding.key);
            assert!(!binding.chords.is_empty(), "{} matches no key", binding.key);
            // One row per key: a label and a place in the table name it.
            assert!(
                seen.insert((binding.key, binding.scope_names())),
                "{} ({}) is listed twice",
                binding.key,
                binding.scope_names()
            );
        }
    }

    #[test]
    fn the_table_groups_the_scopes_in_the_order_of_the_groups() {
        // The rows run window, then rows, then detail: a binding that works
        // in two scopes sits with the first of them.
        let rank = |scope: Scope| match scope {
            Scope::Window => 0,
            Scope::Rows | Scope::Tasks => 1,
            Scope::Detail => 2,
        };
        let mut last = 0;
        for binding in BINDINGS {
            let first = binding
                .scopes
                .iter()
                .map(|scope| rank(*scope))
                .min()
                .expect("a scope");
            assert!(
                first >= last,
                "{} ({}) is out of the scope order",
                binding.key,
                binding.scope_names()
            );
            last = first;
        }
        let mut scopes = Vec::new();
        for binding in BINDINGS {
            for scope in binding.scopes {
                if !scopes.contains(scope) {
                    scopes.push(*scope);
                }
            }
        }
        // Every scope is listed by at least one key.
        for scope in [Scope::Window, Scope::Rows, Scope::Tasks, Scope::Detail] {
            assert!(scopes.contains(&scope), "{scope:?} has no key");
        }
    }

    #[test]
    fn the_digits_name_every_tab_and_no_sixth() {
        // The digits stop at the last tab: the seventh names no tab.
        for index in 0..Tab::ALL.len() {
            let name = char::from(b'1' + index as u8);
            let event = press(KeyCode::Char(name));
            let binding = binding(Scope::Window, &event).expect("a digit binding");
            assert_eq!(binding.act, Act::JumpTab, "{name}");
            assert_eq!(digit(&event), Some(index + 1));
        }
        let seven = press(KeyCode::Char('7'));
        assert!(binding(Scope::Window, &seven).is_none(), "7 does something");
    }

    #[test]
    fn a_control_key_needs_the_control_modifier() {
        let plain = press(KeyCode::Char('c'));
        assert!(binding(Scope::Window, &plain).is_none(), "plain c quits");
        let mut control = press(KeyCode::Char('c'));
        control.modifiers = KeyModifiers::CONTROL;
        assert_eq!(
            binding(Scope::Window, &control).map(|binding| binding.act),
            Some(Act::Quit)
        );
    }

    #[test]
    fn quitting_and_the_detail_keys_work_in_both_places() {
        // q quits from the window and from the detail pane; esc quits in the
        // window and returns from the pane; the pane's own keys are not the
        // window's.
        assert_eq!(
            binding(Scope::Window, &press(KeyCode::Char('q'))).map(|b| b.act),
            Some(Act::Quit)
        );
        assert_eq!(
            binding(Scope::Detail, &press(KeyCode::Char('q'))).map(|b| b.act),
            Some(Act::Quit)
        );
        assert_eq!(
            binding(Scope::Detail, &press(KeyCode::Esc)).map(|b| b.act),
            Some(Act::Back)
        );
        assert_eq!(
            binding(Scope::Window, &press(KeyCode::Esc)).map(|b| b.act),
            Some(Act::Quit),
            "esc no longer quits in the window"
        );
        // The help overlay closes on the key that returns from the pane,
        // and no other key is an overlay key.
        assert!(leaves_overlay(&press(KeyCode::Esc)));
        assert!(!leaves_overlay(&press(KeyCode::Char('q'))));
    }

    #[test]
    fn the_window_keys_stop_at_the_detail_pane() {
        // The pane is modal: the tab keys and the row keys do nothing while
        // it is open.
        for code in [KeyCode::Char('1'), KeyCode::Tab, KeyCode::Enter] {
            assert!(
                binding(Scope::Detail, &press(code)).is_none(),
                "{code:?} reaches the window through the pane"
            );
        }
    }
}
