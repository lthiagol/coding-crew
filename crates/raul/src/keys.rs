//! The keys raul answers, in one table.
//!
//! The window's input handling reads this table, and the gate checks it: a
//! key is answered only by a row that names it, and every row does its act.
//! The footer and the help name the keys in their own words.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tabs::Tab;

/// Where a binding applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The window: the tab bar, the help and quitting.
    Window,
    /// The rows of a tab that lists them.
    Rows,
    /// The Tasks tab's rows: the order keys and the filter's opening key.
    Tasks,
    /// The detail pane, while it covers the window.
    Detail,
    /// The filter line, while it is typed into: its keys type, and no other
    /// key of the window answers.
    Filter,
    /// The Settings tab: its keys change the file's values.
    Settings,
    /// A pick open on the Settings tab: its keys move over the choices and keep
    /// or drop the pick, and no other key answers.
    Picking,
    /// A value of the Settings tab, while it is typed in: its keys type, and no
    /// other key answers.
    Value,
    /// The question quit asks with unsaved changes: save, quit or stay.
    Asking,
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
    /// Open the filter: the footer takes the keys as its text.
    Filter,
    /// Keep the filter text and return to the rows.
    KeepFilter,
    /// Clear the filter text.
    ClearFilter,
    /// Delete the last letter of the filter text.
    EraseFilter,
    /// Type the key into the filter text.
    TypeFilter,
    /// Quit at once, without asking about unsaved changes.
    Exit,
    /// Move the pick's cursor by the step, and preview the choice under it.
    StepPick(isize),
    /// Keep the pick's choice.
    KeepPick,
    /// Drop the pick: the value is what it was before the pick opened.
    DropPick,
    /// Start typing the value under the cursor.
    Begin,
    /// Write the changes to the configuration file.
    Save,
    /// Drop the unsaved changes.
    Undo,
    /// Read the configuration file again.
    Reload,
    /// Keep the value being typed in.
    Keep,
    /// Drop the value being typed in.
    Drop,
    /// Delete the last letter of the value being typed in.
    EraseValue,
    /// Type the key into the value being typed in.
    TypeValue,
    /// Do not quit: the question about unsaved changes is answered with stay.
    Stay,
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
    /// A printable key: a letter, a digit or a sign, with no ctrl or alt.
    Typed,
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
            Chord::Typed => {
                matches!(key.code, KeyCode::Char(_))
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            }
        }
    }
}

/// One row of the table: the keys that do an act, and where they do it. The
/// footer and the help name the keys in their own words.
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    /// The scopes the binding works in.
    pub scopes: &'static [Scope],
    /// The keys that do this.
    pub chords: &'static [Chord],
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
}

/// The keys of the window, in the order the groups read them: the window
/// first, then the rows, then the filter line, then the detail pane.
pub const BINDINGS: [Binding; 50] = [
    Binding {
        scopes: &[Scope::Window, Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('q'), false)],
        act: Act::Quit,
    },
    Binding {
        scopes: &[Scope::Window, Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('c'), true)],
        act: Act::Exit,
    },
    Binding {
        // The window's quit key, as the screens doc names it: in the detail
        // pane the same key returns to the tab instead.
        scopes: &[Scope::Window],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        act: Act::Quit,
    },
    Binding {
        // In the detail pane `?` lists the pane's keys, as it lists the window's.
        scopes: &[Scope::Window, Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('?'), false)],
        act: Act::Help,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[Chord::Digits],
        act: Act::JumpTab,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[
            Chord::Key(KeyCode::Right, false),
            Chord::Key(KeyCode::Char('l'), false),
        ],
        act: Act::NextTab,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[
            Chord::Key(KeyCode::Left, false),
            Chord::Key(KeyCode::Char('h'), false),
        ],
        act: Act::PreviousTab,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[Chord::Key(KeyCode::Tab, false)],
        act: Act::NextTab,
    },
    Binding {
        scopes: &[Scope::Window],
        chords: &[Chord::Key(KeyCode::BackTab, false)],
        act: Act::PreviousTab,
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[
            Chord::Key(KeyCode::Up, false),
            Chord::Key(KeyCode::Char('k'), false),
        ],
        act: Act::Move(-1),
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[
            Chord::Key(KeyCode::Down, false),
            Chord::Key(KeyCode::Char('j'), false),
        ],
        act: Act::Move(1),
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::PageUp, false)],
        act: Act::Page(-1),
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::PageDown, false)],
        act: Act::Page(1),
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::Char('g'), false)],
        act: Act::Top,
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::Char('G'), false)],
        act: Act::Bottom,
    },
    Binding {
        scopes: &[Scope::Rows],
        chords: &[Chord::Key(KeyCode::Enter, false)],
        act: Act::Open,
    },
    Binding {
        scopes: &[Scope::Tasks],
        chords: &[Chord::Key(KeyCode::Char('o'), false)],
        act: Act::NextSort,
    },
    Binding {
        scopes: &[Scope::Tasks],
        chords: &[Chord::Key(KeyCode::Char('O'), false)],
        act: Act::ReverseSort,
    },
    Binding {
        scopes: &[Scope::Tasks],
        chords: &[Chord::Key(KeyCode::Char('/'), false)],
        act: Act::Filter,
    },
    Binding {
        // With a filter in force, esc clears it before it quits the window.
        scopes: &[Scope::Tasks],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        act: Act::ClearFilter,
    },
    Binding {
        scopes: &[Scope::Filter],
        chords: &[Chord::Key(KeyCode::Enter, false)],
        act: Act::KeepFilter,
    },
    Binding {
        scopes: &[Scope::Filter],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        act: Act::ClearFilter,
    },
    Binding {
        scopes: &[Scope::Filter],
        chords: &[Chord::Key(KeyCode::Backspace, false)],
        act: Act::EraseFilter,
    },
    Binding {
        scopes: &[Scope::Filter],
        chords: &[Chord::Typed],
        act: Act::TypeFilter,
    },
    Binding {
        scopes: &[Scope::Filter],
        chords: &[Chord::Key(KeyCode::Char('c'), true)],
        act: Act::Exit,
    },
    Binding {
        scopes: &[Scope::Settings],
        chords: &[Chord::Key(KeyCode::Enter, false)],
        act: Act::Begin,
    },
    Binding {
        scopes: &[Scope::Settings],
        chords: &[Chord::Key(KeyCode::Char('w'), false)],
        act: Act::Save,
    },
    Binding {
        scopes: &[Scope::Settings],
        chords: &[Chord::Key(KeyCode::Char('u'), false)],
        act: Act::Undo,
    },
    Binding {
        scopes: &[Scope::Settings],
        chords: &[Chord::Key(KeyCode::Char('r'), false)],
        act: Act::Reload,
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        act: Act::Back,
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[
            Chord::Key(KeyCode::Up, false),
            Chord::Key(KeyCode::Char('k'), false),
        ],
        act: Act::Move(-1),
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[
            Chord::Key(KeyCode::Down, false),
            Chord::Key(KeyCode::Char('j'), false),
        ],
        act: Act::Move(1),
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::PageUp, false)],
        act: Act::Page(-1),
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::PageDown, false)],
        act: Act::Page(1),
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('g'), false)],
        act: Act::Top,
    },
    Binding {
        scopes: &[Scope::Detail],
        chords: &[Chord::Key(KeyCode::Char('G'), false)],
        act: Act::Bottom,
    },
    Binding {
        scopes: &[Scope::Picking],
        chords: &[
            Chord::Key(KeyCode::Up, false),
            Chord::Key(KeyCode::Char('k'), false),
        ],
        act: Act::StepPick(-1),
    },
    Binding {
        scopes: &[Scope::Picking],
        chords: &[
            Chord::Key(KeyCode::Down, false),
            Chord::Key(KeyCode::Char('j'), false),
        ],
        act: Act::StepPick(1),
    },
    Binding {
        scopes: &[Scope::Picking],
        chords: &[Chord::Key(KeyCode::Enter, false)],
        act: Act::KeepPick,
    },
    Binding {
        scopes: &[Scope::Picking],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        act: Act::DropPick,
    },
    Binding {
        scopes: &[Scope::Picking],
        chords: &[Chord::Key(KeyCode::Char('c'), true)],
        act: Act::Exit,
    },
    Binding {
        scopes: &[Scope::Value],
        chords: &[Chord::Key(KeyCode::Enter, false)],
        act: Act::Keep,
    },
    Binding {
        scopes: &[Scope::Value],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        act: Act::Drop,
    },
    Binding {
        scopes: &[Scope::Value],
        chords: &[Chord::Key(KeyCode::Backspace, false)],
        act: Act::EraseValue,
    },
    Binding {
        scopes: &[Scope::Value],
        chords: &[Chord::Typed],
        act: Act::TypeValue,
    },
    Binding {
        scopes: &[Scope::Value],
        chords: &[Chord::Key(KeyCode::Char('c'), true)],
        act: Act::Exit,
    },
    Binding {
        scopes: &[Scope::Asking],
        chords: &[Chord::Key(KeyCode::Char('w'), false)],
        act: Act::Save,
    },
    Binding {
        scopes: &[Scope::Asking],
        chords: &[Chord::Key(KeyCode::Char('q'), false)],
        act: Act::Quit,
    },
    Binding {
        scopes: &[Scope::Asking],
        chords: &[Chord::Key(KeyCode::Esc, false)],
        act: Act::Stay,
    },
    Binding {
        scopes: &[Scope::Asking],
        chords: &[Chord::Key(KeyCode::Char('c'), true)],
        act: Act::Exit,
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
/// The help overlay closes on any key. `esc`, the key that returns from the
/// detail pane, closes the help alone, so `esc` in the help does not leave
/// raul or the pane.
pub fn leaves_overlay(key: &KeyEvent) -> bool {
    binding(Scope::Detail, key).is_some_and(|binding| binding.act == Act::Back)
}

/// The letter, digit or sign a key press types, when it types one.
pub fn typed(key: &KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(letter) if Chord::Typed.matches(key) => Some(letter),
        _ => None,
    }
}

/// The key a second click on the cursor's row answers as: `enter`, which opens
/// the row or edits the value.
pub fn enter() -> KeyEvent {
    KeyEvent::from(KeyCode::Enter)
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
    fn every_binding_has_a_key_a_scope_and_an_act() {
        // A row with no chord matches nothing, and one with no scope works
        // nowhere: either would list a key that does nothing.
        let mut seen = HashSet::new();
        for binding in BINDINGS {
            assert!(!binding.scopes.is_empty(), "{:?} has no scope", binding.act);
            assert!(
                !binding.chords.is_empty(),
                "{:?} matches no key",
                binding.act
            );
            // One answer per key in a scope: a second row for the same key
            // there would never be reached.
            for scope in binding.scopes {
                for chord in binding.chords {
                    assert!(
                        seen.insert((format!("{chord:?}"), format!("{scope:?}"))),
                        "{chord:?} is bound twice in {scope:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_table_groups_the_scopes_in_the_order_of_the_groups() {
        // The rows run window, then rows, then detail, then the value and the
        // question: a binding that works in two scopes sits with the first of
        // them.
        let rank = |scope: Scope| match scope {
            Scope::Window => 0,
            Scope::Rows | Scope::Tasks | Scope::Filter | Scope::Settings => 1,
            Scope::Detail => 2,
            Scope::Picking | Scope::Value | Scope::Asking => 3,
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
                "{:?} in {:?} is out of the scope order",
                binding.act,
                binding.scopes
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
        for scope in [
            Scope::Window,
            Scope::Rows,
            Scope::Tasks,
            Scope::Detail,
            Scope::Filter,
            Scope::Settings,
            Scope::Picking,
            Scope::Value,
            Scope::Asking,
        ] {
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
            Some(Act::Exit)
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
