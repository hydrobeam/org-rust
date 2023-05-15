use crate::constants::{
    BACKSLASH, DOLLAR, LBRACE, LBRACK, LPAREN, NEWLINE, RBRACE, RBRACK, RPAREN, SLASH,
};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, verify_latex_frag, verify_single_char_latex_frag};

macro_rules! double_ending {
    ($pool: ident,
     $byte_arr: ident,
     $index: ident,
     $curr_ind: ident,
     $parse_opts: ident,
     $parent: ident,
     $byte_1: tt, $byte_2: tt) => {
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
                            Self::Display(bytes_to_str(&$byte_arr[$index + 2..$curr_ind])),
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

#[derive(Debug, Clone)]
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
            curr_ind += 1;
            if *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? == DOLLAR {
                curr_ind += 1;
                double_ending!(pool, byte_arr, index, curr_ind, parse_opts, parent, DOLLAR, DOLLAR)
            } else {
                // i is curr_ind
                // 21012
                // p$i$c
                let single_andy: bool;
                if *byte_arr.get(curr_ind + 1).ok_or(MatchError::EofError)? == DOLLAR {
                    let pre = byte_arr.get(curr_ind - 2);
                    let post = byte_arr.get(curr_ind + 2);
                    if verify_single_char_latex_frag(
                        pre.copied(),
                        byte_arr[curr_ind],
                        post.copied(),
                    ) {
                        return Ok(pool.alloc(
                            Self::Inline(bytes_to_str(&byte_arr[curr_ind..curr_ind + 1])),
                            index,
                            curr_ind + 2,
                            parent,
                        ));
                    } else {
                        return Err(MatchError::InvalidLogic);
                    }
                } else {
                    if !verify_latex_frag(byte_arr, curr_ind, false) {
                        // REVIEW: how important is it to return the right error here
                        return Err(MatchError::InvalidLogic);
                    }
                    loop {
                        match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                            NEWLINE => {
                                match parse_element(
                                    pool,
                                    byte_arr,
                                    curr_ind + 1, // skip the newline
                                    parent,
                                    parse_opts,
                                ) {
                                    Ok(_) | Err(MatchError::EofError) => {
                                        return Err(MatchError::EofError)
                                    }
                                    Err(MatchError::InvalidLogic) => {}
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
                }
            }
        } else if byte_arr[curr_ind] == BACKSLASH {
            curr_ind += 1;
            match *byte_arr.get(curr_ind).ok_or(MatchError::EofError)? {
                LPAREN => double_ending!(
                    pool, byte_arr, index, curr_ind, parse_opts, parent, SLASH, RPAREN
                ),
                LBRACK => double_ending!(
                    pool, byte_arr, index, curr_ind, parse_opts, parent, SLASH, RBRACK
                ),
                chr if !chr.is_ascii_whitespace() => {
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

                    match byte_arr[curr_ind] {
                        LBRACE => loop {
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
                                                &byte_arr[end_name_ind..curr_ind],
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
                        },
                        LBRACK => loop {
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
                                                &byte_arr[end_name_ind..curr_ind],
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
                        },
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
