use core::fmt;

use org_parser::node_pool::NodeID;
use org_parser::types::Parser;

/// Primary exporter trait.
///
/// Exporting backends should implement this trait.
pub trait Exporter<'buf> {
    /// Writes the AST generated from the input into a `String`.
    fn export(input: &str) -> core::result::Result<String, fmt::Error>;
    /// Writes the AST generated from the input into a buffer that implements `Write`.
    fn export_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error>;
    /// Entry point of the exporter to handle macros.
    ///
    /// Exporting macros entails creating a new context and parsing objects,
    /// as opposed to elements.
    fn export_macro_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error>;
    /// Primary exporting routine.
    ///
    /// This method is called recursively until every `Node` in the tree is exhausted.
    fn export_rec(&mut self, node_id: &NodeID, parser: &Parser) -> fmt::Result;
}
