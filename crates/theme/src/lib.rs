//! The palette raul and hugo paint with.
//!
//! A slot names a role; a preset decides what the role looks like. The
//! domain knows nothing about this: `crates/core` never names a colour, and
//! a phase names a slot, never a colour.

pub mod chrome;
pub mod icons;
pub mod picker;
pub mod text;

mod theme;

pub use icons::{Icon, IconSet};
pub use theme::*;
