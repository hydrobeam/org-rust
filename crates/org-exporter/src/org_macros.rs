use org_parser::element::{ArgNumOrText, MacroDef};
use org_parser::object::MacroCall;
use org_parser::Parser;
use std::borrow::Cow;
use std::fs::read_to_string;
use std::path::Path;

use thiserror::Error;

use crate::types::FileError;
use crate::utils::keyword_lookup;
use crate::ConfigOptions;

pub(crate) fn macro_handle<'a>(
    parser: &'a Parser,
    macro_call: &'a MacroCall,
    config: &ConfigOptions,
) -> Result<Cow<'a, str>, MacroError> {
    match macro_call.name {
        "keyword" => {
            if macro_call.args.len() != 1 {
                Err(MacroError::InvalidParameters {
                    expected: 1,
                    received: macro_call.args.len(),
                })?
            }
            keyword_macro(parser, &macro_call.args[0])
        }
        "kw-file" => {
            if macro_call.args.len() != 2 {
                Err(MacroError::InvalidParameters {
                    expected: 2,
                    received: macro_call.args.len(),
                })?
            }
            keyword_file(&macro_call.args[0], &macro_call.args[1], config)
        }
        special_keyword @ ("title" | "author" | "email") => {
            if macro_call.args.is_empty() {
                keyword_macro(parser, special_keyword)
            } else {
                unreachable!("darn")
            }
        }
        _ => macro_execute(parser, macro_call),
    }
}

pub(crate) fn macro_execute<'a>(
    parser: &'a Parser,
    macro_call: &MacroCall<'a>,
) -> Result<Cow<'a, str>, MacroError> {
    let mac_def = parser
        .macros
        .get(macro_call.name)
        .ok_or(MacroError::UndefinedMacro {
            name: macro_call.name.into(),
        })?;

    if macro_call.args.len() != mac_def.num_args as usize {
        Err(MacroError::InvalidParameters {
            expected: mac_def.num_args as usize,
            received: macro_call.args.len(),
        })?
    }
    Ok(apply(mac_def, &macro_call.args))

    // }
}

// generate the new string and parse/export it into our current buffer.
// allows for the inclusion of objects within macros
pub fn apply<'a>(macro_def: &MacroDef, args: &[Cow<'a, str>]) -> Cow<'a, str> {
    let mut macro_contents = String::new();
    for either_enum in &macro_def.input {
        match *either_enum {
            ArgNumOrText::Text(text) => {
                macro_contents.push_str(text);
            }
            ArgNumOrText::ArgNum(num) => {
                // argnums are 1-indexed, so subtract by 1
                macro_contents.push_str(&args[(num - 1) as usize]);
            }
        }
    }

    Cow::from(macro_contents)
    // macro_contents.push('\n');
}

/// Looks up keyword name to find its corresponding value
/// invoked by macro
/// {{{keyword(NAME)}}}
pub(crate) fn keyword_macro<'a>(
    parser: &'a Parser,
    name: &'a str,
) -> Result<Cow<'a, str>, MacroError> {
    keyword_lookup(parser, name)
        .map(|x| x.into())
        .ok_or(MacroError::Keyword { kw: name.into() })
}

pub(crate) fn keyword_file<'a>(
    kw: &'a str,
    file: &'a str,
    config: &ConfigOptions,
) -> Result<Cow<'a, str>, MacroError> {
    let path = Path::new(file.trim());

    let target_path: Cow<Path> = if let Some(v) = config.file_path().as_ref() {
        // TODO: error handling
        let temp_path = v.parent().unwrap().join(path);
        temp_path
            .canonicalize()
            .map_err(|e| FileError {
                context: "Error during macro invocation of kw-file. ".into(),
                path: temp_path,
                source: e,
            })?
            .into()
    } else {
        path.into()
    };

    let out_str = read_to_string(&target_path).map_err(|e| FileError {
        context: "Error during macro invocation of kw-file. ".into(),
        path: target_path.into(),
        source: e,
    })?;
    let parsed = org_parser::parse_org(&out_str);
    parsed
        .keywords
        .get(kw)
        .map(|&f| f.to_owned().into())
        .ok_or(MacroError::Keyword { kw: kw.into() })
}

#[derive(Error, Debug)]
pub enum MacroError {
    #[error("no matching value found for keyword `{kw}`")]
    Keyword { kw: String },
    #[error("expected {expected} params, received {received} instead")]
    InvalidParameters { expected: usize, received: usize },
    #[error("call to invalid macro `{name}`")]
    UndefinedMacro { name: String },
    #[error("{0}")]
    IoError(#[from] FileError),
}
