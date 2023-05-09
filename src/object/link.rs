use crate::{
    parse::parse_object,
    types::{MarkupKind, Match, Node, ParseOpts, Parseable, Result},
};

#[derive(Debug)]
pub struct Link<'a> {
    // actually a pathreg object
    path: &'a str,
    // One or more objects enclosed by square brackets.
    // It can contain the minimal set of objects as well as export snippets,
    // inline babel calls, inline source blocks, macros, and statistics cookies.
    // It can also contain another link, but only when it is a plain or angle link.
    // It can contain square brackets, so long as they are balanced.
    description: Option<&'a str>,
}

impl<'a> Parseable<'a> for Link<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, mut parse_opts: ParseOpts) -> Result<Node> {
        parse_opts.markup.insert(MarkupKind::Link);

        let mut content_vec: Vec<Node> = Vec::new();
        let mut idx = index;
        // if we're being called, that means the first split is the thing
        idx += 1;
        loop {
            match parse_object(byte_arr, idx, parse_opts) {
                Ok(Node::MarkupEnd(leaf)) => {
                    idx = leaf.end;
                    if leaf.obj.contains(MarkupKind::Link) {
                        // close object
                        todo!()
                    } else {
                        // TODO: cache and explode
                        todo!()
                    }
                }
                Ok(val) => {
                    idx = val.get_end();
                    content_vec.push(val);
                }
                Err(_) => {
                    // cache and explode
                }
            }
        }
    }
}
