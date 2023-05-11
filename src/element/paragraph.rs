use crate::{types::{Node, ParseOpts, Parseable, Result}, node_pool::NodeID};

#[derive(Debug, Clone)]
pub struct Paragraph(pub Vec<NodeID>);

impl<'a, 'b> Parseable<'a, 'b> for Paragraph {
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
