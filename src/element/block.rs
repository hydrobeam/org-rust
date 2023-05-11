
use crate::types::{Node, ParseOpts, Parseable, Result};

#[derive(Debug, Clone)]
pub struct Block<'a> {
    name: &'a str,
    data: Option<&'a str>,
    contents: &'a str,
}

impl<'a> Parseable<'a> for Block<'a> {
    fn parse(
        pool: &std::cell::RefCell<crate::node_pool::NodePool<'a>>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<crate::node_pool::NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<crate::node_pool::NodeID> {
        todo!()
    }
}
