use std::cell::RefCell;

use crate::{
    constants::{COLON, NEWLINE, TAB},
    node_pool::{NodeID, NodePool},
    types::{MatchError, Node, ParseOpts, Parseable, Result},
    utils::{fn_until, word},
};

#[derive(Debug, Clone, Copy)]
pub struct Keyword<'a> {
    key: &'a str,
    val: &'a str,
}

impl<'a, 'b> Parseable<'a, 'b> for Keyword<'a> {
    fn parse(
        pool: &'b mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let cookie = word(byte_arr, index, "#+")?;

        let key_word = fn_until(byte_arr, cookie.end, |chr: u8| {
            chr == b':' || chr.is_ascii_whitespace()
        })?;

        match byte_arr[key_word.end] {
            COLON => {
                let val = fn_until(byte_arr, key_word.end + 1, |chr: u8| chr == b'\n')?;
            // TODO: use an fn_until_inclusive to not have to add 1 to the end
            // (we want to eat the ending nl too)
                Ok(pool.alloc(
                    Self {
                        key: key_word.to_str(byte_arr),
                        // not mentioned in the spec, but org-element trims
                        val: val.to_str(byte_arr).trim(),
                    },
                    index,
                    val.end + 1,
                    parent,
                ))
            }
            chr if chr.is_ascii_whitespace() => Err(MatchError::InvalidLogic),
            _ => {
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_keyword() {
        let inp = "#+key:val\n";

        dbg!("haiii");
        dbg!(parse_org(inp));
    }

    #[test]
    fn keyword_longer() {
        let inp = "#+intermittent:src_longerlonger\n ending here \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn keyword_ignore_space() {
        let inp = "#+key:                \t    \t              val\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn keyword_ignore_space_nl() {
        let inp = "#+key:     \nval\n";

        dbg!(parse_org(inp));
    }
}
