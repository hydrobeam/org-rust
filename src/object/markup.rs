use crate::{
    constants::{self, EQUAL, NEWLINE, TILDE},
    parse::{parse_element, parse_object},
    types::{MarkupKind, MatchError, Node, ParseOpts, Parseable, Result},
    utils::{bytes_to_str, verify_markup},
};

#[derive(Debug)]
pub struct Italic<'a>(Vec<Node<'a>>);

#[derive(Debug)]
pub struct Bold<'a>(Vec<Node<'a>>);

#[derive(Debug)]
pub struct StrikeThrough<'a>(Vec<Node<'a>>);

#[derive(Debug)]
pub struct Underline<'a>(Vec<Node<'a>>);

#[derive(Debug, Clone, Copy)]
pub struct Verbatim<'a>(&'a str);

#[derive(Debug, Clone, Copy)]
pub struct Code<'a>(&'a str);

macro_rules! recursive_markup {
    ($name: tt) => {
        impl<'a> Parseable<'a> for $name<'a> {
            fn parse(byte_arr: &'a [u8], index: usize, mut parse_opts: ParseOpts) -> Result<Node> {
                parse_opts.markup.insert(MarkupKind::$name);

                let mut content_vec: Vec<Node> = Vec::new();
                let mut idx = index;
                // if we're being called, that means the first split is the thing
                idx += 1;
                loop {
                    match parse_object(byte_arr, idx, parse_opts) {
                        Ok(Node::MarkupEnd(leaf)) => {
                            idx = leaf.end;
                            if leaf.obj.contains(MarkupKind::$name) {
                                return Ok(Node::make_branch(Self(content_vec), index, idx));
                            } else {
                                return Err(MatchError::InvalidLogic);
                            }
                        }
                        Ok(ret) => {
                            idx = ret.get_end();
                            content_vec.push(ret);
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
            fn parse(byte_arr: &'a [u8], index: usize, mut parse_opts: ParseOpts) -> Result<Node> {
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
                            match parse_element(byte_arr, idx + 1, parse_opts) {
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

                Ok(Node::make_leaf(
                    Self(bytes_to_str(&byte_arr[index + 1..idx])),
                    index + 1,
                    idx + 1,
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

    use super::*;

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
