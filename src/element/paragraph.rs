use crate::types::{Node, ParseOpts, Parseable, Result};

#[derive(Debug)]
pub struct Paragraph<'a> {
    pub contents: Vec<Node<'a>>
}

impl<'a> Parseable<'a> for Paragraph<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
