use org_parser::element::{ArgNumOrText, MacroDef};
use org_parser::object::MacroCall;
use org_parser::Parser;
use std::borrow::Cow;
use std::fs::read_to_string;
use std::path::Path;

use crate::utils::keyword_lookup;
use crate::ConfigOptions;

pub(crate) fn macro_handle<'a>(
    parser: &'a Parser,
    macro_call: &'a MacroCall,
    config: &ConfigOptions,
) -> Result<Cow<'a, str>, Box<dyn std::error::Error>> {
    match macro_call.name {
        "keyword" => keyword_macro(parser, &macro_call.args[0]),
        "kw-file" => keyword_file(&macro_call.args[0], &macro_call.args[1], config),
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
) -> Result<Cow<'a, str>, Box<dyn std::error::Error>> {
    let mac_def = parser.macros.get(macro_call.name).unwrap();
    // FIXME: pretty janky, but have to do this dance to get the macrodef for the macrocall

    // if let Expr::MacroDef(mac_def) = &parser.pool[*macid].obj {
    if macro_call.args.len() == mac_def.num_args as usize {
        Ok(apply(mac_def, &macro_call.args))
    } else {
        unreachable!("darn")
    }

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
) -> Result<Cow<'a, str>, Box<dyn std::error::Error>> {
    // not finding a keyword doesn't blow up the exporter in org-mode
    //
    // TODO: warning system so invalid macros are caught
    Ok(keyword_lookup(parser, name)
        .map(|x| x.into())
        .expect("whoops"))
}

pub fn keyword_file<'a>(
    kw: &'a str,
    file: &'a str,
    config: &ConfigOptions,
) -> Result<Cow<'a, str>, Box<dyn std::error::Error>> {
    let path = Path::new(file.trim());

    let target_path: Cow<Path> = if let Some(v) = config.file_path().as_ref() {
        // TODO: error handling
        v.parent().unwrap().join(path).canonicalize()?.into()
    } else {
        path.into()
    };

    let out_str = read_to_string(target_path)?;
    let parsed = org_parser::parse_org(&out_str);
    if let Some(&val) = parsed.keywords.get(kw) {
        Ok(Cow::from(val.to_owned()))
    } else {
        todo!("whoopsies")
    }
}
