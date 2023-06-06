use std::borrow::Cow;
use std::fmt;

use crate::constants::{COMMA, DOLLAR, HYPHEN, LPAREN, NEWLINE, RBRACE, RPAREN, UNDERSCORE};
use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::Match;

#[derive(Debug, Clone)]
pub struct MacroDef<'a> {
    // Highest ArgNum
    pub num_args: u32,
    pub input: Vec<ArgNumOrText<'a>>,
    pub name: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum ArgNumOrText<'a> {
    Text(&'a str),
    ArgNum(u32),
}

impl<'a> MacroDef<'a> {
    pub(crate) fn parse(mut cursor: Cursor<'a>) -> Result<Match<Self>> {
        let start = cursor.index;
        // we start just after the colon
        // #+macro: NAME INNER
        // INNER: words $1 is an argument $2 is another
        cursor.skip_ws();
        // A string starting with a alphabetic character followed by any number of
        // alphanumeric characters, hyphens and underscores (-_).
        if !cursor.curr().is_ascii_alphabetic() || cursor.curr() == NEWLINE {
            return Err(MatchError::InvalidLogic);
        }

        let name_match = cursor.fn_while(|chr: u8| {
            chr.is_ascii_alphanumeric() || chr == HYPHEN || chr == UNDERSCORE
        })?;
        cursor.index = name_match.end;

        cursor.skip_ws();
        // macro with no body?
        if cursor.curr() == NEWLINE {
            return Err(MatchError::InvalidLogic);
        }

        // let inner_match = cursor.fn_until(|chr: u8| chr.is_ascii_whitespace())?;
        let mut prev_ind = cursor.index;
        let mut ret_vec: Vec<ArgNumOrText> = Vec::new();
        let mut num_args = 0;
        loop {
            match cursor.curr() {
                DOLLAR => {
                    if cursor.peek(1)?.is_ascii_digit() {
                        ret_vec.push(ArgNumOrText::Text(cursor.clamp_backwards(prev_ind)));
                        // TODO: only supports 9 args rn
                        // parse numbers

                        let arg_ident = (cursor.peek(1)? - 48) as u32;
                        num_args = num_args.max(arg_ident);
                        ret_vec.push(ArgNumOrText::ArgNum(arg_ident));
                        // skip past dollar and number
                        cursor.index += 2;
                        prev_ind = cursor.index;
                    } else {
                        cursor.next();
                    }
                }
                NEWLINE => {
                    ret_vec.push(ArgNumOrText::Text(cursor.clamp_backwards(prev_ind)));
                    break;
                }
                _ => {
                    cursor.next();
                }
            }
        }

        Ok(Match {
            start,
            end: cursor.index + 1,
            obj: Self {
                input: ret_vec,
                num_args,
                name: name_match.obj,
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct MacroCall<'a> {
    pub name: &'a str,
    pub args: Vec<&'a str>,
}

impl<'a> Parseable<'a> for MacroCall<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        cursor.word("{{{")?;

        if !cursor.curr().is_ascii_alphabetic() {
            return Err(MatchError::InvalidLogic);
        }
        let name_match = cursor.fn_while(|chr: u8| {
            // permitted characters
            chr.is_ascii_alphanumeric()
                || chr == HYPHEN
                || chr == UNDERSCORE
                // start params
                && (chr != LPAREN
                // macro end
                || chr != RBRACE)
        })?;

        // A string starting with a alphabetic character followed by any number of
        // alphanumeric characters, hyphens and underscores (-_).
        cursor.index = name_match.end;

        match cursor.curr() {
            LPAREN => {
                // used to check if we have {{{name()}}} (emtpy func call)
                cursor.next();
                let mut arg_vec = Vec::new();
                let mut prev_ind = cursor.index;
                loop {
                    match cursor.try_curr()? {
                        NEWLINE => {
                            parse_opts.from_paragraph = true;
                            parse_opts.list_line = false;
                            parse_opts.from_object = false;
                            match parse_element(parser, cursor.adv_copy(1), parent, parse_opts) {
                                Ok(_) => return Err(MatchError::InvalidLogic),
                                Err(MatchError::InvalidLogic) => {}
                                ret @ Err(_) => return ret,
                            }
                        }
                        RBRACE => {
                            if cursor.peek(1)? == RBRACE && cursor.peek(2)? == RBRACE {
                                return Err(MatchError::InvalidLogic);
                            }
                        }
                        RPAREN => {
                            arg_vec.push(cursor.clamp_backwards(prev_ind));
                            cursor.word(")}}}")?;
                            return Ok(parser.alloc(
                                MacroCall {
                                    name: name_match.obj,
                                    args: arg_vec,
                                },
                                start,
                                cursor.index,
                                parent,
                            ));
                        }
                        COMMA => {
                            arg_vec.push(cursor.clamp_backwards(prev_ind));
                            prev_ind = cursor.index + 1;
                        }
                        _ => {}
                    }
                    cursor.next();
                }
            }
            RBRACE => {
                cursor.word("}}}")?;
                return Ok(parser.alloc(
                    MacroCall {
                        name: name_match.obj,
                        args: Vec::new(),
                    },
                    start,
                    cursor.index,
                    parent,
                ));
            }
            _ => return Err(MatchError::InvalidLogic),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    use super::*;

    #[test]
    fn basic_macro() {
        let input = r"{{{abc}}}";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn macro_with_args() {
        let input = r"{{{poem(cool, three)}}}";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn basic_macro_def() {
        let input = r"#+macro: poem hiii $1 $2 text
";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn combined_macros() {
        let input = r"#+macro: poem hiii $1 $2 text

{{{poem(cool, three)}}}
";
        let pool = parse_org(input);
        pool.print_tree();
    }
}
