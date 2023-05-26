use crate::constants::{
    BACKSLASH, DOLLAR, LBRACE, LBRACK, LPAREN, NEWLINE, RBRACE, RBRACK, RPAREN,
};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{Cursor, MatchError, NodeCache, ParseOpts, Parseable, Result};
use crate::utils::{verify_latex_frag, verify_single_char_latex_frag};

use super::parse_entity;

macro_rules! double_ending {
    ($pool: ident,
     $cursor: ident,
     $start: tt,
     $parse_opts: ident,
     $parent: ident,
     $byte_1: tt, $byte_2: tt,
     $type: ident,
     $cache: ident,
    ) => {
        loop {
            match $cursor.try_curr()? {
                NEWLINE => {
                    // the error we return doesn't matter, as long as we error
                    if let Err(MatchError::InvalidLogic) =
                        parse_element($pool, $cursor.adv_copy(1), $parent, $parse_opts, $cache)
                    {
                        $cursor.next();
                    } else {
                        // just blow up REVIEW: find out if it's okay to return InvalidLogic here
                        return Err(MatchError::EofError);
                    }
                }
                $byte_1 => {
                    if $cursor.peek(1)? == $byte_2 {
                        return Ok($pool.alloc(
                            Self::$type($cursor.clamp_backwards($start + 2)),
                            $start,
                            $cursor.index + 2,
                            $parent,
                        ));
                    }
                }
                _ => $cursor.next(),
            }
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
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
        cache: &mut NodeCache,
    ) -> Result<NodeID> {
        let start = cursor.index;
        parse_opts.from_paragraph = true;
        // figure out which fragment we have
        if cursor.curr() == DOLLAR {
            if cursor.peek(1)? == DOLLAR {
                cursor.index += 2;
                double_ending!(
                    pool, cursor, start, parse_opts, parent, DOLLAR, DOLLAR, Display, cache,
                )
            } else if cursor.peek(2)? == DOLLAR && verify_single_char_latex_frag(cursor) {
                return Ok(pool.alloc(
                    Self::Inline(cursor.clamp(cursor.index + 1, cursor.index + 2)),
                    start,
                    cursor.index + 3,
                    parent,
                ));
            } else if verify_latex_frag(cursor, false) {
                cursor.next();
                loop {
                    match cursor.try_curr()? {
                        NEWLINE => {
                            // the error we return doesn't matter, as long as we error
                            if let Err(MatchError::InvalidLogic) =
                                parse_element(pool, cursor.adv_copy(1), parent, parse_opts, cache)
                            {
                                cursor.next();
                            } else {
                                // just blow up REVIEW: find out if it's okay to return InvalidLogic here
                                return Err(MatchError::EofError);
                            }
                        }
                        DOLLAR => {
                            if verify_latex_frag(cursor, true) {
                                return Ok(pool.alloc(
                                    Self::Inline(cursor.clamp_backwards(start + 1)),
                                    start,
                                    cursor.index + 1,
                                    parent,
                                ));
                            }
                        }
                        _ => cursor.next(),
                    }
                }
            } else {
                return Err(MatchError::InvalidLogic);
            }
        } else if cursor.curr() == BACKSLASH {
            cursor.next();
            match cursor.try_curr()? {
                LPAREN => {
                    cursor.next();
                    double_ending!(
                        pool, cursor, start, parse_opts, parent, BACKSLASH, RPAREN, Inline, cache,
                    )
                }
                LBRACK => {
                    cursor.next();
                    double_ending!(
                        pool, cursor, start, parse_opts, parent, BACKSLASH, RBRACK, Display, cache,
                    )
                }
                chr if chr.is_ascii_alphabetic() => {
                    let name_match = cursor.fn_until(|chr| {
                        !chr.is_ascii_alphabetic()
                            || chr.is_ascii_whitespace()
                            || chr == LBRACE
                            || chr == LBRACK
                    });

                    let prev_name_ind = cursor.index;
                    cursor.index = if let Ok(name) = name_match {
                        name.end
                    } else {
                        cursor.len()
                    };
                    let end_name_ind = cursor.index;
                    let name = cursor.clamp(prev_name_ind, end_name_ind);

                    // TODO stop doing everything in LatexFrag
                    if let Ok(entity) = parse_entity(name) {
                        return Ok(pool.alloc(entity, start, end_name_ind, parent));
                    }

                    match cursor.curr() {
                        LBRACE => {
                            cursor.next();
                            loop {
                                match cursor.try_curr()? {
                                    NEWLINE | LBRACE => {
                                        return Err(MatchError::InvalidLogic);
                                    }
                                    RBRACE => {
                                        return Ok(pool.alloc(
                                            Self::Command {
                                                name,
                                                contents: Some(
                                                    cursor.clamp_backwards(end_name_ind + 1),
                                                ),
                                            },
                                            start,
                                            cursor.index + 1,
                                            parent,
                                        ));
                                    }
                                    _ => {}
                                }
                                cursor.next();
                            }
                        }
                        LBRACK => {
                            cursor.next();
                            loop {
                                match cursor.try_curr()? {
                                    NEWLINE | LBRACE | LBRACK | RBRACE => {
                                        return Err(MatchError::InvalidLogic);
                                    }
                                    RBRACK => {
                                        return Ok(pool.alloc(
                                            Self::Command {
                                                name,
                                                contents: Some(
                                                    cursor.clamp_backwards(end_name_ind + 1),
                                                ),
                                            },
                                            start,
                                            cursor.index + 1,
                                            parent,
                                        ))
                                    }
                                    _ => {}
                                }
                                cursor.next();
                            }
                        }
                        _ => {
                            return Ok(pool.alloc(
                                Self::Command {
                                    name,
                                    contents: None,
                                },
                                start,
                                cursor.index,
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

    #[test]
    fn latex_frag_pretext() {
        let input = "one two $three\nfourfive";

        let pool = parse_org(input);

        dbg!(&pool);
        pool.root().print_tree(&pool);
    }
}
