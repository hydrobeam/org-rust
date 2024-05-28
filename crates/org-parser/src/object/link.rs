use std::borrow::Cow;
use std::fmt::Display;

use crate::constants::{
    BACKSLASH, COLON, HYPHEN, LANGLE, LBRACK, LPAREN, POUND, RANGLE, RBRACK, RPAREN, SLASH,
};
use crate::node_pool::NodeID;
use crate::parse::parse_object;
use crate::types::{Cursor, MarkupKind, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::Match;

const ORG_LINK_PARAMETERS: [&str; 9] = [
    "shell", "news", "mailto", "https", "http", "ftp", "help", "file", "elisp",
];

#[derive(Debug, Clone)]
pub struct RegularLink<'a> {
    pub path: Match<PathReg<'a>>,
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
            PathReg::File(inner) => f.write_fmt(format_args!("file:{inner}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainLink<'a> {
    pub protocol: Cow<'a, str>,
    pub path: Cow<'a, str>,
}

impl From<&PlainLink<'_>> for String {
    fn from(value: &PlainLink) -> Self {
        format!("{}:{}", value.protocol, value.path)
    }
}

/// Enum representing various file types
#[derive(Debug, Clone)]
pub enum PathReg<'a> {
    PlainLink(PlainLink<'a>),
    Id(&'a str),
    /// allows changing the name of the exported id
    CustomId(&'a str),
    /// allows linking to specific lines in code blocks
    Coderef(&'a str),
    File(Cow<'a, str>),
    Unspecified(Cow<'a, str>),
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
            b'f' => {
                if let Ok(file_path) = PathReg::parse_file(cursor) {
                    return PathReg::File(file_path.into());
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
                    return PathReg::Coderef(cursor.clamp(cursor.index + 1, cursor.len() - 1));
                }
            }
            chr => {
                if let Ok(link) = parse_plain_link(cursor) {
                    return PathReg::PlainLink(link.obj);
                }
            }
        }
        // unspecified:
        // We can't determine while parsing whether we point to a headline
        // or a filename (we don't track headlines while building)
        // leave it to the exporter.
        // FileName(&'a Path),
        // Fuzzy(&'a str),
        return PathReg::Unspecified(cursor.clamp_forwards(cursor.len()).into());
    }

    fn parse_id(mut cursor: Cursor<'a>) -> Result<&'a str> {
        cursor.word("id:")?;
        let begin_id = cursor.index;

        while let Ok(num) = cursor.try_curr() {
            if !num.is_ascii_hexdigit() || num == HYPHEN {
                return Err(MatchError::InvalidLogic);
            }
            cursor.next();
        }

        return Ok(cursor.clamp_backwards(begin_id));
    }

    fn parse_file(mut cursor: Cursor<'a>) -> Result<&'a str> {
        cursor.word("file:")?;
        let begin_id = cursor.index;

        while let Ok(num) = cursor.try_curr() {
            cursor.next();
        }

        return Ok(cursor.clamp_backwards(begin_id));
    }
}

impl<'a> Parseable<'a> for RegularLink<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.word("[[")?;

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
                    // handles the  [[][]]  case, would panic without this check
                    if cursor.index == start + 2 {
                        return Err(MatchError::InvalidLogic);
                    }

                    if LBRACK == cursor.peek(1)? {
                        let path_reg_end = cursor.index;

                        // skip ][
                        cursor.advance(2);
                        parse_opts.from_object = false;
                        parse_opts.markup.insert(MarkupKind::Link);

                        let mut content_vec: Vec<NodeID> = Vec::new();
                        loop {
                            match parse_object(parser, cursor, parent, parse_opts) {
                                Ok(id) => {
                                    cursor.index = parser.pool[id].end;
                                    content_vec.push(id);
                                }
                                Err(MatchError::MarkupEnd(kind)) => {
                                    if !kind.contains(MarkupKind::Link) {
                                        // TODO: cache and explode
                                        return Err(MatchError::InvalidLogic);
                                    }

                                    let reg_curs = cursor.clamp_off(start + 2, path_reg_end);
                                    let pathreg = Match {
                                        start: start + 2,
                                        end: path_reg_end,
                                        obj: PathReg::new(reg_curs),
                                    };

                                    // set parents of children
                                    // TODO: abstract this? stolen from markup.rs
                                    let new_id = parser.pool.reserve_id();
                                    for id in &mut content_vec {
                                        parser.pool[*id].parent = Some(new_id);
                                    }

                                    return Ok(parser.alloc_with_id(
                                        Self {
                                            path: pathreg,
                                            description: Some(content_vec),
                                        },
                                        start,
                                        cursor.index + 2, // link end is 2 bytes long
                                        parent,
                                        new_id,
                                    ));
                                }
                                ret @ Err(_) => return ret,
                            }
                        }
                    } else if RBRACK == cursor.peek(1)? {
                        // close object;

                        let reg_curs = cursor.clamp_off(start + 2, cursor.index);
                        let pathreg = Match {
                            start: start + 2,
                            end: cursor.index,
                            obj: PathReg::new(reg_curs),
                        };

                        return Ok(parser.alloc(
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

// REVIEW:
// apparently a word constituent..isn't undescore??
// https://www.gnu.org/software/emacs/manual/html_node/elisp/Syntax-Class-Table.html
// Parts of words in human languages.
// These are typically used in variable and command names in programs.
// All upper- and lower-case letters, and the digits, are typically word constituents.

/// PROTOCOL
/// A string which is one of the link type strings in org-link-parameters.
///
/// PATHPLAIN
/// A string containing any non-whitespace character but (, ), <, or >.
/// It must end with a word-constituent character,
/// or any non-whitespace non-punctuation character followed by /.
// Word-constituent characters are letters, digits, and the underscore.
// source: https://www.gnu.org/software/grep/manual/grep.html
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
                // if no progress was made, i.e. just PROTOCOL (https://):

                // rewind until we end with an alphanumeric char or SLASH
                //
                // so:
                // https://abc.org...___
                // would only get: https://abc.org
                //
                // if you do something like https://onea/a/aaaa/,,,,,/
                // then i think that breaks the definition, cause the slash isn't after a non-punc char,,
                // but also if you do that then you're just being difficult.

                while !cursor.peek_rev(1)?.is_ascii_alphanumeric() && cursor.peek_rev(1)? != SLASH {
                    cursor.prev();
                    if cursor.index <= path_start {
                        return Err(MatchError::InvalidLogic);
                    }
                }

                if if let Ok(future_byte) = cursor.try_curr() {
                    !future_byte.is_ascii_alphanumeric()
                } else {
                    true
                } {
                    return Ok(Match {
                        start,
                        end: cursor.index,
                        obj: PlainLink {
                            protocol: protocol.into(),
                            path: cursor.clamp_backwards(path_start).into(),
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
    parser: &mut Parser<'a>,
    mut cursor: Cursor<'a>,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
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

                return Ok(parser.alloc(
                    PlainLink {
                        protocol: protocol.into(),
                        path: cursor.clamp_backwards(path_start).into(),
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
    use pretty_assertions::assert_eq;

    use crate::expr_in_pool;
    use crate::object::PlainLink;
    use crate::parse_org;
    use crate::types::Expr;

    #[test]
    fn basic_plain_link() {
        let input = "https://swag.org";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, PlainLink).unwrap();
        assert_eq!(
            l,
            &PlainLink {
                protocol: "https".into(),
                path: "//swag.org".into()
            }
        )
    }

    #[test]
    fn plain_link_subprotocol() {
        // http and https are protocols
        let input = "http://swag.org";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, PlainLink).unwrap();
        assert_eq!(
            l,
            &PlainLink {
                protocol: "http".into(),
                path: "//swag.org".into()
            }
        )
    }

    #[test]
    fn plain_link_after() {
        let input = "http://swag.com meow";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, PlainLink).unwrap();
        assert_eq!(
            l,
            &PlainLink {
                protocol: "http".into(),
                path: "//swag.com".into()
            }
        )
    }

    #[test]
    fn plain_link_ws_end() {
        // http and https are protocols
        let input = "  mailto:swag@cool.com   ";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, PlainLink).unwrap();

        assert_eq!(
            l,
            &PlainLink {
                protocol: "mailto".into(),
                path: "swag@cool.com".into()
            }
        )
    }

    #[test]
    fn plain_link_word_constituent() {
        // http and https are protocols
        let input = "  https://one_two_three_https______..............~~~!   ";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, PlainLink).unwrap();

        assert_eq!(
            l,
            &PlainLink {
                protocol: "https".into(),
                path: "//one_two_three_https".into()
            }
        )
    }

    #[test]
    fn plain_link_word_constituent_slash() {
        // http and https are protocols
        let input = "  https://one_two_three_https______/..............~~~!   ";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, PlainLink).unwrap();

        assert_eq!(
            l,
            &PlainLink {
                protocol: "https".into(),
                path: "//one_two_three_https______/".into()
            }
        )
    }

    #[test]
    fn basic_angle_link() {
        // http and https are protocols
        let input = "  <https://one two  !!@#!OIO DJDFK Jk> ";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, PlainLink).unwrap();

        assert_eq!(
            l,
            &PlainLink {
                protocol: "https".into(),
                path: "//one two  !!@#!OIO DJDFK Jk".into()
            }
        )
    }

    #[test]
    fn basic_regular_link() {
        let input = "[[hps://.org]]";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn regular_link_malformed() {
        let input = "
word
[#A]
";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn regular_link_description() {
        let input = " [[https://meo][cool site]]";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn regular_link_unclosed_recursive_markup() {
        let input = " [[https://meo][cool *site* ~one two~ three *four ]]";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn regular_link_unclosed_plain_markup() {
        let input = " [[https://meo][cool *site* ~one two~ three *four ~five six ]]";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn file_link() {
        let input = r"
I'll be skipping over the instrumentals unless there's reason to.

[[file:bmc.jpg]]
** songs
";

        let pool = parse_org(input);
        pool.print_tree();
    }
}
