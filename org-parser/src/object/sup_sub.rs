use crate::constants::{LBRACE, STAR};
use crate::node_pool::NodeID;
use crate::parse::parse_object;
use crate::types::{Cursor, MarkupKind, MatchError, ParseOpts, Parseable, Parser, Result};

/// Handle superscript and subscript

#[derive(Clone, Debug)]
pub enum PlainOrRec<'a> {
    Plain(&'a str),
    Rec(Vec<NodeID>),
}

macro_rules! parse_nscript {
    ($name: ident) => {
        #[derive(Clone, Debug)]
        pub struct $name<'a>(pub PlainOrRec<'a>);

        impl<'a> Parseable<'a> for $name<'a> {
            fn parse(
                parser: &mut Parser<'a>,
                mut cursor: Cursor<'a>,
                parent: Option<NodeID>,
                mut parse_opts: ParseOpts,
            ) -> Result<NodeID> {
                if cursor.peek_rev(1)?.is_ascii_whitespace() {
                    return Err(MatchError::InvalidLogic);
                }
                let start = cursor.index;
                // skip ^ or _
                cursor.next();

                match cursor.try_curr()? {
                    LBRACE => {
                        cursor.next();

                        parse_opts.markup.insert(MarkupKind::SupSub);
                        let mut content_vec = Vec::new();

                        loop {
                            match parse_object(parser, cursor, parent, parse_opts) {
                                Ok(id) => {
                                    cursor.index = parser.pool[id].end;
                                    content_vec.push(id);
                                }
                                Err(MatchError::MarkupEnd(kind)) => {
                                    if !kind.contains(MarkupKind::SupSub) {
                                        return Err(MatchError::InvalidLogic);
                                    }

                                    let new_id = parser.pool.reserve_id();
                                    for id in content_vec.iter_mut() {
                                        parser.pool[*id].parent = Some(new_id)
                                    }

                                    return Ok(parser.alloc_with_id(
                                        Self(PlainOrRec::Rec(content_vec)),
                                        start,
                                        cursor.index + 1,
                                        parent,
                                        new_id,
                                    ));
                                }
                                ret @ Err(_) => {
                                    return ret;
                                }
                            }
                        }
                    }
                    // STAR => {
                    //     return Ok(parser.alloc(
                    //         Superscript(PlainOrRec::Plain(cursor.clamp_forwards(cursor.index + 2))),
                    //         start,
                    //         cursor.index + 2,
                    //         parent,
                    //     ))
                    // }
                    chr if !chr.is_ascii_whitespace() => {
                        // explicitly ignoring the spec, i disagree with the definition.
                        // SIGN
                        //     Either a plus sign character (+), a minus sign character (-), or the empty string.
                        // CHARS
                        //     Either the empty string, or a string consisting of any number of alphanumeric characters,
                        //     commas, backslashes, and dots.
                        // FINAL
                        //     An alphanumeric character.

                        //     all this is saying is that it has to be: alphanumeric,comma,backslash,dots.
                        //     i don't see why you wouldn't just allow anything.

                        let ret = cursor.fn_until(|chr: u8| chr.is_ascii_whitespace())?;

                        return Ok(parser.alloc(
                            Self(PlainOrRec::Plain(cursor.clamp_forwards(ret.end))),
                            start,
                            ret.end,
                            parent,
                        ));
                    }
                    _ => return Err(MatchError::InvalidLogic)?,
                }
            }
        }
    };
}

parse_nscript!(Subscript);
parse_nscript!(Superscript);

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_sup() {
        let input = r"a^{\smiley}";

        let pool = parse_org(input);
        pool.print_tree();
    }
}
