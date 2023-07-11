use crate::constants::NEWLINE;
use crate::node_pool::NodeID;
use crate::parse::parse_object;
use crate::types::{Cursor, Expr, MatchError, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Clone)]
pub struct TableCell(pub Vec<NodeID>);

impl<'a> Parseable<'a> for TableCell {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        let mut content_vec: Vec<NodeID> = Vec::new();
        loop {
            match parse_object(parser, cursor, parent, parse_opts) {
                Ok(id) => {
                    cursor.index = parser.pool[id].end;
                    content_vec.push(id);
                }
                Err(MatchError::MarkupEnd(kind)) => {
                    // table cells can end on both a vbar and a newline
                    // a newline indicates the start of the next row
                    // we can't skip past it so that tablerow
                    // has a signal to know when it ends (a table cell ending in a newline)
                    if cursor.curr() != NEWLINE {
                        cursor.next();
                    }
                    break;
                }
                Err(_) => break,
            }
        }

        // set parents of children
        // TODO: abstract this? stolen from markup.rs
        let new_id = parser.pool.reserve_id();
        for id in &mut content_vec {
            parser.pool[*id].parent = Some(new_id);
        }

        // get rid of alignment spaces, deleting the object if it becomes empty
        if let Some(last_id) = content_vec.last() {
            let last_item = &mut parser.pool[*last_id];
            if let Expr::Plain(plains) = last_item.obj {
                let repl_str = plains.trim_end();
                if repl_str.trim_end().is_empty() {
                    content_vec.pop();
                } else {
                    last_item.obj = Expr::Plain(repl_str);
                }
            }
        }

        Ok(parser
            .pool
            .alloc_with_id(Self(content_vec), start, cursor.index, parent, new_id))
    }
}
