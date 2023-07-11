use org_parser::element::{ArgNumOrText, MacroDef};
use org_parser::object::MacroCall;
use org_parser::{Expr, Parser};
use std::borrow::Cow;

use crate::utils::keyword_lookup;

pub(crate) fn macro_handle<'a>(
    parser: &'a Parser,
    macro_call: &'a MacroCall,
) -> Result<Cow<'a, str>, ()> {
    match macro_call.name {
        "keyword" => Ok(keyword_macro(parser, macro_call.args[0])?),
        special_keyword @ ("title" | "author" | "email") => {
            if macro_call.args.is_empty() {
                Ok(keyword_macro(parser, special_keyword)?)
            } else {
                Err(())
            }
        }
        _ => Ok(macro_execute(parser, macro_call)?),
    }
}

pub(crate) fn macro_execute<'a>(
    parser: &'a Parser,
    macro_call: &MacroCall<'a>,
) -> Result<Cow<'a, str>, ()> {
    let macid = parser.macros.get(macro_call.name).unwrap();
    // FIXME: pretty janky, but have to do this dance cause of NodeID

    if let Expr::MacroDef(mac_def) = &parser.pool[*macid].obj {
        if macro_call.args.len() == mac_def.num_args as usize {
            Ok(apply(mac_def, &macro_call.args))
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}

// generate the new string and parse/export it into our current buffer.
// allows for the inclusion of objects within macros
pub fn apply<'a>(macro_def: &MacroDef, args: &[&'a str]) -> Cow<'a, str> {
    let mut macro_contents = String::new();
    for either_enum in &macro_def.input {
        match *either_enum {
            ArgNumOrText::Text(text) => {
                macro_contents.push_str(text);
            }
            ArgNumOrText::ArgNum(num) => {
                // argnums are 1-indexed, so subtract by 1
                macro_contents.push_str(args[(num - 1) as usize]);
            }
        }
    }

    Cow::from(macro_contents)
    // macro_contents.push('\n');
}

/// Looks up keyword name to find its corresponding value
/// invoked by macro
/// {{{keyword(NAME)}}}
pub(crate) fn keyword_macro<'a>(parser: &'a Parser, name: &'a str) -> Result<Cow<'a, str>, ()> {
    // not finding a keyword doesn't blow up the exporter in org-mode
    //
    // TODO: warning system so invalid macros are caught
    if let Some(keyword_val) = keyword_lookup(parser, name) {
        Ok(Cow::from(keyword_val))
    } else {
        Err(())
    }
}
