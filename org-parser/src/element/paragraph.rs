use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_object;
use crate::types::{Cursor, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Clone)]
pub struct Paragraph(pub Vec<NodeID>);

impl<'a> Parseable<'a> for Paragraph {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        let mut content_vec: Vec<NodeID> = Vec::new();
        parse_opts.from_paragraph = true;

        // allocte beforehand since we know paragrpah can never fail
        let new_id = parser.pool.reserve_id();

        while let Ok(id) = parse_object(parser, cursor, Some(new_id), parse_opts) {
            cursor.index = parser.pool[id].end;
            content_vec.push(id);
        }

        Ok(parser.alloc_with_id(
            Paragraph(content_vec),
            start,
            cursor.index + 1, // newline
            parent,
            new_id,
        ))
    }
}
