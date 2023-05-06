use crate::types::{Node, ParseOpts, Parseable, Result};

#[derive(Debug)]
pub struct Comment<'a> {
    content: &'a str,
}

impl<'a> Parseable<'a> for Comment<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
