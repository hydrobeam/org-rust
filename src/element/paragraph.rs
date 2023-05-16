use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_object;
use crate::types::{ParseOpts, Parseable, Result};

#[derive(Debug, Clone)]
pub struct Paragraph(pub Vec<NodeID>);

impl<'a> Parseable<'a> for Paragraph {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let mut content_vec: Vec<NodeID> = Vec::new();
        let mut idx = index;

        // allocte beforehand since we know paragrpah can never fail
        let new_id = pool.reserve_id();

        while let Ok(id) = parse_object(pool, byte_arr, idx, Some(new_id), parse_opts) {
            idx = pool[id].end;
            content_vec.push(id);
        }

        Ok(pool.alloc_with_id(
            Paragraph(content_vec),
            index,
            idx + 1, // newline
            parent,
            new_id,
        ))
    }
}
