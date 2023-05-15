use crate::constants::{
    BACKSLASH, DOLLAR, LBRACE, LBRACK, LPAREN, NEWLINE, RBRACE, RBRACK, RPAREN, SLASH,
};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until};

// #[derive(Debug, Clone)]
// pub struct LatexFragment<'a> {
//     kind: FragKind<'a>,
//     contents: &'a str,
// }

#[derive(Debug, Clone)]
pub enum LatexFragment<'a> {
    Command(Option<&'a str>),
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
            curr_ind += 1;
            if *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? == DOLLAR {
                loop {
                    match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                        NEWLINE => {
                            match parse_element(pool, byte_arr, curr_ind + 1, parent, parse_opts) {
                                Ok(_) | Err(MatchError::EofError) => {
                                    return Err(MatchError::EofError)
                                }
                                Err(MatchError::InvalidLogic) => {}
                            }
                        }
                        DOLLAR => {
                            todo!()
                        }
                        _ => {}
                    }
                    curr_ind += 1;
                }
            } else {
                loop {
                    match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                        NEWLINE => {
                            match parse_element(pool, byte_arr, curr_ind + 1, parent, parse_opts) {
                                Ok(_) | Err(MatchError::EofError) => {
                                    return Err(MatchError::EofError)
                                }
                                Err(MatchError::InvalidLogic) => {}
                            }
                        }
                        DOLLAR => {
                            todo!()
                        }
                        _ => {}
                    }
                    curr_ind += 1;
                }
            }
        } else if byte_arr[curr_ind] == BACKSLASH {
            curr_ind += 1;
            match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                LPAREN => loop {
                    match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                        NEWLINE => {
                            if let Ok(_) | Err(MatchError::EofError) =
                                parse_element(pool, byte_arr, curr_ind + 1, parent, parse_opts)
                            {
                                return Err(MatchError::EofError);
                            }
                        }
                        SLASH => {
                            if *byte_arr.get(curr_ind + 1).ok_or(MatchError::EofError)? == RPAREN {
                                return Ok(pool.alloc(
                                    Self::Inline(bytes_to_str(&byte_arr[index + 2..curr_ind])),
                                    index,
                                    curr_ind + 2,
                                    parent,
                                ));
                            }
                        }
                        _ => {}
                    }
                    curr_ind += 1;
                },
                LBRACK => loop {
                    match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                        NEWLINE => {
                            parse_opts.from_paragraph = true;

                            if let Ok(_) | Err(MatchError::EofError) =
                                parse_element(pool, byte_arr, curr_ind + 1, parent, parse_opts)
                            {
                                return Err(MatchError::EofError);
                            }
                        }
                        SLASH => {
                            if *byte_arr.get(curr_ind + 1).ok_or(MatchError::EofError)? == RBRACK {
                                return Ok(pool.alloc(
                                    Self::Display(bytes_to_str(&byte_arr[index + 2..curr_ind])),
                                    index,
                                    curr_ind + 2,
                                    parent,
                                ));
                            }
                        }
                        _ => {}
                    }
                    curr_ind += 1;
                },
                chr if !chr.is_ascii_whitespace() => {
                    let name_match = fn_until(byte_arr, curr_ind, |chr| {
                        !chr.is_ascii_alphabetic()
                            || chr.is_ascii_whitespace()
                            || chr == LBRACE
                            || chr == LBRACK
                    })?;

                    match byte_arr[name_match.end] {
                        LBRACE => loop {
                            match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                                NEWLINE | LBRACE => {
                                    todo!()
                                }
                                RBRACE => {
                                    todo!()
                                }
                                _ => {}
                            }
                            curr_ind += 1;
                        },
                        LBRACK => loop {
                            match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                                NEWLINE | LBRACE | LBRACK | RBRACE => {
                                    todo!()
                                }
                                RBRACK => {
                                    todo!()
                                }
                                _ => {}
                            }
                            curr_ind += 1;
                        },
                        _ => {todo!()}
                    }
                    // if byte_arr[name_match.end] == LBRACE {
                    // } else if byte_arr[name_match.end] == LBRACK {
                    // } else {
                    //     return Err(MatchError::InvalidLogic);
                    // }
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

macro_rules! latex_style {
    () => {

    };
}
