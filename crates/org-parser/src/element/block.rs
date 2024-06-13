use std::collections::HashMap;

use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{process_attrs, Cursor, MatchError, ParseOpts, Parseable, Parser, Result};
use lazy_static::lazy_static;
use regex::bytes::Regex;

// regexes that search for various ending tokens on a line that only contains whitespace
#[rustfmt::skip]
lazy_static! {
  static ref CENTER_RE  : Regex = Regex::new(r"(?mi)^[ \t]*#\+end_center[\t ]*$") .unwrap();
  static ref QUOTE_RE   : Regex = Regex::new(r"(?mi)^[ \t]*#\+end_quote[\t ]*$")  .unwrap();
  static ref COMMENT_RE : Regex = Regex::new(r"(?mi)^[ \t]*#\+end_comment[\t ]*$").unwrap();
  static ref EXAMPLE_RE : Regex = Regex::new(r"(?mi)^[ \t]*#\+end_example[\t ]*$").unwrap();
  static ref EXPORT_RE  : Regex = Regex::new(r"(?mi)^[ \t]*#\+end_export[\t ]*$") .unwrap();
  static ref SRC_RE     : Regex = Regex::new(r"(?mi)^[ \t]*#\+end_src[\t ]*$")    .unwrap();
  static ref VERSE_RE   : Regex = Regex::new(r"(?mi)^[ \t]*#\+end_verse[\t ]*$")  .unwrap();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block<'a> {
    // Greater Blocks
    Center {
        parameters: HashMap<&'a str, &'a str>,
        contents: Vec<NodeID>,
    },
    Quote {
        parameters: HashMap<&'a str, &'a str>,
        contents: Vec<NodeID>,
    },
    Special {
        parameters: HashMap<&'a str, &'a str>,
        contents: Vec<NodeID>,
        name: &'a str,
    },

    // Lesser Blocks
    Comment {
        parameters: HashMap<&'a str, &'a str>,
        contents: &'a str,
    },
    Example {
        parameters: HashMap<&'a str, &'a str>,
        contents: &'a str,
    },
    Export {
        backend: Option<&'a str>,
        parameters: HashMap<&'a str, &'a str>,
        contents: &'a str,
    },
    Src {
        language: Option<&'a str>,
        parameters: HashMap<&'a str, &'a str>,
        contents: &'a str,
    },
    Verse {
        parameters: HashMap<&'a str, &'a str>,
        contents: &'a str,
    },
}

impl<'a> Parseable<'a> for Block<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<crate::node_pool::NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;
        cursor
            .word("#+begin_")
            .or_else(|_| cursor.word("#+BEGIN_"))?;

        let block_name_match = cursor.fn_until(|chr: u8| chr.is_ascii_whitespace())?;

        // if no progress was made looking for the block_type:
        // i.e.: #+begin_\n
        if cursor.index == block_name_match.end {
            return Err(MatchError::InvalidLogic);
        }
        cursor.index = block_name_match.end;
        // parse paramters
        cursor.skip_ws();

        let block_kind: BlockKind = block_name_match.obj.into();

        let mut language: Option<&str> = None;
        let mut backend: Option<&str> = None;
        match block_kind {
            // TODO: reduce duplication here
            BlockKind::Src => {
                let lang_match = cursor.fn_until(|chr| chr.is_ascii_whitespace())?;
                let trimmed = lang_match.obj.trim();

                if trimmed.is_empty() {
                    language = None;
                } else {
                    language = Some(trimmed);
                }
                cursor.skip_ws();
            }
            BlockKind::Export => {
                let backend_match = cursor.fn_until(|chr| chr.is_ascii_whitespace())?;
                let trimmed = backend_match.obj.trim();

                if trimmed.is_empty() {
                    backend = None;
                } else {
                    backend = Some(trimmed);
                }
                cursor.skip_ws();
            }
            _ => (),
        }
        // TODO: src switches
        let (mut cursor, parameters) = process_attrs(cursor)?;
        // skip newline
        cursor.next();

        // have to predeclare these so that the allocated regex
        // doesn't go out of scope and we can still pull a reference
        // to it.
        let alloc_reg;

        // avoid an allocation for pre-known endings
        let re = if let Some(block_end) = block_kind.to_end() {
            block_end
        } else {
            alloc_reg = Regex::new(&format!(
                r"(?mi)^[ \t]*#\+end_{}[\t ]*$",
                block_name_match.obj
            ))
            .unwrap();
            &alloc_reg
        };

        // Find ending cookie: #+end_{}
        // lesser blocks: clamp a string between the beginning and end
        // greater blocks: parse between the bounds
        // let re = regex::bytes::Regex::new(needle).unwrap();
        let ret = if let Some(val) = re.find(cursor.rest()) {
            val
        } else {
            Err(MatchError::InvalidLogic)?
        };

        let loc = ret.start() + cursor.index;
        let end = ret.end() + cursor.index;

        // handle empty contents
        // if cursor.index > loc {
        //     cursor.index = loc;
        // }

        if block_kind.is_lesser() {
            let contents = cursor.clamp_forwards(loc);
            Ok(parser.alloc(
                match block_kind {
                    BlockKind::Center | BlockKind::Quote | BlockKind::Special(_) => unreachable!(),
                    BlockKind::Comment => Block::Comment {
                        parameters,
                        contents,
                    },
                    BlockKind::Example => Block::Example {
                        parameters,
                        contents,
                    },
                    BlockKind::Export => Block::Export {
                        backend,
                        parameters,
                        contents,
                    },
                    BlockKind::Src => Block::Src {
                        language,
                        parameters,
                        contents,
                    },
                    BlockKind::Verse => Block::Verse {
                        parameters,
                        contents,
                    },
                },
                start,
                end,
                parent,
            ))
        } else {
            let mut contents: Vec<NodeID> = Vec::new();
            let reserve_id = parser.pool.reserve_id();
            // REVIEW: janky
            let mut temp_cursor = cursor.cut_off(loc);
            while let Ok(element_id) =
                // use default parseopts since it wouldn't make sense for the contents
                // of the block to be interpreted as a list, or be influenced from the outside
                parse_element(parser, temp_cursor, Some(reserve_id), ParseOpts::default())
            {
                contents.push(element_id);
                temp_cursor.index = parser.pool[element_id].end;
            }

            Ok(parser.alloc_with_id(
                match block_kind {
                    BlockKind::Center => Block::Center {
                        parameters,
                        contents,
                    },
                    BlockKind::Quote => Block::Quote {
                        parameters,
                        contents,
                    },
                    BlockKind::Special(name) => Block::Special {
                        parameters,
                        contents,
                        name,
                    },
                    BlockKind::Comment
                    | BlockKind::Example
                    | BlockKind::Export
                    | BlockKind::Src
                    | BlockKind::Verse => unreachable!(),
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
enum BlockKind<'a> {
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

    #[rustfmt::skip]
    fn to_end(self) -> Option<&'static Regex> {
        match self {
            BlockKind::Center  => Some(&CENTER_RE ) ,
            BlockKind::Quote   => Some(&QUOTE_RE  ) ,
            BlockKind::Comment => Some(&COMMENT_RE) ,
            BlockKind::Example => Some(&EXAMPLE_RE) ,
            BlockKind::Export  => Some(&EXPORT_RE ) ,
            BlockKind::Src     => Some(&SRC_RE    ) ,
            BlockKind::Verse   => Some(&VERSE_RE  ) ,
            BlockKind::Special(_) => None,
        }
    }
}

impl<'a> From<&'a str> for BlockKind<'a> {
    fn from(value: &'a str) -> Self {
        match value.to_ascii_lowercase().as_str() {
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
    use std::collections::HashMap;

    use crate::element::Block;
    use crate::types::Expr;
    use crate::{expr_in_pool, parse_org};

    use pretty_assertions::assert_eq;

    #[test]
    fn test_basic_block() {
        let input = "#+begin_export\n#+end_export\n";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Export {
                backend: None,
                parameters: HashMap::new(),
                contents: r""
            }
        )
    }
    #[test]
    fn test_special_block() {
        let input = "#+begin_rainbow\n#+end_rainbow\n";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Special {
                parameters: HashMap::new(),
                contents: Vec::new(),
                name: "rainbow"
            }
        )
    }
    #[test]
    fn test_src_block() {
        let input = "#+begin_src python\n#+end_src\n";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Src {
                language: Some("python"),
                parameters: HashMap::new(),
                contents: ""
            }
        )
    }

    #[test]
    fn test_block_params() {
        let input = "#+begin_example :gotta :love :examples\n#+end_example\n";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Example {
                parameters: HashMap::from([("gotta", ""), ("love", ""), ("examples", "")]),
                contents: ""
            }
        )
    }

    #[test]
    fn test_lesser_block_content() {
        let input = "#+begin_example gotta love examples\nsmallexp\n#+end_example\n";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Example {
                parameters: HashMap::new(),
                contents: "smallexp
"
            }
        )
    }

    #[test]
    fn test_big_lesser_block_content() {
        let input = r"#+begin_example
this is a larger example gotta love examples
to demonstrate that it works
string substring
big
one two three
/formatted text? no such thing!/
*abc*
#+end_example
";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Example {
                parameters: HashMap::new(),
                contents: r"this is a larger example gotta love examples
to demonstrate that it works
string substring
big
one two three
/formatted text? no such thing!/
*abc*
"
            }
        )
    }

    #[test]
    fn test_big_greater_block_content() {
        let input = r"
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
        let pool = parse_org(input);
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

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Src {
                language: Some("python"),
                parameters: HashMap::new(),
                contents: r"
here is some text
"
            }
        )
    }

    #[test]
    fn lesser_block_indented() {
        let input = r"
             #+begin_example
             we are eating so good?
             #+end_example
";

        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Example {
                parameters: HashMap::new(),
                contents: r"             we are eating so good?
"
            }
        )
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

    #[test]
    fn gblock_plus_list() {
        let input = r"
- a
   #+begin_quote
hiiiiiiiiiiiiiiiiiii
   #+end_quote
-
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn lblock_plus_list() {
        let input = r"
-
   #+begin_src


hiiiiiiiiiiiiiiiiiii

text
   #+end_src

-
";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            l,
            &Block::Src {
                language: None,
                parameters: HashMap::new(),
                contents: r"

hiiiiiiiiiiiiiiiiiii

text
"
            }
        )
    }

    #[test]
    fn caps() {
        let input = r"
#+BEGIN_VERSE
text
#+END_VERSE
";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            &Block::Verse {
                parameters: HashMap::new(),
                contents: r"text
"
            },
            l
        )
    }
    #[test]
    fn caps_space() {
        let input = r"
#+BEGIN_COMMENT
                                                text
                #+END_COMMENT
";
        let parsed = parse_org(input);
        let l = expr_in_pool!(parsed, Block).unwrap();

        assert_eq!(
            &Block::Comment {
                parameters: HashMap::new(),
                contents: r"                                                text
"
            },
            l
        )
    }
}
