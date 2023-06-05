use org_parser::element::Keyword;
use org_parser::object::{ArgNumOrText, MacroCall, MacroDef};
use org_parser::types::{Expr, Parser};
use std::fmt::{self, Error};

use crate::html::HtmlEscape;
use crate::utils::keyword_lookup;

pub(crate) fn macro_handle<'a>(
    parser: &'a Parser,
    macro_call: &'a MacroCall,
    buf: &mut dyn fmt::Write,
) -> fmt::Result {
    match macro_call.name {
        "keyword" => keyword_macro(parser, macro_call.args[0], buf),
        special_keyword @ ("title" | "author" | "email") => {
            if macro_call.args.len() == 0 {
                keyword_macro(parser, special_keyword, buf)
            } else {
                Err(Error)
            }
        }
        _ => macro_execute(parser, macro_call, buf),
    }
}

pub(crate) fn macro_execute<'a>(
    parser: &'a Parser,
    macro_call: &MacroCall,
    buf: &mut dyn fmt::Write,
) -> fmt::Result {
    let macid = parser.macros.get(macro_call.name).unwrap();
    // FIXME: pretty janky, but have to do this dance cause of NodeID

    if let Expr::Keyword(temp) = &parser.pool[*macid].obj {
        if let Keyword::Macro(mac_def) = temp {
            if macro_call.args.len() == mac_def.num_args as usize {
                apply(mac_def, &macro_call.args, buf)?;
            } else {
                return Err(Error);
            }
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    }

    Ok(())
}

pub fn apply(macro_def: &MacroDef, args: &Vec<&str>, buf: &mut dyn fmt::Write) -> fmt::Result {
    for either_enum in &macro_def.input {
        match *either_enum {
            ArgNumOrText::Text(text) => {
                write!(buf, "{}", HtmlEscape(text))?;
            }
            ArgNumOrText::ArgNum(num) => {
                write!(buf, "{}", HtmlEscape(args[(num - 1) as usize]))?;
            }
        }
    }

    Ok(())
}

/// Looks up keyword name to find its corresponding value
/// invoked by macro
/// {{{keyword(NAME)}}}
pub(crate) fn keyword_macro<'a>(
    parser: &'a Parser,
    name: &'a str,
    buf: &mut dyn fmt::Write,
) -> fmt::Result {
    // not finding a keyword doesn't blow up the exporter in org-mode
    //
    // TODO: warning system so invalid macros are caught
    if let Some(keyword_val) = keyword_lookup(parser, name) {
        write!(buf, "{}", HtmlEscape(keyword_val))
    } else {
        Ok(())
    }
}
