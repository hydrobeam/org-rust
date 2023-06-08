use std::borrow::Cow;
use std::collections::HashMap;

use memchr::memmem::{self, Finder};

use crate::constants::{COLON, NEWLINE, SPACE};
use crate::node_pool::NodeID;
use crate::object::{parse_node_property, NodeProperty};
use crate::parse::parse_element;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::{bytes_to_str, Match};

const END_TOKEN: &str = ":end:\n";

#[derive(Debug, Clone)]
pub struct NamedDrawer<'a> {
    children: Vec<NodeID>,
    name: &'a str,
}

// #[derive(Debug, Clone)]
// pub enum Drawer<'a> {
//     Named(NamedDrawer<'a>),
// }

// #[derive(Debug, Clone)]
// pub struct Property(pub Vec<NodeID>);

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

// impl<'a> Parseable<'a> for PropertyDrawer {
//     fn parse(
//         parser: &mut Parser<'a>,
//         mut cursor: Cursor<'a>,
//         parent: Option<NodeID>,
//         parse_opts: ParseOpts,
//     ) -> Result<NodeID> {
//         let start = cursor.index;
//         cursor.skip_ws();
//         cursor.word(":")?;

//         let name_match = cursor.fn_until(|chr| chr == COLON || chr == NEWLINE)?;

//         if name_match.obj.to_ascii_lowercase() == "properties" {
//             // yahoo
//         } else {
//             return Err(MatchError::InvalidLogic);
//         }

//         cursor.word(":")?;
//         cursor.skip_ws();
//         if cursor.curr() != NEWLINE {
//             return Err(MatchError::InvalidLogic);
//         }
//         let potential_loc = find_end(cursor).ok_or(MatchError::InvalidLogic)?;
//         let end = potential_loc + cursor.index + END_TOKEN.len();

//         let mut moving_loc = potential_loc + cursor.index - 1;
//         while cursor[moving_loc] == SPACE {
//             moving_loc -= 1;
//         }

//         if cursor[moving_loc] != NEWLINE {
//             return Err(MatchError::InvalidLogic);
//         }
//         moving_loc += 1;

//         // handle empty contents
//         // :properties:
//         // :end:
//         if cursor.index > moving_loc {
//             cursor.index = moving_loc;
//         }
//         let mut children = Vec::new();
//         let reserved_id = parser.pool.reserve_id();
//         let mut temp_cursor = cursor.cut_off(moving_loc);
//         loop {
//             match NodeProperty::parse(parser, temp_cursor, Some(reserved_id), parse_opts) {
//                 Ok(id) => {
//                     children.push(id);
//                     temp_cursor.index = parser.pool[id].end;
//                 }
//                 Err(MatchError::EofError) => break,
//                 ret @ Err(_) => return ret,
//             }
//         }

//         Ok(parser.alloc_with_id(Self(children), start, end, parent, reserved_id))
//     }
// }
