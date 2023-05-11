use crate::{
    node_pool::{NodeID, NodePool},
    parse::parse_object,
    types::{Expr, MarkupKind, ParseOpts, Parseable, Result},
};

#[derive(Debug, Clone)]
pub struct Link<'a> {
    // actually a pathreg object
    path: &'a str,
    // One or more objects enclosed by square brackets.
    // It can contain the minimal set of objects as well as export snippets,
    // inline babel calls, inline source blocks, macros, and statistics cookies.
    // It can also contain another link, but only when it is a plain or angle link.
    // It can contain square brackets, so long as they are balanced.
    description: Option<Vec<NodeID>>,
}

impl<'a> Parseable<'a> for Link<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        parse_opts.markup.insert(MarkupKind::Link);

        let mut content_vec: Vec<NodeID> = Vec::new();
        let mut idx = index;
        // if we're being called, that means the first split is the thing
        idx += 1;
        loop {
            if let Ok(id) = parse_object(pool, byte_arr, idx, parent, parse_opts) {
                idx = pool[id].end;
                if let Expr::MarkupEnd(leaf) = pool[id].obj {
                    if leaf.contains(MarkupKind::Link) {
                        // close object
                        todo!()
                    } else {
                        // TODO: cache and explode
                        todo!()
                    }
                } else {
                    content_vec.push(id);
                }
            }
        }
    }
}
