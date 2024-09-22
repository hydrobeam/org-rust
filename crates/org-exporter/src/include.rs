//! Handle #+include: directives
//! #+INCLUDE: "~/.emacs" src emacs-lisp
//!
//!
//! These are what org mode defines in ox.el
//! More specifically, this extracts the following parameters to a
//! plist:
//! :file
//! :coding-system
//! :location
//! :only-contents
//! :lines,
//! :env
//! :minlevel
//! :args
//! :block.
//!
//! this is what we handle:
//! :file
//! :only-contents
//! :lines,
//! :minlevel
//! :args
//! :block.

use org_parser::parse_org;
use std::borrow::Cow;
use std::fs::read_to_string;
use std::num::ParseIntError;
use std::ops::Range;
use std::path::Path;
use thiserror::Error;

use org_parser::element::HeadingLevel;

use crate::types::{ExportError, ExporterInner, FileError};

/// Block types that correspond directly to types within the parser.
#[derive(Debug)]
enum IncludeBlock<'a> {
    Export { backend: Option<&'a str> },
    Example,
    Src { lang: Option<&'a str> },
}

#[derive(Debug)]
pub(crate) struct InclParams<'a> {
    /// The file path to be included
    file: &'a Path,
    /// Whether to surround the included file in a block. `block` being `None` implies the content will be
    /// parsed as org.
    block: Option<IncludeBlock<'a>>,
    // TODO
    only_contents: bool,
    /// A range of lines from the file that will be included
    lines: Option<Range<usize>>,
    // TODO
    min_level: Option<HeadingLevel>,
}

impl<'a> InclParams<'a> {
    // TODO; make error handling less... weird
    fn new(value: &'a str) -> Result<Self, IncludeError> {
        // peekable so we don't accidentally consume :kwarg params when expecting
        // positional arguments
        let mut params = value.split(" ").peekable();

        let provided_path;
        let file_chunk = params.next().ok_or(IncludeError::NoFile)?;

        // TODO: searching through file
        // account for search options
        if let Some((file_name, _search_opts)) = file_chunk.trim_matches('"').split_once("::") {
            provided_path = Path::new(file_name);
            eprintln!("Search options are not yet supported");
        } else {
            provided_path = Path::new(file_chunk);
        }

        let block: Option<IncludeBlock>;
        let is_not_kwarg = |x: &&str| !x.starts_with(':');

        block = if let Some(potential_block) = params.next_if(is_not_kwarg) {
            Some(match potential_block {
                "example" => IncludeBlock::Example,
                "export" => {
                    let backend = if let Some(potential_arg) = params.next_if(is_not_kwarg) {
                        Some(potential_arg)
                    } else {
                        // issue warning?
                        None
                    };
                    IncludeBlock::Export { backend }
                }
                "src" => {
                    let lang = if let Some(potential_lang) = params.next_if(is_not_kwarg) {
                        Some(potential_lang)
                    } else {
                        // issue warning?
                        None
                    };
                    IncludeBlock::Src { lang }
                }
                _ => Err(IncludeError::UnsupportedBlock(potential_block.into()))?,
            })
        } else {
            None
        };

        // defaults for kwargs
        let mut only_contents = false;
        let mut lines = None;
        let mut min_level = None;

        while let Some(kwarg) = params.next() {
            match kwarg {
                ":only-contents" => {
                    only_contents = if let Some(not_kwarg) = params.next_if(is_not_kwarg) {
                        not_kwarg != "nil"
                    } else {
                        // having
                        // :only-contents
                        // without any args feels like it implies truth
                        true
                    };
                }
                ":lines" => {
                    if let Some(not_kwarg) = params.next_if(is_not_kwarg) {
                        let start: usize;
                        let end: usize;

                        let hyphen_ind = not_kwarg.find('-').ok_or(IncludeError::InvalidSyntax(
                            "Lines pattern does not contain '-'".into(),
                        ))?;

                        start = if hyphen_ind == 0 {
                            0
                        } else {
                            usize::from_str_radix(&not_kwarg[..hyphen_ind], 10)?
                        };

                        end = if hyphen_ind == (not_kwarg.len() - 1) {
                            usize::MAX
                        } else {
                            usize::from_str_radix(&not_kwarg[(hyphen_ind + 1)..], 10)?
                        };

                        lines = Some(Range { start, end });
                    }
                }
                ":minlevel" => {
                    if let Some(not_kwarg) = params.next_if(is_not_kwarg) {
                        let temp = not_kwarg.parse::<usize>()?;
                        // FIXME: generalize headline level parsing with heading.rs in the parser
                        min_level = match temp {
                            1 => Some(HeadingLevel::One),
                            2 => Some(HeadingLevel::Two),
                            3 => Some(HeadingLevel::Three),
                            4 => Some(HeadingLevel::Four),
                            5 => Some(HeadingLevel::Five),
                            6 => Some(HeadingLevel::Six),
                            _ => Err(IncludeError::InvalidMinLevel { received: temp })?,
                        };
                    }
                }
                _ => Err(IncludeError::UnsupportedKwarg(kwarg.into()))?,
            }
        }

        Ok(Self {
            file: provided_path,
            block,
            only_contents,
            lines,
            min_level,
        })
    }
}

/// Key entrypoint to handle includes
///
/// - value represents the value in a keyword's key/value pair (#+key: value).
///
///   We parse this to extract what we want from an "#+include:"
/// - writer is what we use to match the desired output format & write.
///
pub(crate) fn include_handle<'a>(
    value: &str,
    writer: &mut impl ExporterInner<'a>,
) -> core::result::Result<(), IncludeError> {
    let ret = InclParams::new(value)?;
    // REVIEW: little uncomfortable reding the full string in before
    // processing lines
    let target_path: Cow<Path> = if let Some(v) = writer.config_opts().file_path().as_ref() {
        // TODO: error handling
        let temp_path = v.parent().unwrap().join(ret.file);
        temp_path
            .canonicalize()
            .map_err(|e| FileError {
                context: "failed to locate file".into(),
                path: temp_path,
                source: e,
            })?
            .into()
    } else {
        ret.file.into()
    };
    let mut out_str = read_to_string(&target_path).map_err(|e| FileError {
        context: "failed to read file".into(),
        path: target_path.into(),
        source: e,
    })?;
    if let Some(lines) = ret.lines {
        out_str = out_str
            .lines()
            .skip(lines.start)
            .take(lines.end - lines.start)
            .collect();
    }

    let feed_str;
    let parsed;

    // goal: create a string that can be parsed into our desired org object
    // HACK: for blocks, this involves wrapping the file in a #+begin_X to be interpreted literally
    // For org files, just parse it directly.

    // TODO: figure out how to not double allocate out_str
    // now it's being allocated when we read_to_string and also
    // when we format! it in a block context.
    if let Some(block) = ret.block {
        match block {
            IncludeBlock::Export { backend } => {
                if let Some(backend) = backend {
                    feed_str = format!(
                        r"#+begin_export {backend}
{out_str}
#+end_export"
                    );
                } else {
                    feed_str = format!(
                        r"#+begin_export
{out_str}
#+end_export"
                    );
                }
            }
            IncludeBlock::Example => {
                feed_str = format!(
                    r"#+begin_example
{out_str}
#+end_example"
                );
            }
            IncludeBlock::Src { lang } => {
                if let Some(lang) = lang {
                    feed_str = format!(
                        r"#+begin_src {lang}
{out_str}
#+end_src"
                    );
                } else {
                    feed_str = format!(
                        r"#+begin_src
{out_str}
#+end_src"
                    );
                }
            }
        }
    } else {
        feed_str = out_str;
    }
    // TODO: minlevel
    // TODO: only-contents

    parsed = parse_org(&feed_str);
    writer
        .export_rec(&parsed.pool.root_id(), &parsed)
        .map_err(|e| Box::new(e))?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum IncludeError {
    #[error("Invalid include syntax: {0}")]
    InvalidSyntax(String),
    #[error("No file provided")]
    NoFile,
    #[error("block `{0}` is not one of [export, example, src]")]
    UnsupportedBlock(String),
    #[error("kwarg `{0}` is not one of [:only-contents, :lines, :minlevel]")]
    UnsupportedKwarg(String),
    #[error("lines provided are not in base 10: {0}")]
    LinesError(#[from] ParseIntError),
    #[error("expected a minlevel of 1-6, received: {received}")]
    InvalidMinLevel { received: usize },
    #[error("minlevel was not a number: {0}")]
    NotStringMinlevel(String),
    #[error("{0}")]
    IoError(#[from] FileError),
    #[error("{0}")]
    FileExport(#[from] Box<ExportError>),
}
