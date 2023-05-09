use crate::types::{Node, ParseOpts, Parseable, Result};

#[derive(Debug)]
pub struct PlainList<'a> {
    contents: Vec<Node<'a>>,
    identation_level: u8,
}


impl<'a> Parseable<'a> for PlainList<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
