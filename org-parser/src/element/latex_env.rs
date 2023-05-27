use memchr::memmem;

use crate::constants::{NEWLINE, RBRACE, STAR};
use crate::node_pool::{NodeID, NodePool};
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Clone, Copy)]
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

        #[rustfmt::skip]
        let name = if cursor[name_match.end] == RBRACE
            && name_match.end != cursor.index // \begin{} case
        {
            name_match.obj
        } else {
            return Err(MatchError::InvalidLogic);
        };

        cursor.move_to(name_match.end + 1);

        if cursor.curr() != NEWLINE {
            return Err(MatchError::InvalidLogic);
        }

        // \end{name}
        let alloc_str = format!("\\end{{{name}}}\n");
        let needle = &alloc_str;

        // skip newline
        cursor.next();

        let mut it = memmem::find_iter(cursor.rest(), needle.as_bytes());
        // returns result at the start of the needle
        let loc;
        let end;
        'l: loop {
            if let Some(potential_loc) = it.next() {
                let mut moving_loc = potential_loc + cursor.index - 1;
                while cursor[moving_loc] != NEWLINE {
                    if !cursor[moving_loc].is_ascii_whitespace() {
                        continue 'l;
                    }
                    moving_loc -= 1;
                }
                loc = moving_loc;
                end = potential_loc + cursor.index + needle.len();
                break;
            } else {
                Err(MatchError::InvalidLogic)?
            }
        }
        // let loc = it.next().ok_or(MatchError::InvalidLogic)? + (name_match.end + 1);

        // handle empty contents
        if cursor.index > loc {
            cursor.index = loc;
        }

        Ok(parser.alloc(
            Self {
                name,
                // + 1 to skip newline
                contents: cursor.clamp_forwards(loc),
            },
            start,
            end,
            parent,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

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

        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }
}
