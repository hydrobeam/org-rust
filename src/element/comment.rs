use crate::{
    constants::COLON,
    types::{Node, ParseOpts, Parseable, Result},
    utils::{fn_until, word},
};

#[derive(Debug, Clone, Copy)]
pub struct Comment<'a>(&'a str);

impl<'a> Parseable<'a> for Comment<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        if byte_arr[index + 1].is_ascii_whitespace() {
            let content = fn_until(byte_arr, index + 1, |chr: u8| chr == b'\n')?;
            // TODO: use an fn_until_inclusive to not have to add 1 to the end
            // (we want to eat the ending nl too)
            Ok(Node::make_leaf(Self(content.obj), index, content.end + 1))
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
