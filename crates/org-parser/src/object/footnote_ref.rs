use crate::constants::{COLON, NEWLINE, RBRACK, SPACE};
use crate::node_pool::NodeID;
use crate::parse::parse_object;
use crate::types::{Cursor, MarkupKind, MatchError, ParseOpts, Parseable, Parser, Result};

// [fn:LABEL]
// [fn:LABEL:DEFINITION]
// [fn::DEFINITION]
#[derive(Debug, Clone)]
pub struct FootnoteRef<'a> {
    pub label: Option<&'a str>,
    pub children: Option<Vec<NodeID>>,
}

impl<'a> Parseable<'a> for FootnoteRef<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.word("[fn:")?;

        // TODO: verify contents of label
        let label_match = cursor.fn_until(|chr| matches!(chr, NEWLINE | COLON | RBRACK | SPACE))?;
        cursor.index = label_match.end;

        match cursor.curr() {
            RBRACK => {
                // [fn:] is not valid
                if label_match.obj.is_empty() {
                    return Err(MatchError::InvalidLogic);
                }

                Ok(parser.alloc(
                    Self {
                        label: Some(label_match.obj),
                        children: None,
                    },
                    start,
                    cursor.index + 1,
                    parent,
                ))
            }
            COLON => {
                cursor.next();
                parse_opts.from_object = false;
                parse_opts.markup.insert(MarkupKind::FootnoteRef);

                let mut content_vec: Vec<NodeID> = Vec::new();
                // if we're being called, that means the first split is the thing
                loop {
                    let begin_def = cursor.index;
                    match parse_object(parser, cursor, parent, parse_opts) {
                        Ok(id) => {
                            cursor.index = parser.pool[id].end;
                            content_vec.push(id);
                        }
                        Err(MatchError::MarkupEnd(kind)) => {
                            if !kind.contains(MarkupKind::FootnoteRef) || cursor.index < start + 2 {
                                return Err(MatchError::InvalidLogic);
                            }

                            // the markup is going to exist,
                            // so update the children's parents
                            let new_id = parser.pool.reserve_id();
                            for id in content_vec.iter_mut() {
                                parser.pool[*id].parent = Some(new_id)
                            }

                            let ret_id = parser.alloc_with_id(
                                Self {
                                    label: if label_match.obj.is_empty() {
                                        None
                                    } else {
                                        Some(label_match.obj)
                                    },
                                    children: Some(content_vec),
                                },
                                start,
                                cursor.index + 1,
                                parent,
                                new_id,
                            );
                            if !label_match.obj.is_empty() {
                                parser.footnotes.insert(label_match.obj, ret_id);
                            }
                            return Ok(ret_id);
                        }
                        ret @ Err(_) => {
                            return ret;
                        }
                    }
                }
            }
            _ => Err(MatchError::InvalidLogic),
        }
    }
}
