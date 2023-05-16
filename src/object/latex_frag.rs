use crate::constants::{
    BACKSLASH, DOLLAR, LBRACE, LBRACK, LPAREN, NEWLINE, RBRACE, RBRACK, RPAREN,
};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, verify_latex_frag, verify_single_char_latex_frag};

macro_rules! double_ending {
    ($pool: ident,
     $byte_arr: ident,
     $index: ident,
     $curr_ind: tt,
     $parse_opts: ident,
     $parent: ident,
     $byte_1: tt, $byte_2: tt,
     $type: ident
    ) => {
        loop {
            match *$byte_arr.get($curr_ind).ok_or(MatchError::EofError)? {
                NEWLINE => {
                    $parse_opts.from_paragraph = true;

                    if let Ok(_) | Err(MatchError::EofError) =
                        parse_element($pool, $byte_arr, $curr_ind + 1, $parent, $parse_opts)
                    {
                        return Err(MatchError::EofError);
                    }
                }
                $byte_1 => {
                    if *$byte_arr.get($curr_ind + 1).ok_or(MatchError::EofError)? == $byte_2 {
                        return Ok($pool.alloc(
                            Self::$type(bytes_to_str(&$byte_arr[$index + 2..$curr_ind])),
                            $index,
                            $curr_ind + 2,
                            $parent,
                        ));
                    }
                }
                _ => {}
            }
            $curr_ind += 1;
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub enum LatexFragment<'a> {
    Command {
        name: &'a str,
        contents: Option<&'a str>,
    },
    Display(&'a str),
    Inline(&'a str),
}

impl<'a> Parseable<'a> for LatexFragment<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        parse_opts.from_paragraph = true;
        let mut curr_ind = index;
        // figure out which fragment we have
        if byte_arr[curr_ind] == DOLLAR {
            if *byte_arr.get(curr_ind + 1).ok_or(MatchError::EofError)? == DOLLAR {
                curr_ind += 2;
                double_ending!(
                    pool, byte_arr, index, curr_ind, parse_opts, parent, DOLLAR, DOLLAR, Display
                )
            } else if *byte_arr.get(curr_ind + 2).ok_or(MatchError::EofError)? == DOLLAR
                && verify_single_char_latex_frag(byte_arr, curr_ind)
            {
                return Ok(pool.alloc(
                    Self::Inline(bytes_to_str(&byte_arr[(curr_ind + 1)..(curr_ind + 2)])),
                    index,
                    curr_ind + 3,
                    parent,
                ));
            } else if verify_latex_frag(byte_arr, curr_ind, false) {
                curr_ind += 1;
                loop {
                    match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                        NEWLINE => {
                            if let Ok(_) | Err(MatchError::EofError) = parse_element(
                                pool,
                                byte_arr,
                                curr_ind + 1, // skip the newline
                                parent,
                                parse_opts,
                            ) {
                                return Err(MatchError::EofError);
                            }
                        }
                        DOLLAR => {
                            if verify_latex_frag(byte_arr, curr_ind, true) {
                                return Ok(pool.alloc(
                                    Self::Inline(bytes_to_str(&byte_arr[index + 1..curr_ind])),
                                    index,
                                    curr_ind + 1,
                                    parent,
                                ));
                            }
                        }
                        _ => {}
                    }
                    curr_ind += 1;
                }
            } else {
                return Err(MatchError::InvalidLogic);
            }
        } else if byte_arr[curr_ind] == BACKSLASH {
            curr_ind += 1;
            match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                LPAREN => {
                    curr_ind += 1;
                    double_ending!(
                        pool, byte_arr, index, curr_ind, parse_opts, parent, BACKSLASH, RPAREN,
                        Inline
                    )
                }

                LBRACK => {
                    curr_ind += 1;
                    double_ending!(
                        pool, byte_arr, index, curr_ind, parse_opts, parent, BACKSLASH, RBRACK,
                        Display
                    )
                }
                chr if chr.is_ascii_alphabetic() => {
                    let name_match = fn_until(byte_arr, curr_ind, |chr| {
                        !chr.is_ascii_alphabetic()
                            || chr.is_ascii_whitespace()
                            || chr == LBRACE
                            || chr == LBRACK
                    });

                    let prev_name_ind = curr_ind;
                    curr_ind = if let Ok(name) = name_match {
                        name.end
                    } else {
                        byte_arr.len()
                    };
                    let end_name_ind = curr_ind;
                    // TODO check if the name is an entity first.

                    // dbg!(bytes_to_str(&byte_arr[prev_name_ind..end_name_ind],));
                    match byte_arr[curr_ind] {
                        LBRACE => {
                            curr_ind += 1;
                            loop {
                                match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                                    NEWLINE | LBRACE => {
                                        return Err(MatchError::InvalidLogic);
                                    }
                                    RBRACE => {
                                        return Ok(pool.alloc(
                                            Self::Command {
                                                name: bytes_to_str(
                                                    &byte_arr[prev_name_ind..end_name_ind],
                                                ),
                                                contents: Some(bytes_to_str(
                                                    &byte_arr[(end_name_ind + 1)..curr_ind],
                                                )),
                                            },
                                            index,
                                            curr_ind + 1,
                                            parent,
                                        ))
                                    }
                                    _ => {}
                                }
                                curr_ind += 1;
                            }
                        }
                        LBRACK => {
                            curr_ind += 1;
                            loop {
                                match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                                    NEWLINE | LBRACE | LBRACK | RBRACE => {
                                        return Err(MatchError::InvalidLogic);
                                    }
                                    RBRACK => {
                                        return Ok(pool.alloc(
                                            Self::Command {
                                                name: bytes_to_str(
                                                    &byte_arr[prev_name_ind..end_name_ind],
                                                ),
                                                contents: Some(bytes_to_str(
                                                    &byte_arr[(end_name_ind + 1)..curr_ind],
                                                )),
                                            },
                                            index,
                                            curr_ind + 1,
                                            parent,
                                        ))
                                    }
                                    _ => {}
                                }
                                curr_ind += 1;
                            }
                        }
                        _ => {
                            return Ok(pool.alloc(
                                Self::Command {
                                    name: bytes_to_str(&byte_arr[prev_name_ind..end_name_ind]),
                                    contents: None,
                                },
                                index,
                                curr_ind,
                                parent,
                            ))
                        }
                    }
                }
                _ => {
                    return Err(MatchError::InvalidLogic);
                }
            }
        } else {
            return Err(MatchError::InvalidLogic);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_latex_frag() {
        let inp = r"\(abc\)";

        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_display() {
        let inp = r"\[abc\]";

        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_display_dollars() {
        let inp = r"$$abc$$";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_inline_dollar() {
        let inp = r"$abc$";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_char_inline_dollar() {
        let inp = r"$c$";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_char_inline_dollar_invalid() {
        let inp = r"$,$";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_command_1() {
        let inp = r"\command{swag}";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }
    #[test]
    fn latex_frag_command_2() {
        let inp = r"\command[swag]";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_command_3() {
        let inp = r"\command no command!";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_command_4() {
        // one backslash + invalid char => not a command!
        let inp = r"\) not a command";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_newline() {
        let inp = r"$ab

c$";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_newline_2() {
        let inp = r"\(ab

c$\)";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_newline_3() {
        let inp = r"\(ab
c
con
t
ent
$\)";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn latex_frag_all() {
        let inp = r"
$\alpha$ $$do
llar$$
\[display
 block\] \(consecutive gaming\)

\command

\comma
and

\command{ab
c}

";
        let pool = parse_org(inp);

        pool.root().print_tree(&pool);
    }
}
