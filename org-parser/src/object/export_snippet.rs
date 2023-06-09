use crate::constants::{COLON, HYPHEN, NEWLINE};
use crate::node_pool::NodeID;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSnippet<'a> {
    pub backend: &'a str,
    pub contents: &'a str,
}

impl<'a> Parseable<'a> for ExportSnippet<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.word("@@")?;
        let backend_match = cursor
            .fn_until(|chr| chr == COLON || !(chr.is_ascii_alphanumeric() || chr == HYPHEN))?;

        cursor.index = backend_match.end;

        cursor.word(":")?;
        let start_contents = cursor.index;
        loop {
            let pot_match = cursor.fn_until(|chr| chr == b'@' || chr == NEWLINE)?;
            cursor.index = pot_match.end;
            match cursor.curr() {
                b'@' => {
                    if cursor.peek(1)? == b'@' {
                        return Ok(parser.alloc(
                            Self {
                                backend: backend_match.obj,
                                contents: cursor.clamp_backwards(start_contents),
                            },
                            start,
                            cursor.index + 2,
                            parent,
                        ));
                    } else {
                        cursor.next();
                    }
                }
                _ => return Err(MatchError::InvalidLogic),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse_org, types::Expr};

    use super::*;

    #[test]
    fn basic_export() {
        let input = r"
@@:@@
";

        let pool = parse_org(input);
        let head = pool.pool.iter().find_map(|x| {
            if let Expr::ExportSnippet(snip) = &x.obj {
                Some(snip)
            } else {
                None
            }
        });
        assert_eq!(
            head.unwrap(),
            &ExportSnippet {
                backend: "",
                contents: ""
            }
        );

    }

    #[test]
    fn cool_export() {
        let input = r"
@@html:valuesss@@
";
        let pool = parse_org(input);
        let head = pool.pool.iter().find_map(|x| {
            if let Expr::ExportSnippet(snip) = &x.obj {
                Some(snip)
            } else {
                None
            }
        });
        assert_eq!(
            head.unwrap(),
            &ExportSnippet {
                backend: "html",
                contents: "valuesss"
            }
        );
    }

    #[test]
    fn newline_export_snippet() {
        let input = r"
@@html:value
sss@@
";
        let pool = parse_org(input);
        let head = pool.pool.iter().find_map(|x| {
            if let Expr::ExportSnippet(snip) = &x.obj {
                Some(snip)
            } else {
                None
            }
        });
        assert!(head.is_none());
    }

    #[test]
    fn at_export_snippet() {
        let input = r"
@@html:va@lue sss@@
";
        let pool = parse_org(input);
        let head = pool.pool.iter().find_map(|x| {
            if let Expr::ExportSnippet(snip) = &x.obj {
                Some(snip)
            } else {
                None
            }
        });
        assert_eq!(
            head.unwrap(),
            &ExportSnippet {
                backend: "html",
                contents: "va@lue sss"
            }
        );
    }
}
