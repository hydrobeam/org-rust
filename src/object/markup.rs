use crate::constants::{EQUAL, NEWLINE, TILDE};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::{parse_element, parse_object};
use crate::types::{Expr, MarkupKind, MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, verify_markup};

#[derive(Debug, Clone)]
pub struct Italic(pub Vec<NodeID>);

#[derive(Debug, Clone)]
pub struct Bold(pub Vec<NodeID>);

#[derive(Debug, Clone)]
pub struct StrikeThrough(pub Vec<NodeID>);

#[derive(Debug, Clone)]
pub struct Underline(pub Vec<NodeID>);

#[derive(Debug, Clone, Copy)]
pub struct Verbatim<'a>(&'a str);

#[derive(Debug, Clone, Copy)]
pub struct Code<'a>(&'a str);

macro_rules! recursive_markup {
    ($name: tt) => {
        impl<'a> Parseable<'a> for $name {
            fn parse(
                pool: &mut NodePool<'a>,
                byte_arr: &'a [u8],
                index: usize,
                parent: Option<NodeID>,
                mut parse_opts: ParseOpts,
            ) -> Result<NodeID> {
                if !verify_markup(byte_arr, index, false) {
                    return Err(MatchError::InvalidLogic);
                }
                parse_opts.from_object = false;
                parse_opts.markup = MarkupKind::$name;
                parse_opts.markup.insert(MarkupKind::$name);

                let mut content_vec: Vec<NodeID> = Vec::new();
                let mut idx = index;
                // if we're being called, that means the first split is the thing
                idx += 1;
                loop {
                    match parse_object(pool, byte_arr, idx, parent, parse_opts) {
                        Ok(id) => {
                            let node = &pool[id];
                            idx = node.end;
                            if let Expr::MarkupEnd(leaf) = node.obj {
                                if leaf.contains(MarkupKind::$name) {
                                    // TODO: abstract this?
                                    let new_id = pool.reserve_id();
                                    for id in content_vec.iter_mut() {
                                        pool[*id].parent = Some(new_id)
                                    }
                                    // we can't just get the next ID because alloc_with_id assumes the node is safe to mutate
                                    // and we can't access an index beyond the len of the list.
                                    return Ok(pool.alloc_with_id(
                                        Self(content_vec),
                                        index,
                                        idx,
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

macro_rules! plain_markup {
    ($name: tt, $byte: tt) => {
        impl<'a> Parseable<'a> for $name<'a> {
            fn parse(
                pool: &mut NodePool<'a>,
                byte_arr: &'a [u8],
                index: usize,
                parent: Option<NodeID>,
                mut parse_opts: ParseOpts,
            ) -> Result<NodeID> {
                if !verify_markup(byte_arr, index, false) {
                    return Err(MatchError::InvalidLogic);
                }

                // skip the opening character, we checked it's valid markup

                let mut idx = index + 1;

                loop {
                    match *byte_arr.get(idx).ok_or(MatchError::EofError)? {
                        $byte => {
                            if verify_markup(byte_arr, idx, true) {
                                break;
                            } else {
                                idx += 1;
                            }
                        }
                        NEWLINE => {
                            parse_opts.from_paragraph = true;
                            match parse_element(pool, byte_arr, idx + 1, parent, parse_opts) {
                                Ok(_) => return Err(MatchError::InvalidLogic),
                                Err(MatchError::InvalidLogic) => {
                                    idx += 1;
                                }
                                Err(MatchError::EofError) => return Err(MatchError::EofError),
                            }
                        }
                        _ => {
                            idx += 1;
                        }
                    }
                }

                Ok(pool.alloc(
                    Self(bytes_to_str(&byte_arr[index + 1..idx])),
                    index + 1,
                    idx + 1,
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
}
