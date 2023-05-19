use std::path::Path;

use crate::constants::{COLON, LANGLE, LPAREN, RANGLE, RBRACK, RPAREN, SLASH, UNDERSCORE};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_object;
use crate::types::{Cursor, Expr, MarkupKind, MatchError, ParseOpts, Parseable, Result};

const ORG_LINK_PARAMETERS: [&'static str; 9] = [
    "shell", "news", "mailto", "https", "http", "ftp", "help", "file", "elisp",
];

#[derive(Debug, Clone)]
pub struct Link<'a> {
    // actually a pathreg object
    path: PathReg<'a>,
    // One or more objects enclosed by square brackets.
    // It can contain the minimal set of objects as well as export snippets,
    // inline babel calls, inline source blocks, macros, and statistics cookies.
    // It can also contain another link, but only when it is a plain or angle link.
    // It can contain square brackets, so long as they are balanced.
    pub description: Option<Vec<NodeID>>,
}

#[derive(Debug, Clone, Copy)]
pub struct PlainLink<'a> {
    pub protocol: &'a str,
    pub path: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum PathReg<'a> {
    FileName(&'a Path),
    PlainLink(PlainLink<'a>),
    Id(&'a str),
    CustomId(&'a str),
    Coderef(&'a str),
    Fuzzy(&'a str),
}

impl<'a> Parseable<'a> for Link<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        parse_opts.markup.insert(MarkupKind::Link);
        let start = cursor.index;

        let mut content_vec: Vec<NodeID> = Vec::new();
        // if we're being called, that means the first split is the thing
        cursor.next();
        while let Ok(id) = parse_object(pool, cursor, parent, parse_opts) {
            cursor.index = pool[id].end;
            if let Expr::MarkupEnd(leaf) = pool[id].obj {
                if leaf.contains(MarkupKind::Link) {
                    // close object
                    todo!()
                } else {
                    // TODO: cache and explode
                    todo!()
                }
            } else {
                content_vec.push(id);
            }
        }

        todo!()
    }
}

/// Word-constituent characters are letters, digits, and the underscore.
/// source: https://www.gnu.org/software/grep/manual/grep.html
fn is_word_constituent(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == UNDERSCORE
}

/// PROTOCOL
/// A string which is one of the link type strings in org-link-parameters11. PATHPLAIN
///
/// A string containing any non-whitespace character but (, ), <, or >.
/// It must end with a word-constituent character,
/// or any non-whitespace non-punctuation character followed by /.
pub(crate) fn parse_plain_link<'a>(
    pool: &mut NodePool<'a>,
    mut cursor: Cursor<'a>,
    parent: Option<NodeID>,
    parse_opts: ParseOpts,
) -> Result<NodeID> {
    let pre_byte = cursor.peek_rev(1)?;
    if is_word_constituent(pre_byte) {
        return Err(MatchError::InvalidLogic);
    }
    let start = cursor.index;

    for (i, &protocol) in ORG_LINK_PARAMETERS.iter().enumerate() {
        if cursor.word(protocol).is_ok() {
            if cursor.peek(1)? == COLON {
                cursor.next();
                let path_start = cursor.index;
                // let pre
                loop {
                    if let Ok(byte) = cursor.try_curr() {
                        match byte {
                            LPAREN | RPAREN | LANGLE | b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' => {
                                return Err(MatchError::InvalidLogic)
                            }
                            RANGLE => break,
                            _ => {
                                cursor.next();
                            }
                        }
                    } else {
                        break;
                    }
                }
                let last_link_byte = cursor[cursor.index - 1];
                // if no progress was made, i.e. just PROTOCOL:
                if cursor.index == path_start {
                    return Err(MatchError::InvalidLogic);
                }

                if
                //  It must end with a word-constituent character ^
                !(is_word_constituent(last_link_byte))
                // or any non-whitespace non-punctuation character followed by /
                || (!cursor.peek_rev(2)?.is_ascii_whitespace() &&
                    !cursor.peek_rev(2)?.is_ascii_punctuation()
                        && last_link_byte == SLASH)
                // Post ⥿ allow ending on eof
                    || if let Ok(future_byte) = cursor.peek(1) {
                       is_word_constituent(future_byte)
                    } else {
                        true
                    }
                {
                    return Err(MatchError::EofError);
                }

                return Ok(pool.alloc(
                    PlainLink {
                        protocol,
                        path: cursor.clamp_backwards(path_start),
                    },
                    start,
                    cursor.index,
                    parent,
                ));
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
) -> Result<NodeID> {
    let start = cursor.index;

    cursor.next();

    for (i, &protocol) in ORG_LINK_PARAMETERS.iter().enumerate() {
        if cursor.word(protocol).is_ok() {
            if cursor.peek(1)? == COLON {
                cursor.next();
                let path_start = cursor.index;
                loop {
                    if let Ok(byte) = cursor.try_curr() {
                        match byte {
                            RBRACK | LANGLE | b'\n' => return Err(MatchError::InvalidLogic),
                            RANGLE => break,
                            _ => {
                                cursor.next();
                            }
                        }
                    } else {
                        break;
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
