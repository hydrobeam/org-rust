// use std::cell::RefCell;
// use std::rc::Rc;
// use Node::*;

// use crate::constants::*;
// use crate::types::{Match, MatchError, Node, ParseOpts, Parseable, Result};
// use crate::utils::{bytes_to_str, fn_until, word};

// #[derive(Debug)]
// pub struct InlineSrc<'a> {
//     lang: &'a str,
//     headers: Option<&'a str>,
//     body: &'a str,
// }

// // pub fn create_node(node: Match<Node>) -> Match<Node> {
// //     return Match {
// //         obj: "meow",
// //         start: 1,
// //         end:  2
// //     }
// // }


// impl<'a> Parseable<'a> for InlineSrc<'a> {
//     fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
//         // TODO: cache this
//         let src_word = word(byte_arr, index, "src_")?;

//         let lang = fn_until(byte_arr, src_word.end, |chr: u8| {
//             !(chr == b'[' || chr == b'{' || chr.is_ascii_whitespace())
//         })?;

//         match byte_arr[lang.end] {
//             LBRACE => {
//                 let body = Self::parse_body(byte_arr, index)?;
//                 Ok(ParseNode::new(Match {
//                     obj: Node::InlineSrc(Self {
//                         lang: lang.obj,
//                         headers: None,
//                         body: body.obj,
//                     }),
//                     start: index,
//                     end: body.end,
//                 }))

//                 // Ok(ParseNode::new(Match {
//                 //     obj: InlineSrc {
//                 //         lang: "meow",
//                 //         headers: None,
//                 //         body: "cu,",
//                 //     },
//                 //     start: 1,
//                 //     end: 5,
//                 // }))
//             }
//             LBRACK => {
//                 let header = Self::parse_header(byte_arr, lang.end)?;
//                 if byte_arr[header.end] != LBRACE {
//                     Err(MatchError)
//                 } else {
//                     let body = Self::parse_body(byte_arr, index)?;
//                     Ok(ParseNode::new(Match {
//                         obj: Node::InlineSrc(Self {
//                             lang: lang.obj,
//                             headers: Some(header.obj),
//                             body: body.obj,
//                         }),
//                         start: index,
//                         end: body.end,
//                     }))
//                 }
//             }
//             _ => return Err(MatchError),
//         }
//     }
// }

// impl<'a> InlineSrc<'a> {
//     // the logic is exactly the same, except for the perimeters
//     fn parse_header(byte_arr: &'a [u8], index: usize) -> Result<Match<&'a str>> {
//         InlineSrc::parse_src(byte_arr, index, LBRACK, RBRACK)
//     }
//     fn parse_body(byte_arr: &'a [u8], index: usize) -> Result<Match<&'a str>> {
//         InlineSrc::parse_src(byte_arr, index, LBRACE, RBRACE)
//     }
//     #[inline(always)]
//     fn parse_src(
//         byte_arr: &'a [u8],
//         index: usize,
//         lperim: u8,
//         rperim: u8,
//     ) -> Result<Match<&'a str>> {
//         // Brackets have to be balanced
//         // -1 for left bracket
//         // +1 for right bracket
//         let mut bracket_count: i32 = 0;

//         let mut j: usize = index;

//         loop {
//             match byte_arr[j] {
//                 chr if chr == lperim => {
//                     bracket_count -= 1;
//                 }
//                 chr if chr == rperim => {
//                     bracket_count += 1;
//                     if bracket_count == 0 {
//                         let start = index;
//                         let end = j + 1;
//                         return Ok(Match {
//                             obj: bytes_to_str(&byte_arr[start..end]),
//                             start,
//                             end,
//                         });
//                     }
//                 }
//                 NEWLINE => {
//                     return Err(MatchError);
//                 }
//                 _ => {}
//             }
//             j += 1;
//         }
//     }
// }
