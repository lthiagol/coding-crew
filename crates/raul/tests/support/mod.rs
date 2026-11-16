//! Fixtures shared by the data unit tests and the process render tests.

use std::fs;
use std::path::PathBuf;

/// Malformed task and item files, in the order they are created. The order
/// differs from their name order, which is what the ordering test needs.
pub const CREATION_ORDER: [&str; 4] = [
    "TSK70-broken.json",
    "ITM90-broken.json",
    "TSK02-broken.json",
    "ITM05-broken.json",
];

/// The same files in name order: what a load must report.
pub const NAME_ORDER: [&str; 4] = [
    "ITM05-broken.json",
    "ITM90-broken.json",
    "TSK02-broken.json",
    "TSK70-broken.json",
];

/// Two tasks carrying `TSK01` and two items carrying `ITM01`, each pair
/// named alpha and beta; the titles follow the names.
pub const DUPLICATE_ID_FILES: [(&str, &str); 4] = [
    (
        "TSK01-alpha.json",
        r#"{"schema_version":1,"id":"TSK01","title":"Alpha task","slug":"alpha-task","created":"2026-11-02T09:00Z","updated":"2026-11-02T09:00Z","status":"approved","status_since":"2026-11-02T09:00Z"}"#,
    ),
    (
        "TSK01-beta.json",
        r#"{"schema_version":1,"id":"TSK01","title":"Beta task","slug":"beta-task","created":"2026-11-02T09:00Z","updated":"2026-11-02T09:00Z","status":"approved","status_since":"2026-11-02T09:00Z"}"#,
    ),
    (
        "ITM01-alpha.json",
        r#"{"schema_version":1,"id":"ITM01","title":"Alpha item","slug":"alpha-item","kind":"idea","status":"open"}"#,
    ),
    (
        "ITM01-beta.json",
        r#"{"schema_version":1,"id":"ITM01","title":"Beta item","slug":"beta-item","kind":"idea","status":"open"}"#,
    ),
];

/// A work folder holding the malformed files, created in `CREATION_ORDER`.
pub fn malformed_work(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("raul-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a work folder");
    for name in CREATION_ORDER {
        fs::write(dir.join(name), "{ not json").expect("writing a malformed file");
    }
    dir
}
