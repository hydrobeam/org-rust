use crate::constants::{COLON, LBRACK, NEWLINE, POUND, RBRACK, SPACE, STAR};
use crate::node_pool::NodeID;
use crate::parse::{parse_element, parse_object};
use crate::types::{Cursor, Expr, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::{bytes_to_str, Match};

use super::{parse_property, PropertyDrawer};

const ORG_TODO_KEYWORDS: [&str; 2] = ["TODO", "DONE"];

// STARS KEYWORD PRIORITY TITLE TAGS
#[derive(Debug, Clone)]
pub struct Heading<'a> {
    pub heading_level: HeadingLevel,
    // Org-Todo type stuff
    pub keyword: Option<&'a str>,
    pub priority: Option<Priority>,
    pub title: Option<(&'a str, Vec<NodeID>)>,
    pub tags: Option<Vec<Tag<'a>>>,
    pub properties: Option<PropertyDrawer<'a>>,
    pub children: Option<Vec<NodeID>>,
}

#[derive(Debug, Clone)]
pub enum Priority {
    A,
    B,
    C,
    Num(u8),
}

#[derive(Debug, Clone)]
pub enum Tag<'a> {
    Raw(&'a str),
    Loc(NodeID), // Loc refers to the parent headline
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeadingLevel {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

/// Implemented not via TryFrom that MatchError can be private
/// while keeping the struct Public
fn try_heading_levelfrom(value: usize) -> Result<HeadingLevel> {
    match value {
        1 => Ok(HeadingLevel::One),
        2 => Ok(HeadingLevel::Two),
        3 => Ok(HeadingLevel::Three),
        4 => Ok(HeadingLevel::Four),
        5 => Ok(HeadingLevel::Five),
        6 => Ok(HeadingLevel::Six),
        _ => Err(MatchError::InvalidLogic),
    }
}

impl From<HeadingLevel> for u8 {
    fn from(value: HeadingLevel) -> Self {
        match value {
            HeadingLevel::One => 1,
            HeadingLevel::Two => 2,
            HeadingLevel::Three => 3,
            HeadingLevel::Four => 4,
            HeadingLevel::Five => 5,
            HeadingLevel::Six => 6,
        }
    }
}

impl<'a> Parseable<'a> for Heading<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        let stars = Heading::parse_stars(cursor)?;
        let heading_level = stars.obj;
        cursor.move_to(stars.end);

        // guaranteed to allocate since this is a valid headline. Setup the id
        let reserved_id = parser.pool.reserve_id();

        let keyword: Option<&str> = if let Ok(keyword_match) = Heading::parse_keyword(cursor) {
            cursor.move_to(keyword_match.end);
            Some(keyword_match.obj)
        } else {
            None
        };

        let priority: Option<Priority> = if let Ok(prio_match) = Heading::parse_priority(cursor) {
            cursor.move_to(prio_match.end);
            Some(prio_match.obj)
        } else {
            None
        };

        let tag_match = Heading::parse_tag(cursor);
        // if the tags are valid:
        // tag_match.start: space
        // tag_match.end: past newline
        //
        // otherwise:
        //
        // tag_match.start: newline
        // tag_match.end: past newline
        let tags = tag_match.obj;

        // use separate idx and shorten the bottom and top of the byte_arr
        // to trim

        // try to trim whitespace off the beginning and end of the area
        // we're searching
        let mut title_end = tag_match.start;
        while cursor[title_end] == SPACE && title_end > cursor.index {
            title_end -= 1;
        }
        let mut temp_cursor = cursor.cut_off(title_end + 1);

        let mut target = None;
        // FIXME: currently repeating work trimming hte beginning at skip_ws and with trim_start
        let title = if bytes_to_str(temp_cursor.rest()).trim_start().is_empty() {
            None
        } else {
            let mut title_vec: Vec<NodeID> = Vec::new();

            temp_cursor.skip_ws();
            let title_start = temp_cursor.index;
            while let Ok(title_id) =
                parse_object(parser, temp_cursor, Some(reserved_id), parse_opts)
            {
                title_vec.push(title_id);
                temp_cursor.move_to(parser.pool[title_id].end);
            }

            let title_entry = cursor.clamp(title_start, title_end + 1);
            target = Some(parser.generate_target(title_entry));

            Some((title_entry, title_vec))
        };

        // jump past the newline
        cursor.move_to(tag_match.end);

        // Handle subelements

        let properties = if let Ok(ret) = parse_property(cursor) {
            cursor.index = ret.end;
            Some(ret.obj)
        } else {
            None
        };

        let mut section_vec: Vec<NodeID> = Vec::new();

        while let Ok(element_id) = parse_element(parser, cursor, Some(reserved_id), parse_opts) {
            if let Expr::Heading(ref mut heading) = parser.pool[element_id].obj {
                if u8::from(heading_level) < u8::from(heading.heading_level) {
                    if let Some(tag_vec) = &mut heading.tags {
                        tag_vec.push(Tag::Loc(reserved_id));
                    } else {
                        heading.tags = Some(vec![Tag::Loc(reserved_id)]);
                    }
                } else {
                    break;
                }
            }

            section_vec.push(element_id);
            cursor.move_to(parser.pool[element_id].end);
        }

        let children = if section_vec.is_empty() {
            None
        } else {
            Some(section_vec)
        };

        let ret_id = parser.alloc_with_id(
            Self {
                heading_level,
                keyword,
                priority,
                title,
                tags,
                children,
                properties,
            },
            start,
            cursor.index,
            parent,
            reserved_id,
        );
        parser.pool[ret_id].id_target = target;
        Ok(ret_id)
    }
}

impl<'a> Heading<'a> {
    fn parse_stars(cursor: Cursor) -> Result<Match<HeadingLevel>> {
        let ret = cursor.fn_while(|chr: u8| chr == STAR)?;

        if cursor[ret.end] != SPACE {
            Err(MatchError::InvalidLogic)
        } else {
            let heading_level: HeadingLevel = try_heading_levelfrom(ret.end - cursor.index)?;
            Ok(Match {
                start: cursor.index,
                end: ret.end,
                obj: heading_level,
            })
            // Ok(ret.end);
        }
    }

    fn parse_keyword(mut cursor: Cursor) -> Result<Match<&str>> {
        let start = cursor.index;
        cursor.skip_ws();

        for (i, val) in ORG_TODO_KEYWORDS.iter().enumerate() {
            // TODO: read up to a whitespace and determine if it's in phf set for keywords
            // this is currently O(n), we can make it O(1)
            if cursor.word(val).is_ok() {
                // keep going in if not whitespace
                // because a keyword might be a subset of another,,,
                if cursor.try_curr()?.is_ascii_whitespace() {
                    return Ok(Match {
                        start,
                        end: cursor.index, // don't move 1 ahead, in case it's a newline
                        obj: val,
                    });
                } else {
                    cursor.index -= val.len();
                }
            }
        }

        Err(MatchError::InvalidLogic)
    }

    // Recognizes the following patterns:
    // [#A]
    // [#1]
    // [#12]
    // TODO: we don't respect the 65 thing for numbers
    fn parse_priority(mut cursor: Cursor) -> Result<Match<Priority>> {
        let start = cursor.index;
        cursor.skip_ws();
        // TODO: check if this is true
        // FIXME breaks in * [#A]EOF
        // one digit: then idx + 4 points to a newline, this must exist
        // two digit: idx + 4 points to RBRACK, also must exist.

        let end_idx;
        let ret_prio: Priority;
        cursor.word("[#")?;

        if cursor.try_curr()?.is_ascii_alphanumeric() && cursor.peek(1)? == RBRACK {
            end_idx = cursor.index + 2;
            ret_prio = match cursor.curr() {
                b'A' => Priority::A,
                b'B' => Priority::B,
                b'C' => Priority::C,
                num => Priority::Num(num - 48),
            };
        } else if cursor.curr().is_ascii_digit()
            && cursor.peek(1)?.is_ascii_digit()
            && cursor.peek(2)? == RBRACK
        {
            end_idx = cursor.index + 3;
            // convert digits from their ascii rep, then add.
            // NOTE: all two digit numbers are valid u8, cannot overflow
            ret_prio = Priority::Num(10 * (cursor.curr() - 48) + (cursor.peek(1)? - 48));
        } else {
            return Err(MatchError::InvalidLogic);
        }

        Ok(Match {
            start,
            end: end_idx,
            obj: ret_prio,
        })
    }

    // return usize is the end of where we parse the title
    fn parse_tag(mut cursor: Cursor) -> Match<Option<Vec<Tag>>> {
        let start = cursor.index;
        cursor.adv_till_byte(NEWLINE);
        let nl_loc = cursor.index;
        cursor.prev();

        // might help optimize out bounds checks?
        // assert!(temp_ind < cursor.len());

        while cursor.curr() == SPACE {
            cursor.prev();
        }

        if cursor.curr() == COLON {
            let mut clamp_ind = cursor.index;
            cursor.prev();
            let mut tag_vec: Vec<Tag> = Vec::new();

            while cursor.index >= start {
                if cursor.curr().is_ascii_alphanumeric()
                    | matches!(cursor.curr(), b'_' | b'@' | b'#' | b'%')
                {
                    cursor.prev();
                } else if cursor.curr() == COLON && clamp_ind.abs_diff(cursor.index) > 1 {
                    let new_str = cursor.clamp(cursor.index + 1, clamp_ind);
                    tag_vec.push(Tag::Raw(new_str));
                    clamp_ind = cursor.index;
                    if cursor[cursor.index - 1] == SPACE {
                        // end the search
                        return Match {
                            start: cursor.index - 1,
                            end: nl_loc + 1,
                            obj: Some(tag_vec),
                        };
                    } else {
                        // otherwise, keep going
                        cursor.prev();
                    }
                } else {
                    // invalid input: reset temp_ind back to end
                    return Match {
                        start: nl_loc,
                        end: nl_loc + 1,
                        obj: None,
                    };
                }
            }
        }

        Match {
            start: nl_loc,
            end: nl_loc + 1,
            obj: None,
        }
        // we reached the start element, without hitting a space. no tags
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::{element::PropertyDrawer, parse_org, types::Expr};

    #[test]
    fn basic_headline() {
        let inp = "* \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_stars() {
        let inp = "****  \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_too_many_stars() {
        let inp = "*********  \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_title() {
        let inp = "*         title                                                \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_keyword() {
        let inp = "* TODO \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_prio() {
        let inp = "* [#A] \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_tag() {
        let inp = "* meow :tagone:\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_tags() {
        let inp = "* meow :tagone:tagtwo:\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_tags_bad() {
        let inp = "* meow one:tagone:tagtwo:\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_tags_bad2() {
        let inp = "* meow :tagone::\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_prio_keyword() {
        let inp = "* TODO [#A] \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_prio_keyword_title() {
        let inp = "* TODO [#A] SWAG \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_prio_keyword_decorated_title() {
        let inp = "* TODO [#A] *one* two /three/ /four* \n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn headline_everything() {
        let inp = r"* DONE [#0] *one* two /three/ /four*       :one:two:three:four:
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this

is a different paragraph
id) =
more subcontent

* [#4] separate andy
";

        let pool = parse_org(inp);
        pool.print_tree();
        // dbg!(parse_org(inp));
    }

    #[test]
    fn properties_check() {
        let input = r"
* a
:properties:
:name: val
:end:

";

        let pool = parse_org(input);

        let head = pool.pool.iter().find_map(|x| {
            if let Expr::Heading(heading) = &x.obj {
                Some(heading)
            } else {
                None
            }
        });
        let got_prop = head.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(
            got_prop,
            &PropertyDrawer::from([("name", Cow::from("val"))])
        );

        let input = r"
* a
:properties:
:name: val
:name+: val again
:end:

";
        let pool = parse_org(input);

        let head = pool.pool.iter().find_map(|x| {
            if let Expr::Heading(heading) = &x.obj {
                Some(heading)
            } else {
                None
            }
        });
        let got_prop = head.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(
            got_prop,
            &PropertyDrawer::from([("name", Cow::from("val val again"))])
        );
    }

    #[test]
    fn tag_parse() {
        let input = r"
* q ac:qbc:
qqqqq

aaaa";

        let pool = parse_org(input);
        pool.print_tree();
    }
}
