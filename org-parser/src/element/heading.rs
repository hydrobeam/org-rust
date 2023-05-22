use crate::constants::{COLON, LBRACK, NEWLINE, POUND, RBRACK, SPACE, STAR};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::{parse_element, parse_object};
use crate::types::{Cursor, Expr, MatchError, ParseOpts, Parseable, Result};
use crate::utils::Match;

const ORG_TODO_KEYWORDS: [&str; 2] = ["TODO", "DONE"];

// STARS KEYWORD PRIORITY TITLE TAGS
#[derive(Debug, Clone)]
pub struct Heading<'a> {
    pub heading_level: HeadingLevel,
    // Org-Todo type stuff
    pub keyword: Option<&'a str>,
    pub priority: Option<Priority>,
    pub title: Option<Vec<NodeID>>,
    pub tags: Option<Vec<Tag<'a>>>,
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

#[derive(Debug, Clone, Copy)]
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
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<Match<Expr<'a>>> {
        let start = cursor.index;

        let stars = Heading::parse_stars(cursor)?;
        let heading_level = stars.obj;
        cursor.move_to(stars.end);

        // guaranteed to allocate since this is a valid headline. Setup the id
        let reserved_id = pool.reserve_id();

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

        let mut title_vec: Vec<NodeID> = Vec::new();
        let mut temp_cursor = Cursor::new(cursor.clamp_forwards(tag_match.start).trim().as_bytes());
        while let Ok(title_match) = parse_object(
            pool,
            temp_cursor,
            // run this song and dance to get the trim method
            // TODO: when trim_ascii is stablized on byte_slices, use that
            Some(reserved_id),
            parse_opts,
        ) {
            temp_cursor.index = title_match.end;
            title_vec.push(pool.alloc(title_match, Some(reserved_id)));
        }

        let title = if title_vec.is_empty() {
            None
        } else {
            Some(title_vec)
        };

        // jump past the newline
        cursor.move_to(tag_match.end);

        // Handle subelements

        let mut section_vec: Vec<NodeID> = Vec::new();

        while let Ok(element_match) = parse_element(pool, cursor, Some(reserved_id), parse_opts) {
            if let Expr::Heading(ref mut heading) = element_match.obj {
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

            cursor.index = element_match.end;
            section_vec.push(pool.alloc(element_match, Some(reserved_id)));
        }

        let children = if section_vec.is_empty() {
            None
        } else {
            Some(section_vec)
        };

        Ok(Match {
            start,
            end: cursor.index,
            obj: Self {
                heading_level,
                keyword,
                priority,
                title,
                tags,
                children,
            }
            .into(),
        })
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
        // FIXME breaks in * [#A]EOF
        // one digit: then idx + 4 points to a newline, this must exist
        // two digit: idx + 4 points to RBRACK, also must exist.
        if cursor.len() <= cursor.index + 4
            && !(cursor.curr() == LBRACK && cursor[cursor.index + 1] == POUND)
        {
            return Err(MatchError::EofError);
        }

        assert!(cursor.index + 4 < cursor.len());

        let end_idx;
        let ret_prio: Priority;

        if cursor[cursor.index + 2].is_ascii_alphanumeric() && cursor[cursor.index + 3] == RBRACK {
            end_idx = cursor.index + 4;
            ret_prio = match cursor[cursor.index + 2] {
                b'A' => Priority::A,
                b'B' => Priority::B,
                b'C' => Priority::C,
                num => Priority::Num(num - 48),
            };
        } else if cursor[cursor.index + 2].is_ascii_digit()
            && cursor[cursor.index + 3].is_ascii_digit()
            && cursor[cursor.index + 4] == RBRACK
        {
            end_idx = cursor.index + 5;
            // convert digits from their ascii rep, then add.
            // NOTE: all two digit numbers are valid u8, cannot overflow
            ret_prio = Priority::Num(
                10 * (cursor[cursor.index + 2] - 48) + (cursor[cursor.index + 3] - 48),
            );
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
                    let new_str = cursor.clamp(cursor.index, clamp_ind);
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
    use crate::parse_org;

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

        let a = parse_org(inp);
        a.root().print_tree(&a);
        // dbg!(parse_org(inp));
    }
}
