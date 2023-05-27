use crate::constants::COLON;
use crate::node_pool::{NodeID, NodePool};
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Clone, Copy)]
pub struct Keyword<'a> {
    pub key: &'a str,
    pub val: &'a str,
}

impl<'a> Parseable<'a> for Keyword<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.word("#+")?;

        let key_word = cursor.fn_until(|chr: u8| chr == b':' || chr.is_ascii_whitespace())?;

        match cursor[key_word.end] {
            COLON => {
                cursor.next();
                let val = cursor.fn_until(|chr: u8| chr == b'\n')?;
                // TODO: use an fn_until_inclusive to not have to add 1 to the end
                // (we want to eat the ending nl too)
                Ok(parser.alloc(
                    Self {
                        key: key_word.obj,
                        // not mentioned in the spec, but org-element trims
                        val: val.obj.trim(),
                    },
                    start,
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
