use std::cell::RefCell;

use crate::node_pool::{NodeID, NodePool};
use crate::{constants::*, object};

use crate::element::{Block, Comment, Heading, Keyword, Paragraph, PlainList};
use crate::object::{Bold, Code, InlineSrc, Italic, Link, StrikeThrough, Underline, Verbatim};
use crate::types::{Expr, MarkupKind, MatchError, Node, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, is_list_start, variant_eq, verify_markup};

pub(crate) fn parse_element<'a, 'b>(
    pool: &'b mut NodePool<'a>,
    byte_arr: &'a [u8],
    index: usize,
    parent: Option<NodeID>,
    mut parse_opts: ParseOpts,
) -> Result<NodeID> {
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
            if let ret @ Ok(_) = Keyword::parse(pool, byte_arr, index, parent, parse_opts) {
                return ret;
            }
            // else if let Ok(block) = Block::parse(byte_arr, index, parse_opts) {
            //     // return r;
            else if let ret @ Ok(_) = Comment::parse(pool, byte_arr, index, parent, parse_opts) {
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
                            return Ok(pool.alloc(
                                Expr::BlankLine,
                                index,
                                idx + 1,
                                parent,
                            ));
                        } else {
                            parse_opts.indentation_level += 1;
                            idx += 1;
                        }
                    } else {
                        // every element will explode if there's an indentation level
                        // except for lsits
                        if is_list_start(byte) {
                            return PlainList::parse(pool, byte_arr, index, parent, parse_opts);
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
        return Ok(parse_paragraph(pool, byte_arr, index, parent, parse_opts));
    } else {
        return Err(MatchError::InvalidLogic);
    }
    // todo!()
}

fn parse_text<'a, 'b>(
    pool: &'b mut NodePool<'a>,
    byte_arr: &'a [u8],
    index: usize,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
) -> NodeID {
    let mut idx = index;
    // dbg!("in text");
    loop {
        // dbg!(idx);
        match parse_object(pool, byte_arr, idx, parent, parse_opts) {
            Ok(_) | Err(MatchError::EofError) => break,
            Err(MatchError::InvalidLogic) => {
                idx += 1;
            }
        }
    }

    pool
        .alloc(bytes_to_str(&byte_arr[index..idx]), index, idx, parent)
}

macro_rules! handle_markup {
    ($name: tt, $pool: ident, $byte_arr: ident, $index: ident, $parent: ident, $parse_opts: ident) => {
        if $parse_opts.markup.contains(MarkupKind::$name) && verify_markup($byte_arr, $index, true)
        {
            return Ok($pool
                .alloc(MarkupKind::$name, $index, $index + 1, None));
        } else if verify_markup($byte_arr, $index, false) {
            let mut new_opts = $parse_opts.clone();

            new_opts.from_object = false;
            new_opts.markup = MarkupKind::$name;
            if let ret @ Ok(_) = $name::parse($pool, $byte_arr, $index, $parent, new_opts) {
                return ret;
            }
        }
    };
}

pub(crate) fn parse_object<'a, 'b>(
    pool: &'b mut NodePool<'a>,
    byte_arr: &'a [u8],
    index: usize,
    parent: Option<NodeID>,
    mut parse_opts: ParseOpts,
) -> Result<NodeID> {
    if let None = byte_arr.get(index) {
        return Err(MatchError::EofError);
    }
    assert!(index < byte_arr.len());

    match byte_arr[index] {
        SLASH => {
            handle_markup!(Italic, pool, byte_arr, index, parent, parse_opts);
        }
        STAR => {
            handle_markup!(Bold, pool, byte_arr, index, parent, parse_opts);
        }
        UNDERSCORE => {
            handle_markup!(Underline, pool, byte_arr, index, parent, parse_opts);
        }
        PLUS => {
            handle_markup!(StrikeThrough, pool, byte_arr, index, parent, parse_opts);
        }
        EQUAL => {
            if let ret @ Ok(_) = Verbatim::parse(pool, byte_arr, index, parent, parse_opts) {
                return ret;
            }
        }
        TILDE => {
            if let ret @ Ok(_) = Code::parse(pool, byte_arr, index, parent, parse_opts) {
                return ret;
            }
        }
        LBRACK => {
            if let ret @ Ok(_) = Link::parse(pool, byte_arr, index, parent, parse_opts) {
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

            match parse_element(pool, byte_arr, index + 1, parent, parse_opts) {
                Ok(_) => return Err(MatchError::InvalidLogic),
                Err(MatchError::InvalidLogic) => {
                    return Ok(pool
                        .alloc(Expr::SoftBreak, index, index + 1, parent))
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
        return Ok(parse_text(pool, byte_arr, index, parent, parse_opts));
    }
}

fn parse_paragraph<'a, 'b>(
    pool: &'b mut NodePool<'a>,
    byte_arr: &'a [u8],
    index: usize,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
) -> NodeID {
    let mut content_vec: Vec<NodeID> = Vec::new();

    let mut idx = index;

    loop {
        match parse_object(pool, byte_arr, idx, parent, parse_opts) {
            Ok(id) => {
                idx = pool[id].end;
                content_vec.push(id);
            }
            Err(_) => {
                // TODO: cache
                break;
            }
        }
    }

    pool.alloc(
        Paragraph(content_vec),
        index,
        idx + 1, // newline
        parent,
    )
}
