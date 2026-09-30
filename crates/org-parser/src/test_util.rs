//! Helpers for snapshot tests.
//!
//! [`snap!`] renders a parsed document with [`Parser::render_tree`] and hands it to
//! [`insta`], which compares it with a stored snapshot. Any change in structure fails the test with a diff.
//!
//! Snapshots are stored in `<crate>/snapshots/` refer to the README for more.

/// Snapshots live in one directory at the crate root, not next to each source file.
pub(crate) const SNAPSHOT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/snapshots");

/// Asserts that a parsed document matches its stored [`insta`] snapshot.
///
/// The source text goes in the snapshot's YAML header as `description`, so the body is only
/// the rendered tree. Insta doesn't compare the header, only the body.
macro_rules! snap {
    ($parser:expr) => {{
        let parser = &$parser;
        ::insta::with_settings!({
            snapshot_path => $crate::test_util::SNAPSHOT_DIR,
            // we use INSTA_YAML_BLOCK_STYLE=1 in .cargo/config.toml to ensure
            // output isn't serialized (not use quotes & newlines)
            description => parser.source.to_string(),
            omit_expression => true,
        }, {
            ::insta::assert_snapshot!(parser.render_tree());
        });
    }};
}
pub(crate) use snap;
