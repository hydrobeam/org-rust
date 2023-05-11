use crate::node_pool::{NodeID, NodePool};
use crate::types::{ParseOpts, Parseable, Result};
use crate::utils::fn_until;

#[derive(Debug, Clone, Copy)]
pub struct Comment<'a>(&'a str);

impl<'a> Parseable<'a> for Comment<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        if byte_arr[index + 1].is_ascii_whitespace() {
            let content = fn_until(byte_arr, index + 1, |chr: u8| chr == b'\n')?;
            // TODO: use an fn_until_inclusive to not have to add 1 to the end
            // (we want to eat the ending nl too)
            Ok(pool.alloc(Self(content.to_str(byte_arr)), index, content.end + 1, None))
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
