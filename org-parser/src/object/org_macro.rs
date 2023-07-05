use crate::constants::{COMMA, HYPHEN, LPAREN, NEWLINE, RBRACE, RPAREN, UNDERSCORE};
use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Clone)]
pub struct MacroCall<'a> {
    pub name: &'a str,
    pub args: Vec<&'a str>,
}

impl<'a> Parseable<'a> for MacroCall<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        cursor.word("{{{")?;

        if !cursor.curr().is_ascii_alphabetic() {
            return Err(MatchError::InvalidLogic);
        }
        let name_match = cursor.fn_while(|chr: u8| {
            // permitted characters
            chr.is_ascii_alphanumeric()
                || chr == HYPHEN
                || chr == UNDERSCORE
                // start params
                && (chr != LPAREN
                // macro end
                || chr != RBRACE)
        })?;

        // A string starting with a alphabetic character followed by any number of
        // alphanumeric characters, hyphens and underscores (-_).
        cursor.index = name_match.end;

        match cursor.curr() {
            LPAREN => {
                // used to check if we have {{{name()}}} (emtpy func call)
                cursor.next();
                let mut arg_vec = Vec::new();
                let mut prev_ind = cursor.index;
                loop {
                    match cursor.try_curr()? {
                        NEWLINE => {
                            parse_opts.from_paragraph = true;
                            parse_opts.list_line = false;
                            parse_opts.from_object = false;
                            match parse_element(parser, cursor.adv_copy(1), parent, parse_opts) {
                                Ok(_) => return Err(MatchError::InvalidLogic),
                                Err(MatchError::InvalidLogic) => {}
                                ret @ Err(_) => return ret,
                            }
                        }
                        RBRACE => {
                            if cursor.word("}}}").is_ok() {
                                return Err(MatchError::InvalidLogic);
                            }
                        }
                        RPAREN => {
                            arg_vec.push(cursor.clamp_backwards(prev_ind));
                            cursor.word(")}}}")?;
                            return Ok(parser.alloc(
                                MacroCall {
                                    name: name_match.obj,
                                    args: arg_vec,
                                },
                                start,
                                cursor.index,
                                parent,
                            ));
                        }
                        COMMA => {
                            arg_vec.push(cursor.clamp_backwards(prev_ind));
                            prev_ind = cursor.index + 1;
                        }
                        _ => {}
                    }
                    cursor.next();
                }
            }
            RBRACE => {
                cursor.word("}}}")?;
                return Ok(parser.alloc(
                    MacroCall {
                        name: name_match.obj,
                        args: Vec::new(),
                    },
                    start,
                    cursor.index,
                    parent,
                ));
            }
            _ => return Err(MatchError::InvalidLogic),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_macro() {
        let input = r"{{{abc}}}";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn macro_with_args() {
        let input = r"{{{poem(cool, three)}}}";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn basic_macro_def() {
        let input = r"#+macro: poem hiii $1 $2 text
";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn combined_macros() {
        let input = r"#+macro: poem hiii $1 $2 text

{{{poem(cool, three)}}}
";
        let pool = parse_org(input);
        pool.print_tree();
    }
}
