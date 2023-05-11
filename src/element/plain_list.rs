use crate::{types::{Node, ParseOpts, Parseable, Result}, node_pool::NodeID};

#[derive(Debug, Clone)]
pub struct PlainList {
    contents: Vec<NodeID>,
    identation_level: u8,
}


impl<'a, 'b> Parseable<'a, 'b> for PlainList {
    fn parse(
        pool: &'b mut crate::node_pool::NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        todo!()
    }
}
