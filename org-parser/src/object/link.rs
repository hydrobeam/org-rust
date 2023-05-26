use std::fmt::Display;

use crate::constants::{
    BACKSLASH, COLON, HYPHEN, LANGLE, LBRACK, LPAREN, POUND, RANGLE, RBRACK, RPAREN, SLASH,
};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_object;
use crate::types::{Cursor, Expr, MarkupKind, MatchError, NodeCache, ParseOpts, Parseable, Result};
use crate::utils::Match;

const ORG_LINK_PARAMETERS: [&'static str; 9] = [
    "shell", "news", "mailto", "https", "http", "ftp", "help", "file", "elisp",
];

#[derive(Debug, Clone)]
pub struct RegularLink<'a> {
    // actually a pathreg object
    pub path: PathReg<'a>,
    // One or more objects enclosed by square brackets.
    // It can contain the minimal set of objects as well as export snippets,
    // inline babel calls, inline source blocks, macros, and statistics cookies.
    // It can also contain another link, but only when it is a plain or angle link.
    // It can contain square brackets, so long as they are balanced.
    pub description: Option<Vec<NodeID>>,
}

impl Display for PathReg<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathReg::PlainLink(link) => {
                f.write_fmt(format_args!("{}:{}", link.protocol, link.path))
            }
            PathReg::Id(inner) => f.write_fmt(format_args!("id:{inner}")),
            PathReg::CustomId(inner) => f.write_fmt(format_args!("#{inner}")),
            PathReg::Coderef(inner) => f.write_fmt(format_args!("({inner})")),
            PathReg::Unspecified(inner) => f.write_fmt(format_args!("{inner}")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlainLink<'a> {
    pub protocol: &'a str,
    pub path: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum PathReg<'a> {
    PlainLink(PlainLink<'a>),
    Id(&'a str),
    CustomId(&'a str),
    Coderef(&'a str),
    Unspecified(&'a str),
    // We can't determine while parsing whether we point to a headline
    // or a filename (we don't track headlines while building)
    // leave it to the exporter.
    // FileName(&'a Path),
    // Fuzzy(&'a str),
}

impl<'a> PathReg<'a> {
    fn new(cursor: Cursor<'a>) -> Self {
        match cursor.curr() {
            b'i' => {
                if let Ok(id) = PathReg::parse_id(cursor) {
                    return PathReg::Id(id);
                } else if let Ok(link) = parse_plain_link(cursor) {
                    return PathReg::PlainLink(link.obj);
                }
            }
            POUND => {
                // custom-id
                return PathReg::CustomId(cursor.clamp(cursor.index + 1, cursor.len() - 1));
            }
            LPAREN => {
                // FIXME: breaks on ()
                if cursor[cursor.len() - 1] == RPAREN {
                    return PathReg::Coderef(cursor.clamp(cursor.index + 1, cursor.len() - 2));
                }
            }
            chr => {
                if let Ok(link) = parse_plain_link(cursor) {
                    return PathReg::PlainLink(link.obj);
                }
            }
        }
        // unspecified
        return PathReg::Unspecified(cursor.clamp_forwards(cursor.len() - 1));
    }

    fn parse_id(mut cursor: Cursor<'a>) -> Result<&'a str> {
        if cursor.peek(1)? != b'd' && cursor.peek(2)? != COLON {
            return Err(MatchError::InvalidLogic);
        }

        cursor.advance(3);
        let begin_id = cursor.index;

        while let Ok(num) = cursor.try_curr() {
            if !num.is_ascii_hexdigit() || num == HYPHEN {
                return Err(MatchError::InvalidLogic);
            }

            cursor.next();
        }

        return Ok(cursor.clamp_backwards(begin_id));
    }
}

impl<'a> Parseable<'a> for RegularLink<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
        cache: &mut NodeCache,
    ) -> Result<NodeID> {
        let start = cursor.index;

        if cursor.curr() != LBRACK && cursor.peek(1)? != LBRACK {
            return Err(MatchError::InvalidLogic);
        }
        cursor.advance(2);

        // find backslash
        loop {
            match cursor.try_curr()? {
                BACKSLASH => {
                    // check for escaped char, and skip past it
                    if let BACKSLASH | LBRACK | RBRACK = cursor.peek(1)? {
                        cursor.advance(2);
                    } else {
                        return Err(MatchError::InvalidLogic);
                    }
                }
                RBRACK => {
                    if LBRACK == cursor.peek(1)? {
                        let path_reg_end = cursor.index;

                        // skip ][
                        cursor.advance(2);
                        parse_opts.markup.insert(MarkupKind::Link);

                        let mut content_vec: Vec<NodeID> = Vec::new();
                        while let Ok(id) = parse_object(pool, cursor, parent, parse_opts, cache) {
                            cursor.index = pool[id].end;
                            if let Expr::MarkupEnd(leaf) = pool[id].obj {
                                if !leaf.contains(MarkupKind::Link) {
                                    // TODO: cache and explode
                                    return Err(MatchError::InvalidLogic);
                                }

                                let pathreg =
                                    PathReg::new(cursor.clamp_off(start + 2, path_reg_end));

                                // set parents of children
                                // TODO: abstract this? stolen from markup.rs
                                let new_id = pool.reserve_id();
                                for id in content_vec.iter_mut() {
                                    pool[*id].parent = Some(new_id)
                                }
                                return Ok(pool.alloc_with_id(
                                    Self {
                                        path: pathreg,
                                        description: Some(content_vec),
                                    },
                                    start,
                                    cursor.index,
                                    parent,
                                    new_id,
                                ));
                            } else {
                                content_vec.push(id);
                            }
                        }
                    } else if RBRACK == cursor.peek(1)? {
                        // close object;
                        let pathreg = PathReg::new(cursor.clamp_off(start + 2, cursor.index));
                        return Ok(pool.alloc(
                            Self {
                                path: pathreg,
                                description: None,
                            },
                            start,
                            cursor.index + 2,
                            parent,
                        ));
                    } else {
                        return Err(MatchError::InvalidLogic);
                    }
                }
                _ => {}
            }
            cursor.next();
        }
    }
}

/// Word-constituent characters are letters, digits, and the underscore.
/// source: https://www.gnu.org/software/grep/manual/grep.html
///
/// REVIEW:
/// apparently a word constituent..isn't ujndescore??
/// https://www.gnu.org/software/emacs/manual/html_node/elisp/Syntax-Class-Table.html
///
/// Parts of words in human languages.
/// These are typically used in variable and command names in programs.
/// All upper- and lower-case letters, and the digits, are typically word constituents.

/// PROTOCOL
/// A string which is one of the link type strings in org-link-parameters11. PATHPLAIN
///
/// A string containing any non-whitespace character but (, ), <, or >.
/// It must end with a word-constituent character,
/// or any non-whitespace non-punctuation character followed by /.
pub(crate) fn parse_plain_link(mut cursor: Cursor<'_>) -> Result<Match<PlainLink<'_>>> {
    if let Ok(pre_byte) = cursor.peek_rev(1) {
        if pre_byte.is_ascii_alphanumeric() {
            return Err(MatchError::InvalidLogic);
        }
    }
    let start = cursor.index;

    for (i, &protocol) in ORG_LINK_PARAMETERS.iter().enumerate() {
        // DO NOT read up to the colon and use phf_set to determine if it's a protocol
        // cause the colon might be in the middle-a-nowhere if we're parsing regular text here
        if cursor.word(protocol).is_ok() {
            if cursor.try_curr()? == COLON {
                cursor.next();
                let path_start = cursor.index;
                // let pre

                while let Ok(byte) = cursor.try_curr() {
                    match byte {
                        RANGLE | LPAREN | RPAREN | LANGLE | b'\t' | b'\n' | b'\x0C' | b'\r'
                        | b' ' => {
                            break;
                        }
                        // RANGLE => break,
                        _ => {
                            cursor.next();
                        }
                    }
                }

                let last_link_byte = cursor[cursor.index - 1];
                // if no progress was made, i.e. just PROTOCOL:

                // rewind until we end with an alphanumeric char or SLASH
                //
                // so:
                // https://abc.org...___
                // would only get: https://abc.org
                //
                // if you do something like https://onea/a/aaaa/,,,,,/
                // then i think that breaks the definition, cause the slash isn't after a non-punc char..
                // but also if you do that then you're just being difficult.
                //

                while !cursor.peek_rev(1)?.is_ascii_alphanumeric()
                    && !(cursor.peek_rev(1)? == SLASH)
                {
                    cursor.prev();
                    if cursor.index <= path_start {
                        return Err(MatchError::InvalidLogic);
                    }
                }

                // Post: allow ending on eof
                if if let Ok(future_byte) = cursor.peek(1) {
                    !future_byte.is_ascii_alphanumeric()
                } else {
                    true
                } {
                    return Ok(Match {
                        start,
                        end: cursor.index,
                        obj: PlainLink {
                            protocol,
                            path: cursor.clamp_backwards(path_start),
                        },
                    });
                } else {
                    return Err(MatchError::EofError);
                }
            } else {
                cursor.index -= protocol.len();
            }
        }
    }

    Err(MatchError::InvalidLogic)
}

pub(crate) fn parse_angle_link<'a>(
    pool: &mut NodePool<'a>,
    mut cursor: Cursor<'a>,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
    cache: &mut NodeCache,
) -> Result<NodeID> {
    let start = cursor.index;

    cursor.next();

    for (i, &protocol) in ORG_LINK_PARAMETERS.iter().enumerate() {
        if cursor.word(protocol).is_ok() {
            if cursor.try_curr()? == COLON {
                cursor.next();
                let path_start = cursor.index;
                while let Ok(byte) = cursor.try_curr() {
                    match byte {
                        RBRACK | LANGLE | b'\n' => return Err(MatchError::InvalidLogic),
                        RANGLE => break,
                        _ => {
                            cursor.next();
                        }
                    }
                }

                // <PROTOCOL:> is valid, don't need to check indices

                return Ok(pool.alloc(
                    PlainLink {
                        protocol,
                        path: cursor.clamp_backwards(path_start),
                    },
                    start,
                    cursor.index + 1, // skip rangle
                    parent,
                ));
            } else {
                cursor.index -= protocol.len();
            }
        }
    }

    Err(MatchError::InvalidLogic)
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_plain_link() {
        let input = "https://swag.org";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn plain_link_subprotocol() {
        // http and https are protocols
        let input = "http://swag.org";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn plain_link_ws_end() {
        // http and https are protocols
        let input = "  mailto:swag@cool.com   ";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn plain_link_word_constituent() {
        // http and https are protocols
        let input = "  https://one_two_three_https______..............~~~!   ";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn plain_link_word_constituent_slash() {
        // http and https are protocols
        let input = "  https://one_two_three_https______/..............~~~!   ";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn basic_angle_link() {
        // http and https are protocols
        let input = "  <https://one two  !!@#!OIO DJDFK Jk> ";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn basic_regular_link() {
        let input = "[[https://meow.org]]";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn regular_link_malformed() {
        let input = "
word
[#A]
";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn regular_link_description() {
        let input = " [[https://meo][cool site]]";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn regular_link_unclosed_recursive_markup() {
        let input = " [[https://meo][cool *site* ~one two~ three *four ]]";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn regular_link_unclosed_plain_markup() {
        let input = " [[https://meo][cool *site* ~one two~ three *four ~five six ]]";
        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }
}
