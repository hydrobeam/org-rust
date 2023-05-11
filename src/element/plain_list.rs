use crate::{
    node_pool::{NodeID, NodePool},
    types::{ParseOpts, Parseable, Result},
};

#[derive(Debug, Clone)]
pub struct PlainList {
    contents: Vec<NodeID>,
    identation_level: u8,
}

impl<'a> Parseable<'a> for PlainList {
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
