//! Helpers for snapshot tests.
//!
//! [`snap!`] renders a parsed document with [`Parser::render_tree`] and hands it to
//! [`insta`], which compares it with a stored snapshot. Any change in structure fails the test with a diff.
//!
//! Snapshots are stored in `<crate>/snapshots/` refer to the README for more.

use crate::types::Parser;

/// Snapshots live in one directory at the crate root, not next to each source file.
pub(crate) const SNAPSHOT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/snapshots");

/// The source text followed by the rendered tree, so a snapshot shows its own input.
pub(crate) fn snapshot_text(parser: &Parser) -> String {
    format!("{:?}\n{}", parser.source, parser.render_tree())
}

/// Asserts that a parsed document matches its stored [`insta`] snapshot.
///
/// The macro expands in the calling test, so `insta` names the snapshot after that test
/// and names it after that test's module path.
macro_rules! snap {
    ($parser:expr) => {{
        let parser = &$parser;
        ::insta::with_settings!({snapshot_path => $crate::test_util::SNAPSHOT_DIR}, {
            ::insta::assert_snapshot!($crate::test_util::snapshot_text(parser));
        });
    }};
}
pub(crate) use snap;
