// #![allow(dead_code)]
#![allow(unused_variables)]

pub mod element;
pub mod object;

pub(crate) mod node_pool;
pub(crate) mod types;
pub(crate) mod utils;

mod parse;

pub use node_pool::{NodeID, NodePool};
pub use types::{Expr, Node, Parser};
pub use utils::Match;

use std::collections::HashMap;

use parse::{parse_element, parse_object};
use types::{Cursor, NodeCache, ParseOpts};

#[rustfmt::skip]
pub(crate) mod constants {
    pub const SLASH       : u8 = b'/';
    pub const STAR        : u8 = b'*';
    pub const POUND       : u8 = b'#';
    pub const PLUS        : u8 = b'+';
    pub const HYPHEN      : u8 = b'-';
    pub const UNDERSCORE  : u8 = b'_';
    pub const LBRACK      : u8 = b'[';
    pub const RBRACK      : u8 = b']';
    pub const LBRACE      : u8 = b'{';
    pub const RBRACE      : u8 = b'}';
    pub const COLON       : u8 = b':';
    pub const SPACE       : u8 = b' ';
    pub const VBAR        : u8 = b'|';
    pub const BACKSLASH   : u8 = b'\\';
    pub const CARET       : u8 = b'^';
    pub const DOLLAR      : u8 = b'$';
    pub const TILDE       : u8 = b'~';
    pub const EQUAL       : u8 = b'=';
    pub const LANGLE      : u8 = b'<';
    pub const RANGLE      : u8 = b'>';
    pub const PERIOD      : u8 = b'.';
    pub const COMMA       : u8 = b',';
    pub const NEWLINE     : u8 = b'\n';
    pub const LPAREN      : u8 = b'(';
    pub const RPAREN      : u8 = b')';
}

/// The main entry point to the parser.
///
/// Repeatedly parses elements until EOF, then returns a [`Parser`].
pub fn parse_org(input: &str) -> Parser<'_> {
    let mut cursor = Cursor::new(input.as_bytes());
    let parse_opts = ParseOpts::default();
    let mut pool = NodePool::new();
    let parent = pool.reserve_id();
    let mut content_vec: Vec<NodeID> = Vec::new();

    let cache = NodeCache::new();
    let mut parser = Parser {
        pool,
        cache,
        targets: HashMap::new(),
        macros: HashMap::new(),
        keywords: HashMap::new(),
        target_occurences: HashMap::new(),
        footnotes: HashMap::new(),
        source: input,
    };
    // main loop
    while let Ok(id) = parse_element(&mut parser, cursor, Some(parent), parse_opts) {
        content_vec.push(id);
        cursor.move_to(parser.pool[id].end);
    }
    parser.alloc_with_id(Expr::Root(content_vec), 0, cursor.index, None, parent);

    parser
}

/// An alternative entry point to the parser for parsing macros.
///
/// Unlike [`parse_org`], this function parses objects, not elements.
pub fn parse_macro_call<'a>(input: &'a str) -> Parser<'a> {
    let mut cursor = Cursor::new(input.as_bytes());
    let parse_opts = ParseOpts::default();
    let mut pool = NodePool::new();
    let parent = pool.reserve_id();
    let mut content_vec: Vec<NodeID> = Vec::new();

    // FIXME: pass in keywords + macros
    let mut parser = Parser {
        pool,
        cache: NodeCache::new(),
        targets: HashMap::new(),
        macros: HashMap::new(),
        keywords: HashMap::new(),
        target_occurences: HashMap::new(),
        footnotes: HashMap::new(),
        source: input,
    };
    while let Ok(id) = parse_object(&mut parser, cursor, Some(parent), parse_opts) {
        content_vec.push(id);
        cursor.move_to(parser.pool[id].end);
    }
    parser.alloc_with_id(Expr::Root(content_vec), 0, cursor.index, None, parent);

    parser
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_paragraph() {
        let inp = "hello_world\n";

        dbg!(parse_org(inp));
    }

    #[test]
    /// Tests whether we handle unexpected eof
    fn test_basic_paragraph_no_nl() {
        let inp = "hello_world";

        dbg!(parse_org(inp));
    }

    #[test]
    fn test_basic_paragraph_newline() {
        let inp = "hello_world\nsame_paragraph\n";

        dbg!(parse_org(inp));
    }

    #[test]
    fn test_basic_markup() {
        let inp = "hello /italic/ more text after\n";

        let pool = parse_org(inp);
        pool.print_tree();
    }

    #[test]
    fn test_newline_in_italic_markup() {
        let inp = "hello /italic \n newline/ more text after\n";

        let pool = parse_org(inp);
        pool.print_tree();
    }

    #[test]
    fn test_newline_in_verbatim() {
        let inp = "hello =italic \n newline= more text after\n";

        // dbg!(parse_org(inp));
        println!("{:?}", parse_org(inp));
    }

    #[test]
    fn lots() {
        let input = r#"
#+macro: greet Hello $1, nice typing... $1.
* Basic Heading

{{{greet(user)}}}

** Child Heading

- https://plain_links.com.
  - <mailto:okiedokie@cool.com>
    - src_python{(technically) inline_src}
- [[Child Heading]]
  - \aleph \leftarrow entities

#+begin_export
<style type="text/css" media="screen">
table, th, td {
  border: 1px solid;
}
</style>
#+end_export

|tables!|[[targets][links to output target]]|styled,, manually :sweat_smile:
|no|default css (yet)|
|||||||||table

1. +does+
2. *it*
3. /all/
4. ~code~
5. =code, again..=
6. /so _nested_, *t^o_o*./

emojis :flushed: :tada: :sunglasses:

\begin{align}
x &+ 4\\
abc &+ 10\\
\end{align}
output MathML, little janky atm (might switch to katex..?)

Target here: <<targets>>\\


# doesn't look the best, imo
-----

#+begin_src rust
nothing styled for source blocks yet, too.
#+end_src

"#;
        let pool = parse_org(input);
        pool.print_tree();
        dbg!(pool);
    }

    #[test]
    fn correctness_cache() {
        let input = r"
- one
- two

--------------
";
        let pool = parse_org(input);
        // dbg!(&pool);
        pool.print_tree();
    }

    #[test]
    fn basic_unicode() {
        let input = r"é
";

        let pool = parse_org(input);
        pool.print_tree();
    }
}
