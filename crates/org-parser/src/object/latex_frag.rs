use crate::constants::{
    BACKSLASH, DOLLAR, LBRACE, LBRACK, LPAREN, NEWLINE, RBRACE, RBRACK, RPAREN,
};
use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};

use super::parse_entity;

macro_rules! double_ending {
    ($parser: ident,
     $cursor: ident,
     $start: tt,
     $parse_opts: ident,
     $parent: ident,
     $byte_1: tt, $byte_2: tt,
     $type: ident
    ) => {
        loop {
            match $cursor.try_curr()? {
                NEWLINE => {
                    // the error we return doesn't matter, as long as we error
                    $parse_opts.from_object = false;
                    $parse_opts.list_line = false;
                    if let Err(MatchError::InvalidLogic) =
                        parse_element($parser, $cursor.adv_copy(1), $parent, $parse_opts)
                    {
                        $cursor.next();
                    } else {
                        // just blow up REVIEW: find out if it's okay to return InvalidLogic here
                        return Err(MatchError::EofError);
                    }
                }
                $byte_1 => {
                    if $cursor.peek(1)? == $byte_2 {
                        return Ok($parser.alloc(
                            Self::$type($cursor.clamp_backwards($start + 2)),
                            $start,
                            $cursor.index + 2,
                            $parent,
                        ));
                    } else {
                        $cursor.next();
                    }
                }
                _ => $cursor.next(),
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        parse_opts.from_paragraph = true;
        // figure out which fragment we have
        if cursor.curr() == DOLLAR {
            if cursor.peek(1)? == DOLLAR {
                cursor.index += 2;
                double_ending!(parser, cursor, start, parse_opts, parent, DOLLAR, DOLLAR, Display)
            } else if cursor.peek(2)? == DOLLAR && verify_single_char_latex_frag(cursor) {
                return Ok(parser.alloc(
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
                            parse_opts.from_object = false;
                            parse_opts.list_line = false;
                            if let Err(MatchError::InvalidLogic) =
                                parse_element(parser, cursor.adv_copy(1), parent, parse_opts)
                            {
                                cursor.next();
                            } else {
                                // just blow up REVIEW: find out if it's okay to return InvalidLogic here
                                return Err(MatchError::EofError);
                            }
                        }
                        DOLLAR => {
                            if verify_latex_frag(cursor, true) {
                                return Ok(parser.alloc(
                                    Self::Inline(cursor.clamp_backwards(start + 1)),
                                    start,
                                    cursor.index + 1,
                                    parent,
                                ));
                            } else {
                                cursor.next();
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
                        parser, cursor, start, parse_opts, parent, BACKSLASH, RPAREN, Inline
                    )
                }
                LBRACK => {
                    cursor.next();
                    double_ending!(
                        parser, cursor, start, parse_opts, parent, BACKSLASH, RBRACK, Display
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
                        return Ok(parser.alloc(entity, start, end_name_ind, parent));
                    }

                    match cursor.try_curr()? {
                        LBRACE => {
                            cursor.next();
                            loop {
                                match cursor.try_curr()? {
                                    NEWLINE | LBRACE => {
                                        return Err(MatchError::InvalidLogic);
                                    }
                                    RBRACE => {
                                        return Ok(parser.alloc(
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
                                        return Ok(parser.alloc(
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
                            return Ok(parser.alloc(
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

fn verify_latex_frag(cursor: Cursor, post: bool) -> bool {
    let before_maybe = cursor.peek_rev(1);
    let after_maybe = cursor.peek(1);

    if post {
        let before_val = before_maybe.unwrap();
        // if we're in post, then a character before the markup Must Exist
        (!before_val.is_ascii_whitespace() && !matches!(before_val, b'.' | b',' | b'$'))
            && if let Ok(after) = after_maybe {
                after.is_ascii_punctuation() || after.is_ascii_whitespace()
            } else {
                // no after => valid
                true
            }
    } else if let Ok(after) = after_maybe {
        !after.is_ascii_whitespace()
            && !matches!(after, b'.' | b',' | b';' | b'$')
            && if let Ok(val) = before_maybe {
                val != DOLLAR
            } else {
                // bof is valid
                true
            }
    } else {
        // if there's no after, cannot be valid markup
        false
    }
}

fn verify_single_char_latex_frag(cursor: Cursor) -> bool {
    // distances:
    // 10123
    // p$i$c
    //
    // we are at the dollar

    // handle access this way in case of underflow
    let pre = cursor.peek_rev(1);
    // pretty much never going to overflow
    let post = cursor.peek(3);

    let Ok(inner) = cursor.peek(1) else {
        return false;
    };

    !(inner.is_ascii_whitespace() || matches!(inner, b'.' | b',' | b'?' | b';' | b'"'))
        // both could be dne
        && if let Ok(after) = post {
            after.is_ascii_punctuation() || after.is_ascii_whitespace()
        } else {
            true
        }
        && if let Ok(before) = pre {
            before != DOLLAR
        } else {
            true
        }
}

#[cfg(test)]
mod tests {
    use crate::{expr_in_pool, object::LatexFragment, parse_org, types::Expr};
    use pretty_assertions::assert_eq;

    #[test]
    fn basic_latex_frag() {
        let input = r"\(abc\)";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(l, &LatexFragment::Inline("abc"))
    }

    #[test]
    fn latex_frag_display() {
        let input = r"\[abc\]";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(l, &LatexFragment::Display("abc"))
    }

    #[test]
    fn latex_frag_display_dollars() {
        let input = r"$$abc$$";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(l, &LatexFragment::Display("abc"))
    }

    #[test]
    fn latex_frag_inline_dollar() {
        let input = r"$abc$";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(l, &LatexFragment::Inline("abc"))
    }

    #[test]
    fn latex_frag_char_inline_dollar() {
        let input = r"$c$";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(l, &LatexFragment::Inline("c"))
    }

    #[test]
    fn latex_frag_char_inline_dollar_invalid() {
        let input = r"$,$";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment);
        assert!(l.is_none())

        // not this
        // assert_eq!(l, &LatexFragment::Inline(","))
    }

    #[test]
    fn latex_frag_command_1() {
        let input = r"\command{swag}";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(
            l,
            &LatexFragment::Command {
                name: "command",
                contents: Some("swag"),
            }
        )
    }
    #[test]
    fn latex_frag_command_2() {
        let input = r"\command[swag]";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(
            l,
            &LatexFragment::Command {
                name: "command",
                contents: Some("swag"),
            }
        )
    }

    #[test]
    fn latex_frag_command_3() {
        let input = r"\command no command!";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(
            l,
            &LatexFragment::Command {
                name: "command",
                contents: None,
            }
        )
    }

    #[test]
    fn latex_frag_command_4() {
        // one backslash + invalid char => not a command!
        let input = r"\) not a command";
        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn latex_frag_newline() {
        let input = r"$ab

c$";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment);
        assert!(l.is_none())

        // assert_eq!(l, &LatexFragment::Inline("ab\n\nc"))
    }

    #[test]
    fn latex_frag_newline_2() {
        let input = r"\(ab

c$\)";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment);
        assert!(l.is_none());

        // assert_eq!(l, &LatexFragment::Inline("ab\n\nc"))
    }

    #[test]
    fn latex_frag_newline_3() {
        let input = r"\(ab
c
con
t
ent
$\)";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, LatexFragment).unwrap();

        assert_eq!(l, &LatexFragment::Inline("ab\nc\ncon\nt\nent\n$"))
    }

    #[test]
    fn latex_frag_all() {
        let input = r"
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
        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn latex_frag_pretext() {
        let input = "one two $three\nfourfive";

        let pool = parse_org(input);

        dbg!(&pool);
        pool.print_tree();
    }

    #[test]
    fn single_backslash_char_eof() {
        let input = r"   \s";
        let pool = parse_org(input);
        let item = expr_in_pool!(pool, Plain).unwrap();
        assert_eq!(item, &r"\s");
    }
}
