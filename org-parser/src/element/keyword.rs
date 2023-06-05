use crate::constants::COLON;
use crate::node_pool::NodeID;
use crate::object::MacroDef;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::bytes_to_str;

#[derive(Debug, Clone)]
pub enum Keyword<'a> {
    Basic { key: &'a str, val: &'a str },
    Macro(MacroDef<'a>),
    Affilliated(&'a str), // TODO
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
        cursor.index = key_word.end;

        match key_word.obj {
            "macro" | "MACRO" => {
                if cursor.curr() == COLON {
                    cursor.next();
                    if let Ok(mac) = MacroDef::parse(cursor) {
                        let nam = mac.obj.name;
                        let id = parser
                            .pool
                            .alloc(Keyword::Macro(mac.obj), start, mac.end, parent);
                        parser.macros.insert(nam, id);
                        return Ok(id);
                    }

                    // if macro fails, interpret it as a regular ol keyword
                }
            }
            _ => {}
        }

        match cursor.curr() {
            COLON => {
                cursor.next();
                let val = cursor.fn_until(|chr: u8| chr == b'\n')?;
                // TODO: use an fn_until_inclusive to not have to add 1 to the end
                // (we want to eat the ending nl too)
                parser.keywords.insert(key_word.obj, val.obj.trim());
                Ok(parser.alloc(
                    Keyword::Basic {
                        key: key_word.obj,
                        // not mentioned in the spec, but org-element trims
                        val: val.obj.trim(),
                    },
                    start,
                    val.end + 1,
                    parent,
                ))
            }
            _ => Err(MatchError::InvalidLogic),
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
