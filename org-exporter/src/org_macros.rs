use org_parser::element::Keyword;
use org_parser::object::MacroCall;
use org_parser::types::{Expr, Parser};
use std::borrow::Cow;

pub(crate) fn macro_handle<'a>(parser: &'a Parser, macro_call: &'a MacroCall) -> Cow<'a, str> {
    match macro_call.name {
        "keyword" => Cow::from(keyword_lookup(parser, macro_call.name)),
        _ => macro_execute(parser, macro_call),
    }
}

pub(crate) fn macro_execute<'a>(parser: &'a Parser, macro_call: &MacroCall) -> Cow<'a, str> {
    let macid = parser.macros.get(macro_call.name).unwrap();
    // FIXME: pretty janky, but have to do this dance cause of NodeID
    if let Expr::Keyword(temp) = &parser.pool[*macid].obj {
        if let Keyword::Macro(mac) = temp {
            mac.apply(&macro_call.args)
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    }
}

/// Looks up keyword name to find its corresponding value
/// invoked by macro
/// {{{keyword(NAME)}}}
pub(crate) fn keyword_lookup<'a>(parser: &'a Parser, name: &'a str) -> &'a str {
    parser.keywords.get(name).unwrap()
}
