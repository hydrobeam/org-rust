#![feature(rustc_private)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::cell::RefCell;

use node_pool::{NodeID, NodePool};
use types::{Expr, Node, ParseOpts};


use crate::{parse::parse_element, utils::bytes_to_str};

mod element;
mod node_pool;
mod object;
mod parse;
mod types;
mod utils;

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
}

pub fn parse_org<'a>(input_text: &'a str) -> NodePool<'a> {
    let byte_arr = input_text.as_bytes();
    let index = 0;
    let parse_opts = ParseOpts::default();

    let mut content_vec: Vec<NodeID> = Vec::new();
    let mut idx = index;

    let mut pool = NodePool::new();
    let parent = pool
        .alloc(Expr::Root(Vec::new()), index, idx, None);


    // NOTE: while let loop does not run! TODO: find out why
    loop {
        // dbg!(idx);
        // dbg!(bytes_to_str(&byte_arr[idx..]));
        match parse_element(&mut pool, byte_arr, idx, Some(parent), parse_opts) {

            Ok(id) => {
                // dbg!(id);
                // dbg!(&pool.borrow()[id]);
                idx = pool[id].end;
                content_vec.push(id);
                // break;
            }
            Err(_) => break,
        }
    }

    pool[parent].obj = Expr::Root(content_vec);
    pool[parent].end = idx;
    pool
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

        // dbg!(parse_org(inp));
        println!("{:?}", parse_org(inp));
    }

    #[test]
    fn test_newline_in_italic() {
        let inp = "hello /italic \n newline/ more text after\n";

        // dbg!(parse_org(inp));
        println!("{:?}", parse_org(inp));
    }

    #[test]
    fn test_newline_in_verbatim() {
        let inp = "hello =italic \n newline= more text after\n";

        // dbg!(parse_org(inp));
        println!("{:?}", parse_org(inp));
    }
}
