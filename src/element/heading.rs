use crate::constants::{COLON, LBRACK, NEWLINE, POUND, RBRACK, SPACE, STAR};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::{parse_element, parse_object};
use crate::types::{Expr, MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, skip_ws, word, Match};

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

impl TryFrom<usize> for HeadingLevel {
    type Error = MatchError;

    fn try_from(value: usize) -> Result<Self> {
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
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let mut curr_ind = index;

        let stars = Heading::parse_stars(byte_arr, curr_ind)?;
        let heading_level = stars.obj;

        // guaranteed to allocate since this is a valid headline. Setup the id
        let reserved_id = pool.reserve_id();
        curr_ind = stars.end;

        let keyword: Option<&str> =
            if let Ok(keyword_match) = Heading::parse_keyword(byte_arr, curr_ind) {
                curr_ind = keyword_match.end;
                Some(keyword_match.obj)
            } else {
                None
            };

        let priority: Option<Priority> =
            if let Ok(prio_match) = Heading::parse_priority(byte_arr, curr_ind) {
                curr_ind = prio_match.end;
                Some(prio_match.obj)
            } else {
                None
            };

        let tag_match = Heading::parse_tag(byte_arr, curr_ind);
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

        let mut title_idx = 0;
        let mut title_vec: Vec<NodeID> = Vec::new();
        while let Ok(title_id) = parse_object(
            pool,
            bytes_to_str(&byte_arr[curr_ind..tag_match.start])
                .trim()
                .as_bytes(),
            // run this song and dance to get the trim method
            // TODO: when trim_ascii is stablized on byte_slices, use that
            title_idx,
            Some(reserved_id),
            parse_opts,
        ) {
            title_vec.push(title_id);
            title_idx = pool[title_id].end;
        }

        let title = if title_vec.is_empty() {
            None
        } else {
            Some(title_vec)
        };

        // jump past the newline
        curr_ind = tag_match.end;

        // Handle subelements

        let mut section_vec: Vec<NodeID> = Vec::new();

        while let Ok(element_id) =
            parse_element(pool, byte_arr, curr_ind, Some(reserved_id), parse_opts)
        {
            if let Expr::Heading(ref mut heading) = pool[element_id].obj {
                if u8::from(heading_level) < u8::from(heading.heading_level) {
                    if let Some(tag_vec) = &mut heading.tags {
                        tag_vec.push(Tag::Loc(reserved_id));
                    } else {
                        heading.tags = Some(vec![Tag::Loc(reserved_id)]);
                    }
                    section_vec.push(element_id);
                    curr_ind = pool[element_id].end;
                } else {
                    break;
                }
            } else {
                section_vec.push(element_id);
                curr_ind = pool[element_id].end;
            }
        }

        let children = if section_vec.is_empty() {
            None
        } else {
            Some(section_vec)
        };

        Ok(pool.alloc_with_id(
            Self {
                heading_level,
                keyword,
                priority,
                title,
                tags,
                children,
            },
            index,
            curr_ind,
            parent,
            reserved_id,
        ))
    }
}

impl<'a> Heading<'a> {
    fn parse_stars(byte_arr: &[u8], index: usize) -> Result<Match<HeadingLevel>> {
        let ret = fn_until(byte_arr, index, |chr: u8| chr != STAR)?;

        if byte_arr[ret.end] != SPACE {
            Err(MatchError::InvalidLogic)
        } else {
            let heading_level: HeadingLevel = (ret.end - index).try_into()?;
            Ok(Match {
                start: index,
                end: ret.end,
                obj: heading_level,
            })
            // Ok(ret.end);
        }
    }

    fn parse_keyword(byte_arr: &[u8], index: usize) -> Result<Match<&str>> {
        let idx = skip_ws(byte_arr, index);

        for (i, val) in ORG_TODO_KEYWORDS.iter().enumerate() {
            if let Ok(word_end) = word(byte_arr, idx, val) {
                // can probably break out if this isn't true
                if byte_arr[word_end].is_ascii_whitespace() {
                    return Ok(Match {
                        start: index,
                        end: word_end,
                        obj: val,
                    });
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
    fn parse_priority(byte_arr: &[u8], index: usize) -> Result<Match<Priority>> {
        let idx = skip_ws(byte_arr, index);
        // one digit: then idx + 4 points to a newline, this must exist
        // two digit: idx + 4 points to RBRACK, also must exist.
        if byte_arr.len() <= idx + 4 && !(byte_arr[idx] == LBRACK && byte_arr[idx + 1] == POUND) {
            return Err(MatchError::EofError);
        }

        let end_idx;
        let ret_prio: Priority;

        if byte_arr[idx + 2].is_ascii_alphanumeric() && byte_arr[idx + 3] == RBRACK {
            end_idx = idx + 4;
            ret_prio = match byte_arr[idx + 2] {
                b'A' => Priority::A,
                b'B' => Priority::B,
                b'C' => Priority::C,
                num => Priority::Num(num - 48),
            };
        } else if byte_arr[idx + 2].is_ascii_digit()
            && byte_arr[idx + 3].is_ascii_digit()
            && byte_arr[idx + 4] == RBRACK
        {
            end_idx = idx + 5;
            // convert digits from their ascii rep, then add.
            // NOTE: all two digit numbers are valid u8, cannot overflow
            ret_prio = Priority::Num(10 * (byte_arr[idx + 2] - 48) + (byte_arr[idx + 3] - 48));
        } else {
            return Err(MatchError::InvalidLogic);
        }

        Ok(Match {
            start: index,
            end: end_idx,
            obj: ret_prio,
        })
    }

    // return usize is the end of where we parse the title
    fn parse_tag(byte_arr: &[u8], curr_ind: usize) -> Match<Option<Vec<Tag>>> {
        let nl_loc = byte_arr[curr_ind..]
            .iter()
            .position(|&x| x == NEWLINE)
            .unwrap_or(byte_arr[curr_ind..].len()) // EOF case, just go to the end
            + curr_ind;

        let mut temp_ind = nl_loc - 1;

        // might help optimize out bounds checks?
        assert!(temp_ind < byte_arr.len());

        while byte_arr[temp_ind] == SPACE {
            temp_ind -= 1;
        }

        if byte_arr[temp_ind] == COLON {
            let mut clamp_ind = temp_ind;
            temp_ind -= 1;
            let mut tag_vec: Vec<Tag> = Vec::new();

            while temp_ind >= curr_ind {
                if byte_arr[temp_ind].is_ascii_alphanumeric()
                    | matches!(byte_arr[temp_ind], b'_' | b'@' | b'#' | b'%')
                {
                    temp_ind -= 1;
                } else if byte_arr[temp_ind] == COLON && clamp_ind.abs_diff(temp_ind) > 1 {
                    let nu_str = &byte_arr[temp_ind + 1..clamp_ind];
                    tag_vec.push(Tag::Raw(bytes_to_str(nu_str)));
                    clamp_ind = temp_ind;
                    if byte_arr[temp_ind - 1] == SPACE {
                        // end the search
                        return Match {
                            start: temp_ind - 1,
                            end: nl_loc + 1,
                            obj: Some(tag_vec),
                        };
                    } else {
                        // otherwise, keep going
                        temp_ind -= 1;
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
