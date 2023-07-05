use crate::constants::{EQUAL, NEWLINE, TILDE};
use crate::node_pool::NodeID;
use crate::parse::{parse_element, parse_object};
use crate::types::{Cursor, MarkupKind, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::verify_markup;

macro_rules! recursive_markup {
    ($name: tt) => {
        #[derive(Debug, Clone)]
        pub struct $name(pub Vec<NodeID>);

        impl<'a> Parseable<'a> for $name {
            fn parse(
                parser: &mut Parser<'a>,
                mut cursor: Cursor<'a>,
                parent: Option<NodeID>,
                mut parse_opts: ParseOpts,
            ) -> Result<NodeID> {
                if !verify_markup(cursor, false) {
                    return Err(MatchError::InvalidLogic);
                }
                let start = cursor.index;
                cursor.next();

                parse_opts.from_object = false;
                parse_opts.markup.insert(MarkupKind::$name);

                let mut content_vec: Vec<NodeID> = Vec::new();
                loop {
                    match parse_object(parser, cursor, parent, parse_opts) {
                        Ok(id) => {
                            cursor.index = parser.pool[id].end;
                            content_vec.push(id);
                        }
                        Err(MatchError::MarkupEnd(kind)) => {
                            if !kind.contains(MarkupKind::$name) || cursor.index < start + 2
                            // prevent ** from being Bold{}
                            {
                                return Err(MatchError::InvalidLogic);
                            }

                            // the markup is going to exist,
                            // so update the children's parents
                            let new_id = parser.pool.reserve_id();
                            for id in content_vec.iter_mut() {
                                parser.pool[*id].parent = Some(new_id)
                            }

                            return Ok(parser.alloc_with_id(
                                Self(content_vec),
                                start,
                                cursor.index + 1,
                                parent,
                                new_id,
                            ));
                        }
                        ret @ Err(_) => {
                            return ret;
                        }
                    }
                }
            }
        }
    };
}

/// $name is the name of the Markup object e.g. Code
/// $byte is the closing delimeter for the markup object, e.g. TILDE
macro_rules! plain_markup {
    ($name: tt, $byte: tt) => {

        #[derive(Debug, Clone, Copy)]
        pub struct $name<'a>(pub &'a str);

        impl<'a> Parseable<'a> for $name<'a> {
            fn parse(
                parser: &mut Parser<'a>,
                mut cursor: Cursor<'a>,
                parent: Option<NodeID>,
                mut parse_opts: ParseOpts,
            ) -> Result<NodeID> {
                if !verify_markup(cursor, false) {
                    return Err(MatchError::InvalidLogic);
                }

                // skip the opening character, we checked it's valid markup
                parse_opts.markup.insert(MarkupKind::$name);

                let start = cursor.index;
                cursor.next();

                loop {
                    match cursor.try_curr()? {
                        chr if parse_opts.markup.byte_match(chr) => {
                            if chr == $byte // check if our closer  is active
                                && cursor.index > start + 1 // prevent ~~ from being Bold{}
                                && verify_markup(cursor, true) {
                                break;
                            } else {
                                // FIXME: doesn't handle link end.
                                // [[___][~abc ] amc~ ]]
                                // won't make one cohesive code object, the rbrack will
                                // kill it
                                return Err(MatchError::MarkupEnd(parse_opts.markup));
                            }
                        }
                        NEWLINE => {
                            parse_opts.from_paragraph = true;
                            parse_opts.from_object = false;
                            parse_opts.list_line = false;
                            match parse_element(parser, cursor.adv_copy(1), parent, parse_opts) {
                                Ok(_) => return Err(MatchError::InvalidLogic),
                                Err(MatchError::InvalidLogic) => {
                                    cursor.next();
                                }
                                ret @ Err(_) => return ret,
                            }
                        }
                        _ => {
                            cursor.next();
                        }
                    }
                }

                Ok(parser.alloc(
                    Self(cursor.clamp_backwards(start + 1)),
                    start,
                    cursor.index + 1,
                    parent,
                ))
            }
        }
    };
}

recursive_markup!(Italic);
recursive_markup!(Bold);
recursive_markup!(StrikeThrough);
recursive_markup!(Underline);

plain_markup!(Code, TILDE);
plain_markup!(Verbatim, EQUAL);

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_verbatim() {
        let inp = "=hello_world=";

        dbg!(parse_org(inp));
    }

    #[test]
    fn basic_code() {
        let inp = "~hello_world~";

        dbg!(parse_org(inp));
    }
    #[test]
    fn basic_italic() {
        let inp = "/hello_world/";

        dbg!(parse_org(inp));
    }
    #[test]
    fn basic_bold() {
        let inp = "*hello_world*";

        dbg!(parse_org(inp));
    }
    #[test]
    fn basic_underline() {
        let inp = "_hello_world_";

        dbg!(parse_org(inp));
    }
    #[test]
    fn basic_strikethrough() {
        let inp = "+hello_world+";

        dbg!(parse_org(inp));
    }

    #[test]
    fn markup_recursive_empty() {
        let inp = "**";

        let pool = parse_org(inp);
        pool.print_tree();
    }

    #[test]
    fn markup_plain_empty() {
        let inp = "~~";

        let pool = parse_org(inp);
        pool.print_tree();
    }

    #[test]
    fn nested_markup() {
        let inp = "abc /one *two* three/ four";

        let pool = parse_org(inp);
        pool.print_tree();
    }

    #[test]
    fn leaky_markup() {
        let inp = "abc /one *two thr/ ee* three four";

        let pool = parse_org(inp);
        pool.print_tree();
    }

    #[test]
    fn mixed_plain_recursive_leaky_markup() {
        let inp = "abc /one ~two thr/ ee~ three four";

        let pool = parse_org(inp);
        pool.print_tree();
    }
    // #[test]
    // fn
    #[test]
    fn markup_not_fail_on_eof() {
        let inp = "/";
        let pool = parse_org(inp);

        pool.print_tree();
    }

    #[test]
    fn markup_plain_single_char() {
        // should be valid
        let inp = "~a~";
        let pool = parse_org(inp);

        pool.print_tree();
    }

    #[test]
    fn markup_recursive_single_char() {
        // should be valid
        let inp = "/a/";
        let pool = parse_org(inp);

        pool.print_tree();
    }
}
