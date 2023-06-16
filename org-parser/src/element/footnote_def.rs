use crate::constants::{NEWLINE, RBRACK, SPACE};
use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{Cursor, Expr, ParseOpts, Parseable, Parser, Result};

#[derive(Debug, Clone)]
pub struct FootnoteDef<'a> {
    pub label: &'a str,
    pub children: Vec<NodeID>,
}

impl<'a> Parseable<'a> for FootnoteDef<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        cursor.word("[fn:")?;
        let label_match = cursor.fn_until(|chr| matches!(chr, NEWLINE | RBRACK | SPACE))?;
        cursor.index = label_match.end;
        cursor.word("]")?;

        // Handle CONTENTS
        // used to restore index to the previous position in the event of two
        // blank lines
        let mut blank_obj: Option<NodeID> = None;
        let mut prev_ind = cursor.index;
        let mut children = Vec::new();
        let reserve_id = parser.pool.reserve_id();

        while let Ok(element_id) = parse_element(parser, cursor, Some(reserve_id), parse_opts) {
            let pool_loc = &parser.pool[element_id];
            match &pool_loc.obj {
                Expr::BlankLine => {
                    if blank_obj.is_some() {
                        cursor.index = prev_ind;
                        break;
                    } else {
                        blank_obj = Some(element_id);
                        prev_ind = cursor.index;
                    }
                }
                Expr::FootnoteDef(_) => {
                    break;
                }
                Expr::Heading(_) => {
                    break;
                }
                _ => {
                    if let Some(blank_id) = blank_obj {
                        children.push(blank_id);
                        blank_obj = None;
                    }
                    children.push(element_id);
                }
            }
            cursor.move_to(pool_loc.end);
        }

        parser.footnotes.insert(label_match.obj, reserve_id);
        let ret_id = parser.alloc_with_id(
            Self {
                label: label_match.obj,
                children,
            },
            start,
            cursor.index,
            parent,
            reserve_id,
        );
        // let a: String = format!("fn.{}", parser.footnotes.len());
        // parser.pool[ret_id].id_target = Some(a.into());
        Ok(ret_id)
    }
}
