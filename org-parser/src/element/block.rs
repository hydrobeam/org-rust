use crate::constants::NEWLINE;
use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use memchr::memmem;

#[derive(Debug, Clone)]
pub struct Block<'a> {
    pub kind: BlockKind<'a>,
    pub parameters: Option<&'a str>,
    pub contents: BlockContents<'a>,
}

// TODO; just expost these two different kinds as structs?
#[derive(Debug, Clone)]
pub enum BlockContents<'a> {
    Greater(Vec<NodeID>),
    Lesser(&'a str),
}

impl<'a> Parseable<'a> for Block<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<crate::node_pool::NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor.word("#+begin_")?;

        let block_name_match = cursor.fn_until(|chr: u8| chr.is_ascii_whitespace())?;

        let block_kind: BlockKind;
        let parameters: Option<&str>;
        // if no progress was made looking for the block_type:
        // i.e.: #+begin_\n
        if cursor.index == block_name_match.end {
            return Err(MatchError::InvalidLogic);
        }
        cursor.index = block_name_match.end;
        // parse paramters
        cursor.skip_ws();

        block_kind = block_name_match.obj.into();

        if cursor.curr() == NEWLINE {
            parameters = None;
        } else {
            let params_match = cursor.fn_until(|chr| chr == NEWLINE)?;
            parameters = Some(params_match.obj);
            cursor.index = params_match.end;
        }

        // have to predeclare these so that the allocated string
        // doesn't go out of scope and we can still pull a reference
        // to it.
        let alloc_str;
        let needle;

        // avoid an allocation for pre-known endings
        if let Some(block_end) = block_kind.to_end() {
            needle = block_end;
        } else {
            alloc_str = format!("#+end_{}\n", block_name_match.obj);
            needle = &alloc_str;
        }

        cursor.next();
        let mut it = memmem::find_iter(cursor.rest(), needle.as_bytes());
        // returns result at the start of the needle

        // done this way to handle indented blocks,
        // such as in the case of lists
        //
        let loc;
        let end;
        'l: loop {
            if let Some(potential_loc) = it.next() {
                // - 1 since the match is at the start of the word,
                // which is going to be #
                let mut moving_loc = potential_loc + cursor.index - 1;
                while cursor[moving_loc] != NEWLINE {
                    if !cursor[moving_loc].is_ascii_whitespace() {
                        continue 'l;
                    }
                    moving_loc -= 1;
                }
                loc = moving_loc;
                end = potential_loc + cursor.index + needle.len();
                break;
            } else {
                Err(MatchError::InvalidLogic)?
            }
        }
        // let loc = it.next().ok_or(MatchError::InvalidLogic)? + curr_ind;
        // handle empty contents
        if cursor.index > loc {
            cursor.index = loc;
        }

        if block_kind.is_lesser() {
            Ok(parser.alloc(
                Self {
                    kind: block_kind,
                    parameters,
                    // + 1 to skip newline
                    contents: BlockContents::Lesser(cursor.clamp_forwards(loc)),
                },
                start,
                end,
                parent,
            ))
        } else {
            let mut content_vec: Vec<NodeID> = Vec::new();
            let reserve_id = parser.pool.reserve_id();
            // janky
            let mut temp_cursor = cursor.cut_off(loc);
            while let Ok(element_id) =
                parse_element(parser, temp_cursor, Some(reserve_id), parse_opts)
            {
                content_vec.push(element_id);
                temp_cursor.index = parser.pool[element_id].end;
            }

            Ok(parser.alloc_with_id(
                Self {
                    kind: block_kind,
                    parameters,
                    contents: BlockContents::Greater(content_vec),
                },
                start,
                end,
                parent,
                reserve_id,
            ))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BlockKind<'a> {
    // Greater
    Center,
    Quote,
    Special(&'a str), // holds the block kind

    // Leser
    Comment,
    Example,
    Export,
    Src,
    Verse,
}

impl BlockKind<'_> {
    pub fn is_lesser(&self) -> bool {
        matches!(
            self,
            BlockKind::Comment
                | BlockKind::Example
                | BlockKind::Export
                | BlockKind::Src
                | BlockKind::Verse
        )
    }

    fn to_end(&self) -> Option<&str> {
        match self {
            BlockKind::Center => Some("#+end_center\n"),
            BlockKind::Quote => Some("#+end_quote\n"),
            BlockKind::Comment => Some("#+end_comment\n"),
            BlockKind::Example => Some("#+end_example\n"),
            BlockKind::Export => Some("#+end_export\n"),
            BlockKind::Src => Some("#+end_src\n"),
            BlockKind::Verse => Some("#+end_verse\n"),
            BlockKind::Special(_) => None,
        }
    }
}

impl<'a> From<&'a str> for BlockKind<'a> {
    fn from(value: &'a str) -> Self {
        // DOES NOT HANDLE SRC!!
        // SRC holds its language in its value
        // REVIEW: this is janky
        match value.to_lowercase().as_str() {
            "center" => Self::Center,
            "quote" => Self::Quote,
            "comment" => Self::Comment,
            "example" => Self::Example,
            "export" => Self::Export,
            "verse" => Self::Verse,
            "src" => Self::Src,
            _ => Self::Special(value),
        }
    }
}

impl<'a> From<BlockKind<'a>> for &'a str {
    fn from(value: BlockKind<'a>) -> Self {
        match value {
            BlockKind::Center => "center",
            BlockKind::Quote => "quote",
            BlockKind::Special(val) => val,
            BlockKind::Comment => "comment",
            BlockKind::Example => "example",
            BlockKind::Export => "export",
            BlockKind::Src => "src",
            BlockKind::Verse => "verse",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn test_basic_block() {
        let inp = "#+begin_export\n#+end_export\n";

        dbg!(parse_org(inp));
    }
    #[test]
    fn test_special_block() {
        let inp = "#+begin_rainbow\n#+end_rainbow\n";

        dbg!(parse_org(inp));
    }
    #[test]
    fn test_src_block() {
        let inp = "#+begin_src python\n#+end_src\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn test_src_block_params() {
        let inp = "#+begin_src python yay i love python\n#+end_src\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn test_block_params() {
        let inp = "#+begin_example gotta love examples\n#+end_example\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn test_lesser_block_content() {
        let inp = "#+begin_example gotta love examples\nsmallexp\n#+end_example\n";

        dbg!(parse_org(inp));
    }

    #[test]
    #[rustfmt::skip]
    fn test_big_lesser_block_content() {
        let inp =
r"#+begin_example
this is a larger example gotta love examples
to demonstrate that it works
string substring
big
one two three
/formatted text? no such thing!/
*abc*
#+end_example
";
        dbg!(parse_org(inp));
    }

    #[test]
    fn test_big_greater_block_content() {
        let inp = r"
#+begin_quote

/formatted text? such thing!/
*abc*

* headlines too
anything is possible

blank lines

#+keyword: one

#+begin_src rust
let nest = Some()

if let Some(nested) = nest {
    dbg!(meta);
}

#+end_src

** headline :tag:meow:
#+end_quote
";
        let pool = parse_org(inp);
        pool.print_tree();
    }

    #[test]
    fn block_ending_proper() {
        let input = r"

text before
#+begin_src python

here is some text
#+end_src

here is after

";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn lesser_block_indented() {
        let input = r"
             #+begin_example
             we are eating so good?
             #+end_example
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn greater_block_indented() {
        let input = r"
             #+begin_swag
             we are eating so good?
             #+end_swag
";

        let pool = parse_org(input);
        pool.print_tree();
    }
}
