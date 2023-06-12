use std::collections::HashMap;

use crate::constants::{COLON, DOLLAR, HYPHEN, NEWLINE, UNDERSCORE};
use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{Attr, Cursor, Expr, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::Match;

#[derive(Debug, Clone, Copy)]
pub struct Keyword<'a> {
    key: &'a str,
    val: &'a str,
}

#[derive(Debug, Clone)]
pub enum Affiliated<'a> {
    Name(Option<NodeID>),
    Caption(Option<NodeID>, &'a str),
    Attr {
        child_id: Option<NodeID>,
        backend: &'a str,
        val: &'a str,
    },
}

// static ORG_AFFILIATED_KEYWORDS: phf::Set<&str> = phf::phf_set! {
//     "attr_html",
//     "caption",
//     "data",
//     "header",
//     "name",
//     "plot",
//     "results",
// };

impl<'a> Parseable<'a> for Keyword<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.word("#+")?;

        // ,#+attr_html: :class one :class one two three four :attr :attr1
        if cursor.word("attr_").is_ok() {
            let backend = cursor.fn_until(|chr: u8| chr == b':' || chr.is_ascii_whitespace())?;
            cursor.index = backend.end;
            cursor.word(":")?;

            let mut new_attrs: Vec<Attr> = Vec::new();

            // val is in the form
            // :key val :key val :key val
            let val_start_ind = cursor.index;
            loop {
                match cursor.try_curr()? {
                    NEWLINE => break,
                    COLON => {
                        cursor.next();
                        let key_match = cursor.fn_until(|chr| chr.is_ascii_whitespace())?;
                        cursor.index = key_match.end;
                        cursor.skip_ws();
                        if NEWLINE == cursor.try_curr()? {
                            new_attrs.push(Attr {
                                key: key_match.obj.trim(),
                                val: "",
                            });
                            break;
                        }

                        let val_match = cursor.fn_until(|chr| chr == COLON || chr == NEWLINE)?;
                        cursor.index = val_match.end;

                        new_attrs.push(Attr {
                            key: key_match.obj.trim(),
                            val: val_match.obj.trim(),
                        });
                    }
                    _ => cursor.skip_ws(),
                }
            }
            let val = cursor.clamp_backwards(val_start_ind);
            // skip past newline
            cursor.next();
            let end = cursor.index;

            let child_id = loop {
                if let Ok(child_id) = parse_element(parser, cursor, parent, parse_opts) {
                    let node = &mut parser.pool[child_id];
                    if let Expr::Affiliated(aff) = &node.obj {
                        // skip affiliated objects
                        cursor.index = node.end;
                    } else {
                        if let Some(attrs) = &mut node.attrs {
                            attrs
                                .entry(backend.obj)
                                .and_modify(|attr_vec| attr_vec.append(&mut new_attrs));
                        } else {
                            node.attrs = Some(HashMap::from([(backend.obj, new_attrs)]));
                        }
                        break Some(child_id);
                    }
                } else {
                    break None;
                };
            };

            return Ok(parser.alloc(
                Affiliated::Attr {
                    child_id,
                    backend: backend.obj,
                    val: val.trim(),
                },
                start,
                end,
                parent,
            ));
        }
        let key_word = cursor.fn_until(|chr: u8| chr == b':' || chr.is_ascii_whitespace())?;
        cursor.index = key_word.end;
        if cursor.curr() != COLON {
            return Err(MatchError::InvalidLogic)?;
        }

        cursor.next();

        // keywords are pure ascii so use the cheaper option
        match key_word.obj.to_ascii_lowercase().as_str() {
            "macro" => {
                if let Ok(mac) = MacroDef::parse(cursor) {
                    let nam = mac.obj.name;
                    let id = parser.pool.alloc(mac.obj, start, mac.end, parent);
                    parser.macros.insert(nam, id);
                    return Ok(id);
                }
            }
            "name" => {
                let val = cursor.fn_until(|chr: u8| chr == b'\n')?;
                cursor.index = val.end;
                cursor.next();

                let child_id = loop {
                    if let Ok(child_id) = parse_element(parser, cursor, parent, parse_opts) {
                        let node = &mut parser.pool[child_id];
                        if let Expr::Affiliated(aff) = &node.obj {
                            // skip affiliated objects
                            cursor.index = node.end;
                        } else {
                            parser.pool[child_id].id_target =
                                Some(parser.generate_target(val.obj.trim()));
                            break Some(child_id);
                        }
                    } else {
                        break None;
                    };
                };
                let ret_id = parser.alloc(Affiliated::Name(child_id), start, val.end + 1, parent);

                return Ok(ret_id);
            }
            "caption" => {
                let val = cursor.fn_until(|chr: u8| chr == b'\n')?;
                cursor.index = val.end;
                cursor.next();

                let child_id = loop {
                    if let Ok(child_id) = parse_element(parser, cursor, parent, parse_opts) {
                        let node = &mut parser.pool[child_id];
                        if let Expr::Affiliated(aff) = &node.obj {
                            // skip affiliated objects
                            cursor.index = node.end;
                        } else {
                            break Some(child_id);
                        }
                    } else {
                        break None;
                    };
                };

                return Ok(parser.alloc(
                    Affiliated::Caption(child_id, val.obj.trim()),
                    start,
                    val.end + 1,
                    parent,
                ));
            }
            _ => {}
        }

        let val = cursor.fn_until(|chr: u8| chr == b'\n')?;
        // TODO: use an fn_until_inclusive to not have to add 1 to the end
        // (we want to eat the ending nl too)
        parser.keywords.insert(key_word.obj, val.obj.trim());
        Ok(parser.alloc(
            Keyword {
                key: key_word.obj,
                // not mentioned in the spec, but org-element trims
                val: val.obj.trim(),
            },
            start,
            val.end + 1,
            parent,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct MacroDef<'a> {
    // Highest ArgNum
    pub num_args: u32,
    pub input: Vec<ArgNumOrText<'a>>,
    pub name: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum ArgNumOrText<'a> {
    Text(&'a str),
    ArgNum(u32),
}

impl<'a> MacroDef<'a> {
    pub(crate) fn parse(mut cursor: Cursor<'a>) -> Result<Match<Self>> {
        let start = cursor.index;
        // we start just after the colon
        // #+macro: NAME INNER
        // INNER: words $1 is an argument $2 is another
        cursor.skip_ws();
        // A string starting with a alphabetic character followed by any number of
        // alphanumeric characters, hyphens and underscores (-_).
        if !cursor.curr().is_ascii_alphabetic() || cursor.curr() == NEWLINE {
            return Err(MatchError::InvalidLogic);
        }

        let name_match = cursor.fn_while(|chr: u8| {
            chr.is_ascii_alphanumeric() || chr == HYPHEN || chr == UNDERSCORE
        })?;
        cursor.index = name_match.end;

        cursor.skip_ws();
        // macro with no body?
        if cursor.curr() == NEWLINE {
            return Err(MatchError::InvalidLogic);
        }

        // let inner_match = cursor.fn_until(|chr: u8| chr.is_ascii_whitespace())?;
        let mut prev_ind = cursor.index;
        let mut ret_vec: Vec<ArgNumOrText> = Vec::new();
        let mut num_args = 0;
        loop {
            match cursor.curr() {
                DOLLAR => {
                    if cursor.peek(1)?.is_ascii_digit() {
                        ret_vec.push(ArgNumOrText::Text(cursor.clamp_backwards(prev_ind)));
                        // TODO: only supports 9 args rn
                        // parse numbers

                        let arg_ident = (cursor.peek(1)? - 48) as u32;
                        num_args = num_args.max(arg_ident);
                        ret_vec.push(ArgNumOrText::ArgNum(arg_ident));
                        // skip past dollar and number
                        cursor.index += 2;
                        prev_ind = cursor.index;
                    } else {
                        cursor.next();
                    }
                }
                NEWLINE => {
                    ret_vec.push(ArgNumOrText::Text(cursor.clamp_backwards(prev_ind)));
                    break;
                }
                _ => {
                    cursor.next();
                }
            }
        }

        Ok(Match {
            start,
            end: cursor.index + 1,
            obj: Self {
                input: ret_vec,
                num_args,
                name: name_match.obj,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parse_org,
        types::{Attr, Expr},
    };

    #[test]
    fn basic_keyword() {
        let inp = "#+key:val\n";

        dbg!("haiii");
        dbg!(parse_org(inp));
    }

    #[test]
    fn keyword_longer() {
        let inp = "#+intermittent:src_longerlonger\n ending here \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn keyword_ignore_space() {
        let inp = "#+key:                \t    \t              val\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn keyword_ignore_space_nl() {
        let inp = "#+key:     \nval\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn attr_backend_affiliated_keyword() {
        // check for spaces, whitespace between val, black vals and multiple attrs
        let input = r"
#+attr_html: :black yes        :class :words    multiple spaces accepted
|table
";
        let pool = parse_org(input);
        let table = &pool
            .pool
            .iter()
            .find_map(|a| {
                if let Expr::Table(table) = &a.obj {
                    Some(a)
                } else {
                    None
                }
            })
            .unwrap()
            .attrs
            .as_ref()
            .unwrap()["html"];

        assert_eq!(
            table,
            &vec![
                Attr {
                    key: "black",
                    val: "yes"
                },
                Attr {
                    key: "class",
                    val: ""
                },
                Attr {
                    key: "words",
                    val: "multiple spaces accepted"
                },
            ]
        );
    }
}
