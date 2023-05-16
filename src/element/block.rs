use crate::constants::NEWLINE;
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, skip_ws, word};
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
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<crate::node_pool::NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let begin_cookie = word(byte_arr, index, "#+begin_")?;

        let block_name_match = fn_until(byte_arr, begin_cookie.end, |chr: u8| {
            chr.is_ascii_whitespace()
        })?;

        let block_kind: BlockKind;
        let parameters: Option<&str>;
        // if no progress was made looking for the block_type:
        // i.e.: #+begin_\n
        if begin_cookie.end == block_name_match.end {
            return Err(MatchError::InvalidLogic);
        }
        // parse paramters
        let mut curr_ind = skip_ws(byte_arr, block_name_match.end);

        if block_name_match.to_str(byte_arr) == "src" {
            // skip_ws skipped to the end of the line:
            // i.e. there is no language
            if curr_ind == block_name_match.end {
                return Err(MatchError::InvalidLogic);
            }
            let lang = fn_until(byte_arr, curr_ind, |chr| chr.is_ascii_whitespace())?;

            block_kind = BlockKind::Src(lang.to_str(byte_arr));
            curr_ind = skip_ws(byte_arr, lang.end);
        } else {
            block_kind = block_name_match.to_str(byte_arr).into();
        }

        if byte_arr[curr_ind] == NEWLINE {
            parameters = None;
        } else {
            let params_match = fn_until(byte_arr, curr_ind, |chr| chr == NEWLINE)?;
            parameters = Some(params_match.to_str(byte_arr));
            curr_ind = params_match.end;
        }

        let mut it;
        // have to predeclare these so that the allocated string
        // doesn't go out of scope and we can still pull a reference
        // to it.
        let alloc_str;
        let needle;

        // avoid an allocation for pre-known endings
        if let Some(block_end) = block_kind.to_end() {
            needle = block_end;
        } else {
            alloc_str = format!("\n#+end_{}\n", block_name_match.to_str(byte_arr));
            needle = &alloc_str;
        }

        it = memmem::find_iter(&byte_arr[curr_ind..], needle.as_bytes());
        // returns result at the start of the needle
        let loc = it.next().ok_or(MatchError::InvalidLogic)? + curr_ind;

        if block_kind.is_lesser() {
            Ok(pool.alloc(
                Self {
                    kind: block_kind,
                    parameters,
                    contents: BlockContents::Lesser(bytes_to_str(&byte_arr[curr_ind..loc])),
                },
                index,
                curr_ind + needle.len(),
                parent,
            ))
        } else {
            let mut content_vec: Vec<NodeID> = Vec::new();
            let reserve_id = pool.reserve_id();
            while let Ok(element_id) = parse_element(
                pool,
                &byte_arr[..loc],
                curr_ind,
                Some(reserve_id),
                parse_opts,
            ) {
                content_vec.push(element_id);
                curr_ind = pool[element_id].end;
            }

            Ok(pool.alloc_with_id(
                Self {
                    kind: block_kind,
                    parameters,
                    contents: BlockContents::Greater(content_vec),
                },
                index,
                curr_ind + needle.len(),
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
    Src(&'a str), // holds the language
    Verse,
}

impl<'a> BlockKind<'_> {
    pub fn is_lesser(&self) -> bool {
        matches!(
            self,
            BlockKind::Comment
                | BlockKind::Example
                | BlockKind::Export
                | BlockKind::Src(_)
                | BlockKind::Verse
        )
    }

    fn to_end(&self) -> Option<&str> {
        match self {
            BlockKind::Center => Some("\n#+end_center\n"),
            BlockKind::Quote => Some("\n#+end_quote\n"),
            BlockKind::Comment => Some("\n#+end_comment\n"),
            BlockKind::Example => Some("\n#+end_example\n"),
            BlockKind::Export => Some("\n#+end_export\n"),
            BlockKind::Src(_) => Some("\n#+end_src\n"),
            BlockKind::Verse => Some("\n#+end_verse\n"),
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
            "src" => unreachable!(),
            _ => Self::Special(value),
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
        let ret = parse_org(inp);
        ret.root().print_tree(&ret);
    }
}
