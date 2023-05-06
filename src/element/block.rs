
use crate::types::{Node, ParseOpts, Parseable, Result};

#[derive(Debug)]
pub struct Block<'a> {
    name: &'a str,
    data: Option<&'a str>,
    contents: &'a str,
}

impl<'a> Parseable<'a> for Block<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}
