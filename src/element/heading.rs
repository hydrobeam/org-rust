use crate::types::{Node, ParseOpts, Parseable, Result};

// STARS KEYWORD PRIORITY TITLE TAGS
#[derive(Debug)]
pub struct Heading<'a> {
    level: u8,
    // Org-Todo type stuff
    keyword: Option<&'a str>,
    priority: Option<char>,
    title: Option<Vec<Node<'a>>>,
}

impl<'a> Parseable<'a> for Heading<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        todo!()
    }
}

impl<'a> Heading<'a> {
    fn parse_stars() {
        todo!()
    }
    fn parse_keyword() {
        todo!()
    }
    fn parse_priority() {
        todo!()
    }
    fn parse_tag() {
        todo!()
    }
}
