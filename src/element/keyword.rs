use crate::types::{Node, ParseOpts, Parseable, Result};

#[derive(Debug)]
pub struct Keyword<'a> {
    key: &'a str,
    val: &'a str,
}

impl<'a> Parseable<'a> for Keyword<'a> {
    fn parse(byte_arr: &[u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
