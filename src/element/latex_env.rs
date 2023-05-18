use memchr::memmem;

use crate::constants::{NEWLINE, RBRACE, STAR};
use crate::node_pool::{NodeID, NodePool};
use crate::types::{MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, word};

#[derive(Debug, Clone, Copy)]
pub struct LatexEnv<'a> {
    name: &'a str,
    contents: &'a str,
}

impl<'a> Parseable<'a> for LatexEnv<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let begin_cookie_end = word(byte_arr, index, r"\begin{")?;
        let name_match = fn_until(byte_arr, begin_cookie_end, |chr| {
            !chr.is_ascii_alphanumeric() && chr != STAR || (chr == NEWLINE || chr == RBRACE)
        })?;

        #[rustfmt::skip]
        let name = if byte_arr[name_match.end] == RBRACE
            && name_match.end != begin_cookie_end // \begin{} case
        {
            name_match.obj
        } else {
            return Err(MatchError::InvalidLogic);
        };

        let mut curr_ind = name_match.end + 1;
        if byte_arr[curr_ind] != NEWLINE {
            return Err(MatchError::InvalidLogic);
        }
        // \end{name}
        let alloc_str = format!("\\end{{{name}}}\n");
        let needle = &alloc_str;

        let mut it = memmem::find_iter(&byte_arr[curr_ind..], needle.as_bytes());
        // returns result at the start of the needle
        let loc;
        let end;
        loop {
            if let Some(potential_loc) = it.next() {
                let mut moving_loc = potential_loc + curr_ind - 1;
                while byte_arr[moving_loc] != NEWLINE {
                    if !byte_arr[moving_loc].is_ascii_whitespace() {
                        continue;
                    }
                    moving_loc -= 1;
                }
                loc = moving_loc;
                end = potential_loc + curr_ind + needle.len();
                break;
            } else {
                Err(MatchError::InvalidLogic)?
            }
        }
        // let loc = it.next().ok_or(MatchError::InvalidLogic)? + (name_match.end + 1);

        Ok(pool.alloc(
            Self {
                name,
                // + 1 to skip newline
                contents: bytes_to_str(&byte_arr[(curr_ind + 1)..loc]),
            },
            index,
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
