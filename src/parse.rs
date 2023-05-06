use crate::constants::*;

use crate::element::{Block, Comment, Heading, Keyword, Paragraph};
use crate::object::{Bold, InlineSrc, Italic, Link, Verbatim};
use crate::types::{Leaf, MarkupKind, Match, MatchError, Node, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, variant_eq, verify_markup};

pub(crate) fn parse_element(byte_arr: &[u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
    // dbg!("testing");
    // let meow = byte_arr.get(index).ok_or(MatchError)?;
    if let None = byte_arr.get(index) {
        return Err(MatchError::EofError);
    }
    assert!(index < byte_arr.len());
    match byte_arr[index] {
        // STAR => {
        //     if let ret @ Ok(_) = Heading::parse(byte_arr, index, parse_opts) {
        //          ret
        //     } else {
        //         return parse_paragraph(byte_arr, index, parse_opts);
        //     }
        // }
        // POUND => {
        //     if let Ok(keyword) = Keyword::parse(byte_arr, index, parse_opts) {
        //         // return r;
        //     } else if let Ok(block) = Block::parse(byte_arr, index, parse_opts) {
        //         // return r;
        //     } else if let (comment) = Comment::parse(byte_arr, index, parse_opts) {
        //         // return r;
        //     } else {
        //     }
        //     // ret = Block::carse(byte_arr, index);
        // }
        // VBAR => {
        //     if let Ok(table) = Table::parse(byte_arr, index) {
        //     } else {
        //     }
        // }
        // chr if chr.is_ascii_whitespace() => {
        //     // TODO: idk
        //     // read until a non ws or newline character is hit? if it's hit and we aren't in a special character,
        //     // (.. i.e. just a list item pretty much?)
        //     // then reset the index (or never update it anyways) and parse the line
        //     //
        //     //
        // } // HYPHEN => {
        //     if let Ok(list) = List::parse(byte_arr, index) {
        //     } else {
        //     }
        // }
        // _ => parse_paragraph(byte_arr, index, parse_opts),
        _ => {
            if !parse_opts.from_paragraph {
                parse_paragraph(byte_arr, index, parse_opts)
            } else {
                Err(MatchError::InvalidLogic)
            }
        }
    }

    // todo!()
}

fn parse_text(byte_arr: &[u8], index: usize, parse_opts: ParseOpts) -> Match<Leaf> {
    let mut idx = index;
    // let ret = *byte_arr.get(index).ok_or(MatchError)?;
    loop {
        match parse_object(byte_arr, idx, parse_opts) {
            Ok(_) | Err(MatchError::EofError) => break,
            Err(MatchError::InvalidLogic) => {
                idx += 1;
            }
        }
    }

    Match {
        obj: Leaf::Plain(bytes_to_str(&byte_arr[index..idx])),
        start: index,
        end: idx,
    }
}

pub(crate) fn parse_object(
    byte_arr: &[u8],
    index: usize,
    mut parse_opts: ParseOpts,
) -> Result<Node> {
    if let None = byte_arr.get(index) {
        return Err(MatchError::EofError);
    }
    assert!(index < byte_arr.len());

    match byte_arr[index] {
        LBRACK => {
            if let ret @ Ok(_) = Link::parse(byte_arr, index, parse_opts) {
                return ret;
            }
        }
        SLASH => {
            if parse_opts.markup.contains(MarkupKind::Italic)
                && verify_markup(byte_arr, index, true)
            {
                return Ok(Node::make_leaf(MarkupKind::Italic, index, index + 1));
            } else if verify_markup(byte_arr, index, false) {
                let mut new_opts = parse_opts.clone();

                new_opts.from_object = false;
                new_opts.markup = MarkupKind::Italic;
                if let ret @ Ok(_) = Italic::parse(byte_arr, index, new_opts) {
                    return ret;
                }
            }
        }
        STAR => {
            if parse_opts.markup.contains(MarkupKind::Bold) && verify_markup(byte_arr, index, true)
            {
                return Ok(Node::make_leaf(MarkupKind::Bold, index, index + 1));
            }

            let mut new_opts = parse_opts.clone();

            new_opts.from_object = false;
            new_opts.markup = MarkupKind::Bold;
            if verify_markup(byte_arr, index, false) {
                if let ret @ Ok(_) = Bold::parse(byte_arr, index, new_opts) {
                    return ret;
                }
            }
        }
        EQUAL => {
            let mut new_opts = parse_opts.clone();

            new_opts.from_object = false;
            new_opts.markup = MarkupKind::Verbatim;

            if let ret @ Ok(_) = Verbatim::parse(byte_arr, index, new_opts) {
                return ret;
            }
        }
        // RBRACK => {
        //     // [[one][]]
        //     if parse_opts.in_link {
        //         return Ok(Node::make_le(Match {
        //             obj: Node::MarkupEnd(MarkupKind::Link),
        //             start: index,
        //             end: index + 1,
        //         }));
        //     }
        // }
        NEWLINE => {
            parse_opts.from_paragraph = true;

            match parse_element(byte_arr, index + 1, parse_opts) {
                Ok(_) => return Err(MatchError::InvalidLogic),
                Err(MatchError::InvalidLogic) => {
                    return Ok(Node::make_leaf(Leaf::SoftBreak, index, index + 1))
                }
                Err(MatchError::EofError) => return Err(MatchError::EofError),
            }
        }
        _ => {}
    }

    if parse_opts.from_object {
        return Err(MatchError::InvalidLogic);
    } else {
        parse_opts.from_object = true;
        return Ok(Node::Leaf(parse_text(byte_arr, index, parse_opts)));
    }
}

fn parse_paragraph(byte_arr: &[u8], index: usize, parse_opts: ParseOpts) -> Result<Node> {
    let mut content_vec: Vec<Node> = Vec::new();

    let mut idx = index;

    loop {
        // dbg!(1);
        match parse_object(byte_arr, idx, parse_opts) {
            Ok(inner) => {
                idx = inner.get_end();
                content_vec.push(inner);
            }
            Err(_) => {
                // TODO: cache
                break;
            }
        }
    }

    Ok(Node::make_branch(
        Paragraph(content_vec),
        index,
        idx + 1, // newline
    ))
    // match byte_arr[index] {
    //     NEWLINE => {
    //         parse_opts.from_paragraph = true;
    //         if let Err(_) = parse_element(byte_arr, index, parse_opts) {
    //         } else {
    //             return Err(MatchError);
    //         }
    //     }
    //     _ => {}
    // }
    // todo!()
}
