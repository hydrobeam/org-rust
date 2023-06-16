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
        cursor.word("<<")?;
        if cursor.peek(1)?.is_ascii_whitespace() {
            return Err(MatchError::InvalidLogic);
        }

        let inner_target_match =
            cursor.fn_until(|chr: u8| chr == NEWLINE || chr == RANGLE || chr == LANGLE)?;
        cursor.index = inner_target_match.end;
        cursor.word(">>")?;

        let ret_id = parser.alloc(Self(inner_target_match.obj), start, cursor.index, parent);

        parser.pool[ret_id].id_target = Some(parser.generate_target(inner_target_match.obj));
        return Ok(ret_id);
    }
}
