use crate::constants::{EQUAL, NEWLINE, TILDE};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::{parse_element, parse_object};
use crate::types::{Cursor, Expr, MarkupKind, MatchError, ParseOpts, Parseable, Result};
use crate::utils::verify_markup;

#[derive(Debug, Clone)]
pub struct Italic(pub Vec<NodeID>);

#[derive(Debug, Clone)]
pub struct Bold(pub Vec<NodeID>);

#[derive(Debug, Clone)]
pub struct StrikeThrough(pub Vec<NodeID>);

#[derive(Debug, Clone)]
pub struct Underline(pub Vec<NodeID>);

#[derive(Debug, Clone, Copy)]
pub struct Verbatim<'a>(pub &'a str);

#[derive(Debug, Clone, Copy)]
pub struct Code<'a>(pub &'a str);

macro_rules! recursive_markup {
    ($name: tt) => {
        impl<'a> Parseable<'a> for $name {
            fn parse(
                pool: &mut NodePool<'a>,
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
                // if we're being called, that means the first split is the thing
                loop {
                    match parse_object(pool, cursor, parent, parse_opts) {
                        Ok(id) => {
                            let node = &pool[id];
                            cursor.move_to(node.end);
                            if let Expr::MarkupEnd(leaf) = node.obj {
                                if leaf.contains(MarkupKind::$name) && cursor.index > start + 2
                                // prevent ** from being Bold{}
                                {
                                    // TODO: abstract this?
                                    let new_id = pool.reserve_id();
                                    for id in content_vec.iter_mut() {
                                        pool[*id].parent = Some(new_id)
                                    }
                                    // we can't just get the next ID because alloc_with_id assumes the node is safe to mutate
                                    // and we can't access an index beyond the len of the list.
                                    return Ok(pool.alloc_with_id(
                                        Self(content_vec),
                                        start,
                                        cursor.index,
                                        parent,
                                        new_id,
                                    ));
                                } else {
                                    return Err(MatchError::InvalidLogic);
                                }
                            } else {
                                content_vec.push(id);
                            }
                        }
                        Err(_) => {
                            return Err(MatchError::InvalidLogic);
                            // cache and explode
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
        impl<'a> Parseable<'a> for $name<'a> {
            fn parse(
                pool: &mut NodePool<'a>,
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
                                    return Err(MatchError::InvalidLogic);
                            }
                        }
                        NEWLINE => {
                            parse_opts.from_paragraph = true;
                            match parse_element(pool, cursor.adv_copy(1), parent, parse_opts) {
                                Ok(_) => return Err(MatchError::InvalidLogic),
                                Err(MatchError::InvalidLogic) => {
                                    cursor.next();
                                }
                                Err(MatchError::EofError) => return Err(MatchError::EofError),
                                Err(MatchError::InvalidIndentation) =>
                                    return Err(MatchError::InvalidIndentation),
                            }
                        }
                        _ => {
                            cursor.next();
                        }
                    }
                }

                Ok(pool.alloc(
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

        let a = parse_org(inp);
        a.root().print_tree(&a);
    }

    #[test]
    fn markup_plain_empty() {
        let inp = "~~";

        let a = parse_org(inp);
        a.root().print_tree(&a);
    }

    #[test]
    fn nested_markup() {
        let inp = "abc /one *two* three/ four";

        let a = parse_org(inp);
        a.root().print_tree(&a);
    }

    #[test]
    fn leaky_markup() {
        let inp = "abc /one *two thr/ ee* three four";

        let a = parse_org(inp);
        a.root().print_tree(&a);
    }

    #[test]
    fn mixed_plain_recursive_leaky_markup() {
        let inp = "abc /one ~two thr/ ee~ three four";

        let a = parse_org(inp);
        a.root().print_tree(&a);
    }
    // #[test]
    // fn
    #[test]
    fn markup_not_fail_on_eof() {
        let inp = "/";
        let a = parse_org(inp);

        a.root().print_tree(&a);
    }

    #[test]
    fn markup_plain_single_char() {
        // should be valid
        let inp = "~a~";
        let a = parse_org(inp);

        a.root().print_tree(&a);
    }

    #[test]
    fn markup_recursive_single_char() {
        // should be valid
        let inp = "/a/";
        let a = parse_org(inp);

        a.root().print_tree(&a);
    }
}
