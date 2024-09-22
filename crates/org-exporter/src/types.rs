use core::fmt;
use org_parser::{NodeID, Parser};
use std::{ops::Range, path::PathBuf};
use thiserror::Error;

pub(crate) type Result<T> = core::result::Result<T, ExportError>;

use crate::{include::IncludeError, org_macros::MacroError};

#[derive(Debug, Clone, Default)]
pub struct ConfigOptions {
    /// Used for evaluating relative paths in #+include: statements
    file_path: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("{0}-{1}: {source}", span.start, span.end)]
    LogicError {
        span: Range<usize>,
        source: LogicErrorKind,
    },
    #[error("{0}")]
    WriteError(#[from] fmt::Error),
}

#[derive(Debug, Error)]
pub enum LogicErrorKind {
    #[error("{0}")]
    Include(#[from] IncludeError),
    #[error("{0}")]
    Macro(#[from] MacroError),
}

#[derive(Debug, Error)]
#[error("{context} {path}: {source}")]
pub struct FileError {
    pub context: String,
    pub path: PathBuf,
    pub source: std::io::Error,
}

impl ConfigOptions {
    pub fn new(file_path: Option<PathBuf>) -> Self {
        Self { file_path }
    }
    pub fn file_path(&self) -> &Option<PathBuf> {
        &self.file_path
    }
}

/// Trait for exporter implementations
///
/// Exporting backends must implement this trait.
pub trait Exporter<'buf> {
    /// Writes the AST generated from the input into a `String`.
    fn export(input: &str, conf: ConfigOptions) -> Result<String>;
    /// Writes the AST generated from the input into a buffer that implements `Write`.
    fn export_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
        conf: ConfigOptions,
    ) -> Result<()>;
    fn export_tree<T: fmt::Write>(
        parsed: &Parser,
        buf: &'buf mut T,
        conf: ConfigOptions,
    ) -> Result<()>;

    // fn errors(&self) -> Vec<ExportError>;
    // fn warnings(&self);
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
        conf: ConfigOptions,
    ) -> Result<()>;
    /// Primary exporting routine.
    ///
    /// This method is called recursively until every `Node` in the tree is exhausted.
    fn export_rec(&mut self, node_id: &NodeID, parser: &Parser) -> Result<()>;
    /// The canonical name of the exporting backend
    /// REVIEW: make public?
    fn backend_name() -> &'static str;
    fn config_opts(&self) -> &ConfigOptions;
}
