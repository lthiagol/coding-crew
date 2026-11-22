//! The chained panic hook still reports the panic it runs before.
//!
//! The hook is process-global, so this lives in its own test binary: no other
//! test shares it, and the terminal stays untouched.

use std::sync::Mutex;

use coding_crew_core::screen::chain_hook;

/// The order the hooks ran in, so the test can pin `leave` before the report.
static ORDER: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn leave() {
    ORDER.lock().unwrap().push("leave");
}

#[test]
fn a_panic_is_still_reported_by_the_hook_it_chains_to() {
    // The hook leaves the terminal, then hands the panic to the hook that
    // was set before it: the panic is reported, not swallowed.
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_: &std::panic::PanicHookInfo<'_>| {
        ORDER.lock().unwrap().push("reported");
    }));
    chain_hook(leave);
    let _ = std::panic::catch_unwind(|| panic!("a render-path panic"));
    std::panic::set_hook(before);
    assert_eq!(
        *ORDER.lock().unwrap(),
        ["leave", "reported"],
        "the hook must leave the terminal, then report the panic"
    );
}
