use crate::constants::{
    BACKSLASH, DOLLAR, EQUAL, HYPHEN, LANGLE, NEWLINE, PLUS, POUND, SLASH, STAR, TILDE, UNDERSCORE,
};
use crate::node_pool::{NodeID, NodePool};

use crate::element::{Block, Comment, Heading, Item, Keyword, LatexEnv, Paragraph, PlainList};
use crate::object::{
    parse_angle_link, parse_plain_link, Bold, Code, Italic, LatexFragment, StrikeThrough,
    Underline, Verbatim,
};
use crate::types::{Cursor, Expr, MarkupKind, MatchError, ParseOpts, Parseable, Result};
use crate::utils::verify_markup;

pub(crate) fn parse_element<'a>(
    pool: &mut NodePool<'a>,
    mut cursor: Cursor<'a>,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
) -> Result<NodeID> {
    cursor.is_index_valid()?;

    // indentation check
    let mut indented_loc = cursor.index;
    let mut new_opts = parse_opts;
    loop {
        let byte = cursor[indented_loc];
        if byte.is_ascii_whitespace() {
            if byte == NEWLINE {
                return Ok(pool.alloc(Expr::BlankLine, cursor.index, indented_loc + 1, parent));
            } else {
                new_opts.indentation_level += 1;
                indented_loc += 1;
            }
        }
        // every element will explode if there's an indentation level
        // except for lsits
        else {
            break;
        }
    }

    let indentation_level = indented_loc - cursor.index;

    // the min indentation level is 0, if it manages to be less than parse_opts' indentation
    // level then we're in a list

    if !parse_opts.list_line {
        if indentation_level + 1 == parse_opts.indentation_level.into() && parse_opts.from_list {
            if let ret @ Ok(_) =
                Item::parse(pool, cursor.move_to_copy(indented_loc), parent, new_opts)
            {
                return ret;
            } else {
                return Err(MatchError::InvalidIndentation);
            }
        } else if indentation_level < parse_opts.indentation_level.into() {
            return Err(MatchError::InvalidIndentation);
        }
    }

    cursor.move_to(indented_loc);

    match cursor.curr() {
        STAR => {
            // parse_opts, (doesn't totally matter to use the default vs preloaded,
            // since we account for it, but default makes more sense maybe>?)
            if (indentation_level) > 0 {
                if let ret @ Ok(_) = PlainList::parse(pool, cursor, parent, parse_opts) {
                    return ret;
                }
            } else if let ret @ Ok(_) = Heading::parse(pool, cursor, parent, ParseOpts::default()) {
                return ret;
            }
        }
        PLUS => {
            if let ret @ Ok(_) = PlainList::parse(pool, cursor, parent, new_opts) {
                return ret;
            }
        }
        HYPHEN => {
            if let ret @ Ok(_) = PlainList::parse(pool, cursor, parent, new_opts) {
                return ret;
            }
        }
        chr if chr.is_ascii_digit() => {
            if let ret @ Ok(_) = PlainList::parse(pool, cursor, parent, new_opts) {
                return ret;
            }
        }
        POUND => {
            if let ret @ Ok(_) = Keyword::parse(pool, cursor, parent, parse_opts) {
                return ret;
            } else if let ret @ Ok(_) = Block::parse(pool, cursor, parent, parse_opts) {
                return ret;
            } else if let ret @ Ok(_) = Comment::parse(pool, cursor, parent, parse_opts) {
                return ret;
            }
        }
        BACKSLASH => {
            if let ret @ Ok(_) = LatexEnv::parse(pool, cursor, parent, parse_opts) {
                return ret;
            }
        }
        // VBAR => {
        //     if let Ok(table) = Table::parse(cursor) {
        //     } else {
        //     }
        // }
        _ => {}
    }

    if !parse_opts.from_paragraph {
        Paragraph::parse(pool, cursor, parent, parse_opts)
    } else {
        Err(MatchError::InvalidLogic)
    }
}

macro_rules! handle_markup {
    ($name: tt, $pool: ident, $cursor: ident, $parent: ident, $parse_opts: ident) => {
        if $parse_opts.markup.contains(MarkupKind::$name) {
            // None parent cause this
            // FIXME: we allocate in the pool for "marker" return types,,
            if verify_markup($cursor, true) {
                return Ok($pool.alloc(MarkupKind::$name, $cursor.index, $cursor.index + 1, None));
            } else {
                return Err(MatchError::InvalidLogic);
            }
        } else if let ret @ Ok(_) = $name::parse($pool, $cursor, $parent, $parse_opts) {
            return ret;
        }
    };
}

pub(crate) fn parse_object<'a>(
    pool: &mut NodePool<'a>,
    cursor: Cursor<'a>,
    parent: Option<NodeID>,
    mut parse_opts: ParseOpts,
) -> Result<NodeID> {
    match cursor.try_curr()? {
        SLASH => {
            handle_markup!(Italic, pool, cursor, parent, parse_opts);
        }
        STAR => {
            handle_markup!(Bold, pool, cursor, parent, parse_opts);
        }
        UNDERSCORE => {
            handle_markup!(Underline, pool, cursor, parent, parse_opts);
        }
        PLUS => {
            handle_markup!(StrikeThrough, pool, cursor, parent, parse_opts);
        }
        EQUAL => {
            if let ret @ Ok(_) = Verbatim::parse(pool, cursor, parent, parse_opts) {
                return ret;
            }
        }
        TILDE => {
            if let ret @ Ok(_) = Code::parse(pool, cursor, parent, parse_opts) {
                return ret;
            }
        }
        // LBRACK => {
        //     if let ret @ Ok(_) = Link::parse(pool, cursor, parent, parse_opts) {
        //         return ret;
        //     }
        // }
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
        BACKSLASH => {
            if let ret @ Ok(_) = LatexFragment::parse(pool, cursor, parent, parse_opts) {
                return ret;
            }
        }
        DOLLAR => {
            if let ret @ Ok(_) = LatexFragment::parse(pool, cursor, parent, parse_opts) {
                return ret;
            }
        }
        NEWLINE => {
            parse_opts.from_paragraph = true;
            parse_opts.list_line = false;

            match parse_element(pool, cursor.adv_copy(1), parent, parse_opts) {
                Err(MatchError::InvalidLogic) => {
                    return Ok(pool.alloc(Expr::SoftBreak, cursor.index, cursor.index + 1, parent));
                }
                // EofError isn't exactly the right error for the Ok(_) case
                // but we do it to send a signal to `parse_text` to stop collecting:
                // it catches on EofError
                Ok(_) | Err(MatchError::EofError) => return Err(MatchError::EofError),
                Err(MatchError::InvalidIndentation) => return Err(MatchError::InvalidIndentation),
            }
        }
        LANGLE => {
            if let ret @ Ok(_) = parse_angle_link(pool, cursor, parent, parse_opts) {
                return ret;
            }
        }
        _ => {}
    }

    if let ret @ Ok(_) = parse_plain_link(pool, cursor, parent, parse_opts) {
        return ret;
    }

    if parse_opts.from_object {
        Err(MatchError::InvalidLogic)
    } else {
        parse_opts.from_object = true;
        Ok(parse_text(pool, cursor, parent, parse_opts))
    }
}

fn parse_text<'a>(
    pool: &mut NodePool<'a>,
    mut cursor: Cursor<'a>,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
) -> NodeID {
    let start = cursor.index;

    while let Err(MatchError::InvalidLogic) = parse_object(pool, cursor, parent, parse_opts) {
        cursor.next();
    }

    pool.alloc(cursor.clamp_backwards(start), start, cursor.index, parent)
}
