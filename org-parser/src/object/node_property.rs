use std::borrow::Cow;

use crate::constants::COLON;
use crate::element::PropertyDrawer;
use crate::node_pool::NodeID;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::{bytes_to_str, Match};
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct NodeProperty<'a> {
    pub name: &'a str,
    pub val: Cow<'a, str>,
}

pub(crate) fn parse_node_property<'a>(
    mut cursor: Cursor<'a>,
    properties: &mut PropertyDrawer<'a>,
    // end index
) -> Result<usize> {
    cursor.is_index_valid()?;
    let start = cursor.index;
    cursor.skip_ws();
    cursor.word(":")?;

    let name_match = cursor.fn_until(|chr| chr == COLON || chr.is_ascii_whitespace())?;
    let name = name_match.obj;
    cursor.index = name_match.end;
    if cursor.curr() != COLON {
        return Err(MatchError::InvalidLogic);
    }

    cursor.index = name_match.end;
    // skip past the colon
    cursor.next();

    let val_match = cursor.fn_until(|chr: u8| chr == b'\n')?;
    let val = val_match.obj.trim();
    if name.ends_with("+") {
        let new_name = name.trim_end_matches('+');
        properties
            .entry(new_name)
            .and_modify(|n| {
                write!(n.to_mut(), " {val}").unwrap(); // writing into a string is always safe
            })
            .or_insert(Cow::from(val));
    } else {
        properties.insert(name, Cow::from(val));
    }

    Ok(val_match.end + 1)
}
