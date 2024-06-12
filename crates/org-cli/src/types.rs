use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug)]
pub enum InpType<'a> {
    File(&'a Path),
    Dir(&'a Path),
}

#[derive(Debug)]
pub enum OutType<'a> {
    File(&'a Path),
    Dir(&'a Path),
}

/// Adding extra information to errors.
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Error")]
    Logic,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{path}: {err}")]
    WithPath { err: Box<CliError>, path: PathBuf },
    #[error("{err}\n{cause}")]
    WithCause { err: Box<CliError>, cause: String },
}

impl CliError {
    pub fn new() -> Self {
        Self::Logic
    }
    pub fn with_path(self, path: &Path) -> Self {
        Self::WithPath {
            err: Box::new(self),
            path: path.to_owned(),
        }
    }
    pub fn with_cause(self, cause: &str) -> Self {
        Self::WithCause {
            err: Box::new(self),
            cause: cause.to_string(),
        }
    }
}
