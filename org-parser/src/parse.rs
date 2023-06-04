use crate::constants::{
    BACKSLASH, CARET, COLON, DOLLAR, EQUAL, HYPHEN, LANGLE, LBRACK, NEWLINE, PLUS, POUND, RBRACE,
    RBRACK, SLASH, STAR, TILDE, UNDERSCORE, VBAR,
};
use crate::node_pool::NodeID;

use crate::element::{
    Block, Comment, Heading, Item, Keyword, LatexEnv, Paragraph, PlainList, Table,
};
use crate::object::{
    parse_angle_link, parse_plain_link, Bold, Code, Emoji, Italic, LatexFragment, RegularLink,
    StrikeThrough, Subscript, Superscript, Underline, Verbatim, InlineSrc,
};
use crate::types::{Cursor, Expr, MarkupKind, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::verify_markup;

pub(crate) fn parse_element<'a>(
    parser: &mut Parser<'a>,
    mut cursor: Cursor<'a>,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
) -> Result<NodeID> {
    if let Some(id) = parser.cache.get(&cursor.index) {
        return Ok(*id);
    }

    cursor.is_index_valid()?;
    // means a newline checking thing called this, and newline breaks all
    // table rows
    if parse_opts.markup.contains(MarkupKind::Table) {
        return Err(MatchError::MarkupEnd(MarkupKind::Table));
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

    cursor.move_to(indented_loc);
    if let Some(id) = parser.cache.get(&cursor.index) {
        return Ok(*id);
    }

    // let elements have child paragraph elements when they propogate,
    // from_paragraph is only to prevent recursing into a paragraph
    // TODO: less weird to express this maybe..?
    let mut no_para_opts = parse_opts;
    no_para_opts.from_paragraph = false;
    new_opts.from_paragraph = false;

    // for lists: items don't keep track of their indentation level
    if !parse_opts.list_line {
        if indentation_level + 1 == parse_opts.indentation_level.into()
            && parse_opts.from_list
            // stop unindented headings from being lists
            && !(indentation_level == 0 && cursor.curr() == STAR)
        {
            if let ret @ Ok(_) = Item::parse(parser, cursor, parent, new_opts) {
                return ret;
            } else {
                return Err(MatchError::InvalidIndentation);
            }
        } else if indentation_level < parse_opts.indentation_level.into() {
            return Err(MatchError::InvalidIndentation);
        }
    }

    match cursor.curr() {
        STAR => {
            // parse_opts, (doesn't totally matter to use the default vs preloaded,
            // since we account for it, but default makes more sense maybe>?)
            if (indentation_level) > 0 {
                if let ret @ Ok(_) = PlainList::parse(parser, cursor, parent, new_opts) {
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
        chr if chr.is_ascii_alphanumeric() => {
            if let ret @ Ok(_) = PlainList::parse(parser, cursor, parent, new_opts) {
                return ret;
            }
        }
        POUND => {
            if let ret @ Ok(_) = Keyword::parse(parser, cursor, parent, no_para_opts) {
                return ret;
            } else if let ret @ Ok(_) = Block::parse(parser, cursor, parent, no_para_opts) {
                return ret;
            } else if let ret @ Ok(_) = Comment::parse(parser, cursor, parent, no_para_opts) {
                return ret;
            }
        }
        BACKSLASH => {
            if let ret @ Ok(_) = LatexEnv::parse(parser, cursor, parent, no_para_opts) {
                return ret;
            }
        }
        VBAR => {
            if let ret @ Ok(_) = Table::parse(parser, cursor, parent, no_para_opts) {
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
                return Err(MatchError::MarkupEnd(MarkupKind::$name));
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
    if let Some(id) = parser.cache.get(&cursor.index) {
        return Ok(*id);
    }

    match cursor.try_curr()? {
        SLASH => {
            handle_markup!(Italic, parser, cursor, parent, parse_opts);
        }
        STAR => {
            handle_markup!(Bold, parser, cursor, parent, parse_opts);
        }
        UNDERSCORE => {
            handle_markup!(Underline, parser, cursor, parent, parse_opts);

            if let ret @ Ok(_) = Subscript::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
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
                if let Ok(byte) = cursor.peek(1) {
                    if byte == RBRACK {
                        return Err(MatchError::MarkupEnd(MarkupKind::Link));
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
                // it keeps collecting while eating InvalidLogic
                Ok(_) | Err(MatchError::EofError) => return Err(MatchError::EofError),
                // propogate the error backup
                ret @ Err(_) => return ret,
            }
        }
        LANGLE => {
            if let ret @ Ok(_) = parse_angle_link(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        VBAR => {
            if parse_opts.markup.contains(MarkupKind::Table) {
                return Err(MatchError::MarkupEnd(MarkupKind::Table));
            }
        }
        COLON => {
            if let ret @ Ok(_) = Emoji::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        RBRACE => {
            if parse_opts.markup.contains(MarkupKind::SupSub) {
                return Err(MatchError::MarkupEnd(MarkupKind::SupSub));
            }
        }
        CARET => {
            if let ret @ Ok(_) = Superscript::parse(parser, cursor, parent, parse_opts) {
                return ret;
            }
        }
        b's' => {
            if let ret @ Ok(_) = InlineSrc::parse(parser, cursor, parent, parse_opts) {
                return ret;
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
