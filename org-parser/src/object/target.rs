use crate::constants::{LANGLE, NEWLINE, RANGLE};
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

        let ret = cursor.fn_until(|chr: u8| chr == NEWLINE || chr == RANGLE || chr == LANGLE)?;

        cursor.index = ret.end;
        if matches!(cursor.curr(), NEWLINE | LANGLE) {
            return Err(MatchError::InvalidLogic);
        }

        // now we must have a RANGLE at cursor.curr()
        // so, handle the next element
        match cursor.peek(1)? {
            RANGLE => {
                let ret_id = parser.alloc(Self(ret.obj), start, cursor.index + 2, parent);

                // TODO: use builder pattern to not have to do this:
                // don't want to modify every invocation of alloc to include target_id
                parser.pool[ret_id].id_target = Some(parser.generate_target(ret.obj));
                Ok(ret_id)
            }
            _ => Err(MatchError::InvalidLogic),
        }
    }
}
