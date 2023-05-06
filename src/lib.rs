#![allow(dead_code)]
#![allow(unused_variables)]

use types::{Node, ParseOpts};

use crate::{
    parse::parse_element,
    types::{Branch, Leaf},
};

mod element;
mod object;
mod parse;
mod types;
mod utils;

#[rustfmt::skip]
pub mod constants {
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

pub fn parse_org(input_text: &str) -> Node {
    let byte_arr = input_text.as_bytes();
    let index = 0;
    let parse_opts = ParseOpts::default();

    let mut content_vec: Vec<Node> = Vec::new();
    let mut idx = index;

    loop {
        match parse_element(byte_arr, idx, parse_opts) {
            Ok(inner) => {
                idx = inner.get_end();
                content_vec.push(inner);
                // break;
            }
            Err(_) => break,
        }
    }

    Node::make_branch(content_vec, index, idx)
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

    // #[test]
    // fn it_works() {
    //     let ream: Regex = Regex::new(r"#\+begin_src\n(?P<source>.*)\n#\+end_src").unwrap();
    //     dbg!(ream
    //         .captures_iter(
    //             "#+begin_src\nmeowow\n#+end_src\n#+begin_src\n notmatchplease\n#+end_src"
    //         )
    //         .collect::<Vec<_>>());
    // }
}
