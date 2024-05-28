use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::utils::normalize_path;

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

#[derive(Error, Debug)]
pub enum CliError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{path}: {err}")]
    WithPath { err: Box<CliError>, path: PathBuf },
    #[error("{cause}.\n{err}")]
    WithCause { err: Box<CliError>, cause: String },
}

impl CliError {
    pub fn with_path(self, path: &Path) -> Self {
        Self::WithPath {
            err: Box::new(self),
            path: normalize_path(path),
        }
    }
    pub fn with_cause(self, cause: &str) -> Self {
        Self::WithCause {
            err: Box::new(self),
            cause: cause.to_string(),
        }
    }
}
