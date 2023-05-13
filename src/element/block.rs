use crate::constants::NEWLINE;
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, skip_ws, word};
use memchr::memmem;

#[derive(Debug, Clone)]
pub struct Block<'a> {
    kind: BlockKind<'a>,
    parameters: Option<&'a str>,
    contents: BlockContents<'a>,
}

#[derive(Debug, Clone)]
enum BlockContents<'a> {
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
        if begin_cookie.end != block_name_match.end {
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
            // skip the newline
            curr_ind = params_match.end + 1;
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
            alloc_str = format!("#+end_{}\n", block_name_match.to_str(byte_arr));
            needle = &alloc_str;
        }

        it = memmem::find_iter(&byte_arr[curr_ind..], needle.as_bytes());
        let loc = it.next().ok_or(MatchError::InvalidLogic)?;

        if block_kind.is_lesser() {
            Ok(pool.alloc(
                Self {
                    kind: block_kind,
                    parameters,
                    contents: BlockContents::Lesser(bytes_to_str(
                        &byte_arr[curr_ind..(curr_ind + loc)],
                    )),
                },
                index,
                curr_ind + needle.len(),
                parent,
            ))
        } else {
            let mut content_vec: Vec<NodeID> = Vec::new();
            while let Ok(element_id) =
                parse_element(pool, &byte_arr[..loc], curr_ind, parent, parse_opts)
            {
                content_vec.push(element_id);
                curr_ind = pool[element_id].end;
            }

            Ok(pool.alloc(
                Self {
                    kind: block_kind,
                    parameters,
                    contents: BlockContents::Greater(content_vec),
                },
                index,
                curr_ind + needle.len(),
                parent,
            ))
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BlockKind<'a> {
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
    fn is_lesser(&self) -> bool {
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
            BlockKind::Center => Some("#+end_center\n"),
            BlockKind::Quote => Some("#+end_quote\n"),
            BlockKind::Comment => Some("#+end_comment\n"),
            BlockKind::Example => Some("#+end_example\n"),
            BlockKind::Export => Some("#+end_export\n"),
            BlockKind::Src(_) => Some("#+end_src\n"),
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
            "src" => unreachable!(),
            _ => Self::Special(value),
        }
    }
}
