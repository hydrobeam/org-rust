use crate::{
    constants::{COLON, NEWLINE, TAB},
    types::{MatchError, Node, ParseOpts, Parseable, Result},
    utils::{bytes_to_str, fn_until, word},
};

#[derive(Debug, Clone, Copy)]
pub struct Keyword<'a> {
    key: &'a str,
    val: &'a str,
}

impl<'a> Parseable<'a> for Keyword<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
        let cookie = word(byte_arr, index, "#+")?;

        let key_word = fn_until(byte_arr, cookie.end, |chr: u8| {
            chr == b':' || chr.is_ascii_whitespace()
        })?;

        match byte_arr[key_word.end] {
            COLON => {
                let val = fn_until(byte_arr, key_word.end + 1, |chr: u8| chr == b'\n')?;
                Ok(Node::make_leaf(
                    Self {
                        key: key_word.obj,
                        // not mentioned in the spec, but org-element trims
                        val: val.obj.trim(),
                    },
                    index,
                    val.end,
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
