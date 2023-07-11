use crate::constants::{LBRACE, LBRACK, NEWLINE, RBRACE, RBRACK};
use crate::node_pool::NodeID;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::Match;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineSrc<'a> {
    pub lang: &'a str,
    pub headers: Option<&'a str>,
    pub body: &'a str,
}

impl<'a> Parseable<'a> for InlineSrc<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.word("src_")?;

        let lang =
            cursor.fn_until(|chr: u8| chr == b'[' || chr == b'{' || chr.is_ascii_whitespace())?;

        cursor.index = lang.end;

        match cursor.curr() {
            LBRACE => {
                let body = Self::parse_body(cursor)?;
                Ok(parser.alloc(
                    Self {
                        lang: lang.obj,
                        headers: None,
                        body: body.obj,
                    },
                    start,
                    body.end,
                    None,
                ))
            }
            LBRACK => {
                let header = Self::parse_header(cursor)?;
                cursor.move_to(header.end);
                if cursor.curr() == LBRACE {
                    let body = Self::parse_body(cursor)?;
                    Ok(parser.alloc(
                        Self {
                            lang: lang.obj,
                            headers: Some(header.obj),
                            body: body.obj,
                        },
                        start,
                        body.end,
                        parent,
                    ))
                } else {
                    Err(MatchError::InvalidLogic)
                }
            }
            // We are whitespace here, which means there was whitespace after the src_
            // so blow up
            _ => Err(MatchError::InvalidLogic),
        }
    }
}

impl<'a> InlineSrc<'a> {
    // the logic is exactly the same, except for the perimeters
    fn parse_header(cursor: Cursor) -> Result<Match<&str>> {
        InlineSrc::parse_src(cursor, LBRACK, RBRACK)
    }
    fn parse_body(cursor: Cursor) -> Result<Match<&str>> {
        InlineSrc::parse_src(cursor, LBRACE, RBRACE)
    }
    #[inline(always)]
    fn parse_src(mut cursor: Cursor, lperim: u8, rperim: u8) -> Result<Match<&str>> {
        // Brackets have to be balanced
        // -1 for left bracket
        // +1 for right bracket
        let mut bracket_count: i32 = 0;

        let start = cursor.index;
        loop {
            match cursor.curr() {
                chr if chr == lperim => {
                    bracket_count -= 1;
                }
                chr if chr == rperim => {
                    bracket_count += 1;
                    if bracket_count == 0 {
                        return Ok(Match {
                            start,
                            // +1 to skip past lperim and rperim
                            end: cursor.index + 1,
                            obj: cursor.clamp_backwards(start + 1),
                        });
                    }
                }
                NEWLINE => {
                    return Err(MatchError::InvalidLogic);
                }
                _ => {}
            } // end of match

            cursor.next();
        } // end of loop
    }
}

#[cfg(test)]
mod tests {
    use crate::expr_in_pool;
    use crate::object::InlineSrc;
    use crate::parse_org;
    use crate::types::Expr;
    use pretty_assertions::assert_eq;

    #[test]
    fn basic_src() {
        let input = "src_python{neat}";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, InlineSrc).unwrap();

        assert_eq!(
            l,
            &InlineSrc {
                lang: "python",
                headers: None,
                body: "neat"
            }
        )
    }

    #[test]
    fn inlinesrc_header() {
        let input = "src_python[fun]{rad}";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, InlineSrc).unwrap();

        assert_eq!(
            l,
            &InlineSrc {
                lang: "python",
                headers: Some("fun"),
                body: "rad"
            }
        )
    }
}
