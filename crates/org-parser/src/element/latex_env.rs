use crate::constants::{NEWLINE, RBRACE, STAR};
use crate::node_pool::NodeID;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use regex::bytes::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatexEnv<'a> {
    pub name: &'a str,
    pub contents: &'a str,
}

impl<'a> Parseable<'a> for LatexEnv<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.word(r"\begin{")?;
        let name_match = cursor.fn_until(|chr| {
            !chr.is_ascii_alphanumeric() && chr != STAR || (chr == NEWLINE || chr == RBRACE)
        })?;

        cursor.index = name_match.end;
        cursor.word("}\n")?;
        let name = name_match.obj;

        let a = format!(r"(?m)^[ \t]*\\end{{{name}}}[\t ]*$");
        // HACK/FIXME: i simply cannot figure out how to properly escape the curls in the regex.
        // always getting hit with:
        // error: repetition quantifier expects a valid decimal
        let a = a.replace("{", r"\{");
        let a = a.replace("}", r"\}");

        let ending_re: Regex = Regex::new(&a).unwrap();
        let matched_reg = ending_re
            .find_at(cursor.byte_arr, cursor.index)
            .ok_or(MatchError::InvalidLogic)?;

        Ok(parser.alloc(
            Self {
                name,
                contents: cursor.clamp_forwards(matched_reg.start()),
            },
            start,
            matched_reg.end(),
            parent,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{element::LatexEnv, expr_in_pool, parse_org, types::Expr};

    #[test]
    fn basic_latex_env() {
        let inp = r"
\begin{align*}
\end{align*}
";

        dbg!(parse_org(inp));
    }

    #[test]
    fn latex_env_with_content() {
        let inp = r"
\begin{align*}

\text{latex constructs}\\
\alpha\\
\beta\\

10x + 4 &= 3\\

\end{align*}
";

        dbg!(parse_org(inp));
    }

    #[test]
    fn latex_env_failed_header() {
        // not alpha numeric
        let inp = r"
\begin{star!}
\end{star!}
";

        dbg!(parse_org(inp));

        let inp = r"
\begin{a13214-}
\end{a13214-}
";
        dbg!(parse_org(inp));
        // failed construction
        let inp = r"
\begin{one}more stuff
\end{one}
";
        dbg!(parse_org(inp));
    }

    #[test]
    fn latex_empty_start() {
        let inp = r"
\begin{}
\end{}
";
        dbg!(parse_org(inp));
    }

    #[test]
    fn latex_failed_end() {
        let inp = r"
\begin{start}
\end{notstart}
";
        dbg!(parse_org(inp));
    }

    #[test]
    fn latex_env_indented() {
        let input = r"
             \begin{align}
             we are eating so good?
             \end{align}
";

        let parsed = parse_org(input);

        let l = expr_in_pool!(parsed, LatexEnv).unwrap();

        assert_eq!(
            l,
            &LatexEnv {
                name: "align",
                contents: "             we are eating so good?\n"
            }
        )
    }
}
