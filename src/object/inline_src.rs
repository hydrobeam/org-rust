use std::cell::RefCell;

use crate::constants::*;
use crate::node_pool::{NodeID, NodePool};
use crate::types::{MatchError, Node, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, word, Match};

#[derive(Debug, Clone, Copy)]
pub struct InlineSrc<'a> {
    pub lang: &'a str,
    pub headers: Option<&'a str>,
    pub body: &'a str,
}

impl<'a, 'b> Parseable<'a, 'b> for InlineSrc<'a> {
    fn parse(
        pool: &'b mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        // TODO: cache this
        // REVIEW: maybe not :3
        let src_word = word(byte_arr, index, "src_")?;

        let lang = fn_until(byte_arr, src_word.end, |chr: u8| {
            !(chr == b'[' || chr == b'{' || chr.is_ascii_whitespace())
        })?;

        match byte_arr[lang.end] {
            LBRACE => {
                let body = Self::parse_body(byte_arr, index)?;
                Ok(pool.alloc(
                    Self {
                        lang: body.to_str(byte_arr),
                        headers: None,
                        body: body.to_str(byte_arr),
                    },
                    index,
                    body.end,
                    None,
                ))
            }
            LBRACK => {
                let header = Self::parse_header(byte_arr, lang.end)?;
                if byte_arr[header.end] != LBRACE {
                    let body = Self::parse_body(byte_arr, index)?;
                    Ok(pool.alloc(
                        Self {
                            lang: lang.to_str(byte_arr),
                            headers: Some(header.to_str(byte_arr)),
                            body: body.to_str(byte_arr),
                        },
                        index,
                        body.end,
                        None,
                    ))
                } else {
                    Err(MatchError::InvalidLogic)
                }
            }
            // We are whitespace here, which means there was whitespace after the src_
            // so blow up
            _ => return Err(MatchError::InvalidLogic),
        }
    }
}

impl<'a> InlineSrc<'a> {
    // the logic is exactly the same, except for the perimeters
    fn parse_header(byte_arr: &'a [u8], index: usize) -> Result<Match> {
        InlineSrc::parse_src(byte_arr, index, LBRACK, RBRACK)
    }
    fn parse_body(byte_arr: &'a [u8], index: usize) -> Result<Match> {
        InlineSrc::parse_src(byte_arr, index, LBRACE, RBRACE)
    }
    #[inline(always)]
    fn parse_src(byte_arr: &'a [u8], index: usize, lperim: u8, rperim: u8) -> Result<Match> {
        // Brackets have to be balanced
        // -1 for left bracket
        // +1 for right bracket
        let mut bracket_count: i32 = 0;

        let mut j: usize = index;

        loop {
            match byte_arr[j] {
                chr if chr == lperim => {
                    bracket_count -= 1;
                }
                chr if chr == rperim => {
                    bracket_count += 1;
                    if bracket_count == 0 {
                        let start = index;
                        let end = j + 1;
                        return Ok(Match { start, end });
                    }
                }
                NEWLINE => {
                    return Err(MatchError::InvalidLogic);
                }
                _ => {}
            } // end of match

            j += 1;
        } // end of loop
    }
}
