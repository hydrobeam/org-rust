use std::arch::x86_64::_MM_MASK_UNDERFLOW;

use crate::{constants::*, object};

use crate::element::{Block, Comment, Heading, Keyword, Paragraph, PlainList};
use crate::object::{Bold, Code, InlineSrc, Italic, Link, StrikeThrough, Underline, Verbatim};
use crate::types::{
    BlankLine, MarkupKind, Match, MatchError, Node, ParseOpts, Parseable, Result, SoftBreak,
};
use crate::utils::{bytes_to_str, fn_until, is_list_start, variant_eq, verify_markup};

pub(crate) fn parse_element(
    byte_arr: &[u8],
    index: usize,
    mut parse_opts: ParseOpts,
) -> Result<Node> {
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
        POUND => {
            if let ret @ Ok(_) = Keyword::parse(byte_arr, index, parse_opts) {
                return ret;
            }
            // else if let Ok(block) = Block::parse(byte_arr, index, parse_opts) {
            //     // return r;
            else if let ret @ Ok(_) = Comment::parse(byte_arr, index, parse_opts) {
                return ret;
            }

            // else {
            // }
            // ret = Block::carse(byte_arr, index);
        }
        // VBAR => {
        //     if let Ok(table) = Table::parse(byte_arr, index) {
        //     } else {
        //     }
        // }
        chr if chr.is_ascii_whitespace() => {
            {
                let mut idx = index;
                loop {
                    let byte = byte_arr[idx];
                    if byte.is_ascii_whitespace() {
                        if byte == NEWLINE {
                            return Ok(Node::make_leaf(BlankLine, index, idx + 1));
                        } else {
                            parse_opts.indentation_level += 1;
                            idx += 1;
                        }
                    } else {
                        // every element will explode if there's an indentation level
                        // except for lsits
                        if is_list_start(byte) {
                            return PlainList::parse(byte_arr, idx, parse_opts);
                        } else {
                            return Err(MatchError::InvalidLogic);
                        }
                    }
                }
            }
        }

        // // HYPHEN => {
        //     if let Ok(list) = List::parse(byte_arr, index) {
        //     } else {
        //     }
        // }
        // _ => parse_paragraph(byte_arr, index, parse_opts),
        _ => {}
    }

    if !parse_opts.from_paragraph {
        return parse_paragraph(byte_arr, index, parse_opts);
    } else {
        return Err(MatchError::InvalidLogic);
    }
    // todo!()
}

fn parse_text(byte_arr: &[u8], index: usize, parse_opts: ParseOpts) -> Node {
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

    Node::make_leaf(bytes_to_str(&byte_arr[index..idx]), index, idx)
}

macro_rules! handle_markup {
    ($name: tt, $byte_arr: ident, $index: ident, $parse_opts: ident) => {
        if $parse_opts.markup.contains(MarkupKind::$name) && verify_markup($byte_arr, $index, true)
        {
            return Ok(Node::make_leaf(MarkupKind::$name, $index, $index + 1));
        } else if verify_markup($byte_arr, $index, false) {
            let mut new_opts = $parse_opts.clone();

            new_opts.from_object = false;
            new_opts.markup = MarkupKind::$name;
            if let ret @ Ok(_) = $name::parse($byte_arr, $index, new_opts) {
                return ret;
            }
        }
    };
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
        SLASH => {
            handle_markup!(Italic, byte_arr, index, parse_opts);
        }
        STAR => {
            handle_markup!(Bold, byte_arr, index, parse_opts);
        }
        UNDERSCORE => {
            handle_markup!(Underline, byte_arr, index, parse_opts);
        }
        PLUS => {
            handle_markup!(StrikeThrough, byte_arr, index, parse_opts);
        }
        EQUAL => {
            if let ret @ Ok(_) = Verbatim::parse(byte_arr, index, parse_opts) {
                return ret;
            }
        }
        TILDE => {
            if let ret @ Ok(_) = Code::parse(byte_arr, index, parse_opts) {
                return ret;
            }
        }
        LBRACK => {
            if let ret @ Ok(_) = Link::parse(byte_arr, index, parse_opts) {
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
                    return Ok(Node::make_leaf(SoftBreak, index, index + 1))
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
        return Ok(parse_text(byte_arr, index, parse_opts));
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
}
