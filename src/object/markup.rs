use crate::{
    parse::{parse_element, parse_object},
    types::{Leaf, MarkupKind, MatchError, Node, ParseOpts, Parseable, Result},
};
use bitflags::bitflags;

#[derive(Debug)]

pub struct Italic<'a> {
    contents: Vec<Node<'a>>,
}

#[derive(Debug)]
pub struct Bold<'a> {
    contents: Vec<Node<'a>>,
}

#[derive(Debug)]
pub struct StrikeThrough<'a> {
    contents: Vec<Node<'a>>,
}

#[derive(Debug)]
pub struct Underline<'a> {
    contents: Vec<Node<'a>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Verbatim<'a> {
    contents: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct Code<'a> {
    contents: &'a str,
}

impl<'a> Parseable<'a> for Italic<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, mut parse_opts: ParseOpts) -> Result<Node> {
        parse_opts.markup.insert(MarkupKind::Italic);

        let mut content_vec: Vec<Node> = Vec::new();
        let mut idx = index;
        // if we're being called, that means the first split is the thing
        idx += 1;
        loop {
            match parse_object(byte_arr, idx, parse_opts) {
                Ok(Node::Leaf(leaf)) => {
                    if let Leaf::MarkupEnd(kind) = leaf.obj {
                        idx = leaf.end;
                        if kind.contains(MarkupKind::Italic) {
                            return Ok(Node::make_branch(
                                Self {
                                    contents: content_vec,
                                },
                                index,
                                idx,
                            ));
                        } else {
                            return Err(MatchError::InvalidLogic);
                        }
                    } else {
                        idx = leaf.end;
                        content_vec.push(Node::Leaf(leaf))
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

impl<'a> Parseable<'a> for Bold<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
impl<'a> Parseable<'a> for Underline<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
impl<'a> Parseable<'a> for StrikeThrough<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
impl<'a> Parseable<'a> for Code<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
impl<'a> Parseable<'a> for Verbatim<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
