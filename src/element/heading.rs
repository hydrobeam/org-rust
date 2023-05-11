use crate::node_pool::{NodeID, NodePool};
use crate::types::{ParseOpts, Parseable, Result};

// STARS KEYWORD PRIORITY TITLE TAGS
#[derive(Debug, Clone)]
pub struct Heading<'a> {
    level: u8,
    // Org-Todo type stuff
    keyword: Option<&'a str>,
    priority: Option<char>,
    title: Option<Vec<NodeID>>,
}

impl<'a> Parseable<'a> for Heading<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
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
