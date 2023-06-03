#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::BTreeMap;

use node_pool::{NodeID, NodePool};
use types::{Cursor, Expr, NodeCache, ParseOpts, Parser};

use crate::parse::parse_element;

pub mod element;
pub mod node_pool;
pub mod object;
mod parse;
pub mod types;
pub(crate) mod utils;

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
    pub const X           : u8 = b'X';
    pub const SPACE       : u8 = b' ';
    pub const TAB         : u8 = b'\t';
    pub const AT          : u8 = b'@';
    pub const VBAR        : u8 = b'|';
    pub const PERCENT     : u8 = b'%';
    pub const BACKSLASH   : u8 = b'\\';
    pub const CARET       : u8 = b'^';
    pub const DOLLAR      : u8 = b'$';
    pub const TILDE       : u8 = b'~';
    pub const EQUAL       : u8 = b'=';
    pub const LANGLE      : u8 = b'<';
    pub const RANGLE      : u8 = b'>';
    pub const PERIOD      : u8 = b'.';
    pub const COMMA       : u8 = b',';
    pub const SEMICOLON   : u8 = b';';
    pub const EXCLAMATION : u8 = b'!';
    pub const QUESTION    : u8 = b'?';
    pub const DOUBLEQUOTE : u8 = b'"';
    pub const NEWLINE     : u8 = b'\n';
    pub const LPAREN      : u8 = b'(';
    pub const RPAREN      : u8 = b')';
}

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
        targets: BTreeMap::new(),
    };
    while let Ok(id) = parse_element(&mut parser, cursor, Some(parent), parse_opts) {
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
}
