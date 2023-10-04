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

use std::ops::Range;
use std::path::Path;

use org_parser::element::HeadingLevel;

#[derive(Debug)]
enum IncludeBlock<'a> {
    Export { backend: Option<&'a str> },
    Example,
    Src { lang: Option<&'a str> },
}

#[derive(Debug)]
struct InclParams<'a> {
    file: &'a Path,
    block: Option<IncludeBlock<'a>>,
    only_contents: bool,
    lines: Option<Range<usize>>,
    min_level: Option<HeadingLevel>,
}

impl<'a> InclParams<'a> {
    fn new(value: &'a str) -> Result<Self, String> {
        // todo!("parse");

        let mut params = value.split(" ").peekable();
        let provided_path;
        let file_chunk = params.next().ok_or("No file provided")?;
        if let Some((file_name, _search_opts)) = file_chunk.trim_matches('"').split_once("::") {
            provided_path = Path::new(file_name);
            eprintln!("Search options are not yet supported");
        } else {
            provided_path = Path::new(file_chunk);
        }
        // let file_spec = ;
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
                _ => Err(format!("Invalid Block name {}", potential_block))?,
            })
        } else {
            None
        };

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

                        let hyphen_ind = not_kwarg
                            .find('-')
                            .ok_or("Lines pattern does not contain '-'")?;

                        start = if hyphen_ind == 0 {
                            0
                        } else {
                            usize::from_str_radix(&not_kwarg[..hyphen_ind], 10)
                                .map_err(|_| "Failed to parse string")?
                        };

                        end = if hyphen_ind == not_kwarg.len() {
                            usize::MAX
                        } else {
                            usize::from_str_radix(&not_kwarg[(hyphen_ind + 1)..], 10)
                                .map_err(|_| "Failed to parse string")?
                        };

                        lines = Some(Range { start, end });
                    }
                }
                ":minlevel" => {
                    if let Some(not_kwarg) = params.next_if(is_not_kwarg) {
                        let temp = not_kwarg.parse::<usize>().map_err(|_| "Failed to parse")?;
                        // FIXME: generalize headline level parsing with heading.rs in the parser
                        min_level = match temp {
                            1 => Some(HeadingLevel::One),
                            2 => Some(HeadingLevel::Two),
                            3 => Some(HeadingLevel::Three),
                            4 => Some(HeadingLevel::Four),
                            5 => Some(HeadingLevel::Five),
                            6 => Some(HeadingLevel::Six),
                            _ => Err(format!("Invalid heading level {}", temp))?,
                        };
                    }
                }
                _ => Err(format!("Invalid parameter name {}", kwarg))?,
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
