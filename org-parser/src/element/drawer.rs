use std::borrow::Cow;
use std::collections::HashMap;

use memchr::memmem::{self, Finder};

use crate::constants::{COLON, HYPHEN, NEWLINE, SPACE, UNDERSCORE};
use crate::node_pool::NodeID;
use crate::object::{parse_node_property, NodeProperty};
use crate::parse::parse_element;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::{bytes_to_str, Match};

const END_TOKEN: &str = ":end:\n";

#[derive(Debug, Clone)]
pub struct Drawer<'a> {
    pub children: Vec<NodeID>,
    pub name: &'a str,
}

impl<'a> Parseable<'a> for Drawer<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.is_index_valid()?;
        cursor.skip_ws();
        cursor.word(":")?;

        let name_match = cursor.fn_until(|chr| {
            chr == COLON
                || chr == NEWLINE
                || !(chr.is_ascii_alphanumeric() || chr == HYPHEN || chr == UNDERSCORE)
        })?;

        cursor.index = name_match.end;
        cursor.word(":")?;
        cursor.skip_ws();
        if cursor.curr() != NEWLINE {
            return Err(MatchError::InvalidLogic);
        }
        cursor.next();
        let potential_loc = find_end(cursor).ok_or(MatchError::InvalidLogic)?;
        let end = potential_loc + cursor.index + END_TOKEN.len();

        let mut moving_loc = potential_loc + cursor.index - 1;
        while cursor[moving_loc] == SPACE {
            moving_loc -= 1;
        }

        if cursor[moving_loc] != NEWLINE {
            return Err(MatchError::InvalidLogic);
        }
        moving_loc += 1;

        // handle empty contents
        // :NAME:
        // :end:
        if cursor.index > moving_loc {
            cursor.index = moving_loc;
        }
        let mut children: Vec<NodeID> = Vec::new();
        let reserve_id = parser.pool.reserve_id();
        let mut temp_cursor = cursor.cut_off(moving_loc);

        // TODO: headings aren't elements, so they cannot be contained here
        // based on org-element, they break the formation of the drawer..
        while let Ok(element_id) =
            // use default parseopts since it wouldn't make sense for the contents
            // of the block to be interpreted as a list, or be influenced from the outside
            parse_element(parser, temp_cursor, Some(reserve_id), ParseOpts::default())
        {
            children.push(element_id);
            temp_cursor.index = parser.pool[element_id].end;
        }

        Ok(parser.alloc_with_id(
            Self {
                children,
                name: name_match.obj,
            },
            start,
            end,
            parent,
            reserve_id,
        ))
    }
}

pub type PropertyDrawer<'a> = HashMap<&'a str, Cow<'a, str>>;

pub(crate) fn parse_property(mut cursor: Cursor) -> Result<Match<PropertyDrawer>> {
    let start = cursor.index;
    cursor.is_index_valid()?;
    cursor.skip_ws();
    cursor.word(":")?;

    let name_match = cursor.fn_until(|chr| chr == COLON || chr == NEWLINE)?;

    if name_match.obj.to_ascii_lowercase() == "properties" {
        // yahoo
    } else {
        return Err(MatchError::InvalidLogic);
    }
    cursor.index = name_match.end;

    cursor.word(":")?;
    cursor.skip_ws();
    if cursor.curr() != NEWLINE {
        return Err(MatchError::InvalidLogic);
    }
    cursor.next();
    let potential_loc = find_end(cursor).ok_or(MatchError::InvalidLogic)?;
    let end = potential_loc + cursor.index + END_TOKEN.len();

    let mut moving_loc = potential_loc + cursor.index - 1;
    while cursor[moving_loc] == SPACE {
        moving_loc -= 1;
    }

    if cursor[moving_loc] != NEWLINE {
        return Err(MatchError::InvalidLogic);
    }
    moving_loc += 1;

    // handle empty contents
    // :properties:
    // :end:
    if cursor.index > moving_loc {
        cursor.index = moving_loc;
    }
    let mut children = HashMap::new();
    let mut temp_cursor = cursor.cut_off(moving_loc);
    loop {
        match parse_node_property(temp_cursor, &mut children) {
            Ok(node_end) => {
                temp_cursor.index = node_end;
            }
            Err(MatchError::EofError) => break,
            Err(e) => return Err(e),
        }
    }

    Ok(Match {
        start,
        end,
        obj: children,
    })
}

fn find_end(cursor: Cursor) -> Option<usize> {
    memmem::find(cursor.rest(), END_TOKEN.as_bytes())
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    use super::*;

    #[test]
    fn basic_drawer() {
        let input = r"

:NAME:
hello
:end:

halloo
";

        let pool = parse_org(input);
        pool.print_tree();
    }
}
