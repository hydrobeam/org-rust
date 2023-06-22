use crate::node_pool::NodeID;
use crate::types::{Cursor, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Clone, Copy)]
pub struct Comment<'a>(pub &'a str);

impl<'a> Parseable<'a> for Comment<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        if cursor.peek(1)?.is_ascii_whitespace() {
            // skip past "# "
            cursor.advance(2);
            let content = cursor.fn_until(|chr: u8| chr == b'\n')?;
            // TODO: use an fn_until_inclusive to not have to add 1 to the end
            // (we want to eat the ending nl too)
            Ok(parser.alloc(Self(content.obj), start, content.end + 1, parent))
        } else {
            Err(crate::types::MatchError::InvalidLogic)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_comment() {
        let inp = "# this is a comment\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn basic_comment_not() {
        let inp = "#this is not a comment";
        dbg!(parse_org(inp));
    }
}
