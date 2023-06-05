use crate::constants::{LANGLE, RANGLE};
use crate::node_pool::NodeID;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Copy, Clone)]
pub struct Target<'a>(pub &'a str);

impl<'a> Parseable<'a> for Target<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        if cursor.curr() != LANGLE
            || cursor.peek(1)? != LANGLE
            || cursor.peek(2)?.is_ascii_whitespace()
        {
            return Err(MatchError::InvalidLogic);
        }
        cursor.advance(2);

        let ret = cursor
            .fn_until(|chr: u8| chr.is_ascii_whitespace() || chr == RANGLE || chr == LANGLE)?;

        if cursor[ret.end].is_ascii_whitespace() || cursor[ret.end] == LANGLE {
            return Err(MatchError::InvalidLogic);
        }
        cursor.index = ret.end;

        // now we must have a RANGLE at cursor.curr()
        // so, handle the next element
        match cursor.peek(1)? {
            RANGLE => {
                parser.targets.insert(ret.obj, ret.obj);
                Ok(parser.alloc(Self(ret.obj), start, cursor.index + 2, parent))
            }
            _ => Err(MatchError::InvalidLogic),
        }
    }
}
