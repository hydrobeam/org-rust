use crate::constants::{
    BACKSLASH, DOLLAR, EQUAL, HYPHEN, LANGLE, LBRACK, NEWLINE, PLUS, POUND, RBRACK, SLASH, STAR,
    TILDE, UNDERSCORE, VBAR,
};
use crate::node_pool::{NodeID, NodePool};

use crate::element::{
    Block, Comment, Heading, Item, Keyword, LatexEnv, Paragraph, PlainList, Table,
};
use crate::object::{
    parse_angle_link, parse_plain_link, Bold, Code, Italic, LatexFragment, RegularLink,
    StrikeThrough, Underline, Verbatim,
};
use crate::types::{Cursor, Expr, MarkupKind, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::verify_markup;

pub(crate) fn parse_element<'a>(
    parser: &mut Parser<'a>,
    mut cursor: Cursor<'a>,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
) -> Result<NodeID> {
    cursor.is_index_valid()?;

    // means a newline checking thing called this, and newline breaks all
    // table rows
    if parse_opts.markup.contains(MarkupKind::Table) {
        return Ok(parser.alloc(MarkupKind::Table, cursor.index, cursor.index + 1, None));
    }

    // indentation check
    let mut indented_loc = cursor.index;
    let mut new_opts = parse_opts;
    loop {
        let byte = cursor[indented_loc];
        if byte.is_ascii_whitespace() {
            if byte == NEWLINE {
                return Ok(parser.alloc(Expr::BlankLine, cursor.index, indented_loc + 1, parent));
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
                Item::parse(parser, cursor.move_to_copy(indented_loc), parent, new_opts)
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
                if let ret @ Ok(_) = PlainList::parse(parser, cursor, parent, parse_opts) {
                    return ret;
                }
            } else if let ret @ Ok(_) = Heading::parse(parser, cursor, parent, ParseOpts::default())
            {
                return ret;
            }
        }
        PLUS => {
            if let ret @ Ok(_) = PlainList::parse(parser, cursor, parent, new_opts) {
                return ret;
            }
        }
        HYPHEN => {
            if let ret @ Ok(_) = PlainList::parse(parser, cursor, parent, new_opts) {
                return ret;
            }
        }
        chr if chr.is_ascii_digit() => {
            if let ret @ Ok(_) = PlainList::parse(parser, cursor, parent, new_opts) {
                return ret;
            }
        }
        POUND => {
            if let ret @ Ok(_) = Keyword::parse(parser, cursor, parent, parse_opts) {
                return ret;
            } else if let ret @ Ok(_) = Block::parse(parser, cursor, parent, parse_opts) {
                return ret;
            } else if let ret @ Ok(_) = Comment::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        BACKSLASH => {
            if let ret @ Ok(_) = LatexEnv::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        VBAR => {
            if let ret @ Ok(_) = Table::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        _ => {}
    }

    if !parse_opts.from_paragraph {
        Paragraph::parse(parser, cursor, parent, parse_opts)
    } else {
        Err(MatchError::InvalidLogic)
    }
}

macro_rules! handle_markup {
    ($name: tt, $parser: ident, $cursor: ident, $parent: ident, $parse_opts: ident) => {
        if $parse_opts.markup.contains(MarkupKind::$name) {
            // None parent cause this
            // FIXME: we allocate in the pool for "marker" return types,,
            if verify_markup($cursor, true) {
                return Ok($parser.alloc(
                    MarkupKind::$name,
                    $cursor.index,
                    $cursor.index + 1,
                    None,
                ));
            } else {
                return Err(MatchError::InvalidLogic);
            }
        } else if let ret @ Ok(_) = $name::parse($parser, $cursor, $parent, $parse_opts) {
            return ret;
        }
    };
}

pub(crate) fn parse_object<'a>(
    parser: &mut Parser<'a>,
    cursor: Cursor<'a>,
    parent: Option<NodeID>,
    mut parse_opts: ParseOpts,
) -> Result<NodeID> {
    match cursor.try_curr()? {
        SLASH => {
            handle_markup!(Italic, parser, cursor, parent, parse_opts);
        }
        STAR => {
            handle_markup!(Bold, parser, cursor, parent, parse_opts);
        }
        UNDERSCORE => {
            handle_markup!(Underline, parser, cursor, parent, parse_opts);
        }
        PLUS => {
            handle_markup!(StrikeThrough, parser, cursor, parent, parse_opts);
        }
        EQUAL => {
            if let ret @ Ok(_) = Verbatim::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        TILDE => {
            if let ret @ Ok(_) = Code::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        LBRACK => {
            if let ret @ Ok(_) = RegularLink::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        RBRACK => {
            // ripped off handle_markup
            // TODO: abstract this
            // if we're in a link description, and we hit ]] , return the ending
            if parse_opts.markup.contains(MarkupKind::Link) {
                // FIXME: we allocate in the pool for "marker" return types,,
                if let Ok(byte) = cursor.peek(1) {
                    if byte == RBRACK {
                        return Ok(parser.alloc(
                            MarkupKind::Link,
                            cursor.index,
                            cursor.index + 2,
                            None,
                        ));
                    }
                }
            }
        }
        BACKSLASH => {
            if let ret @ Ok(_) = LatexFragment::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        DOLLAR => {
            if let ret @ Ok(_) = LatexFragment::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        NEWLINE => {
            parse_opts.from_paragraph = true;
            parse_opts.list_line = false;
            // REVIEW: added to make parsing  a table from a NEWLINE
            // work, not sure if needed elsewhere i.e. why didn't i catch
            // this earlier? any other affected elements?
            parse_opts.from_object = false;

            match parse_element(parser, cursor.adv_copy(1), parent, parse_opts) {
                Err(MatchError::InvalidLogic) => {
                    return Ok(parser.alloc(
                        Expr::SoftBreak,
                        cursor.index,
                        cursor.index + 1,
                        parent,
                    ));
                }
                // EofError isn't exactly the right error for the Ok(_) case
                // but we do it to send a signal to `parse_text` to stop collecting:
                // it catches on EofError
                Ok(_) | Err(MatchError::EofError) => return Err(MatchError::EofError),
                Err(MatchError::InvalidIndentation) => return Err(MatchError::InvalidIndentation),
            }
        }
        LANGLE => {
            if let ret @ Ok(_) = parse_angle_link(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        VBAR => {
            if parse_opts.markup.contains(MarkupKind::Table) {
                return Ok(parser.alloc(MarkupKind::Table, cursor.index, cursor.index + 1, None));
            }
        }
        _ => {}
    }

    if let Ok(plain_link_match) = parse_plain_link(cursor) {
        return Ok(parser.alloc(
            plain_link_match.obj,
            cursor.index,
            plain_link_match.end,
            parent,
        ));
    }

    if parse_opts.from_object {
        Err(MatchError::InvalidLogic)
    } else {
        parse_opts.from_object = true;
        Ok(parse_text(parser, cursor, parent, parse_opts))
    }
}

fn parse_text<'a>(
    parser: &mut Parser<'a>,
    mut cursor: Cursor<'a>,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
) -> NodeID {
    let start = cursor.index;

    while let Err(MatchError::InvalidLogic) = parse_object(parser, cursor, parent, parse_opts) {
        cursor.next();
    }

    parser.alloc(cursor.clamp_backwards(start), start, cursor.index, parent)
}
