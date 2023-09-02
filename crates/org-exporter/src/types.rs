use core::fmt;

use org_parser::{NodeID, Parser};

/// Trait for exporter implementations
///
/// Exporting backends must implement this trait.
pub trait Exporter<'buf> {
    /// Writes the AST generated from the input into a `String`.
    fn export(input: &str) -> core::result::Result<String, fmt::Error>;
    /// Writes the AST generated from the input into a buffer that implements `Write`.
    fn export_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> fmt::Result;
    fn export_tree<T: fmt::Write>(parsed: &Parser, buf: &'buf mut T) -> fmt::Result;
}

/// Private interface for Exporter types.
pub(crate) trait ExporterInner<'buf> {
    /// Entry point of the exporter to handle macros.
    ///
    /// Exporting macros entails creating a new context and parsing objects,
    /// as opposed to elements.
    fn export_macro_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> fmt::Result;
    /// Primary exporting routine.
    ///
    /// This method is called recursively until every `Node` in the tree is exhausted.
    fn export_rec(&mut self, node_id: &NodeID, parser: &Parser) -> fmt::Result;
}
