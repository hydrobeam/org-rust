use crate::types::Cursor;
use phf::phf_set;

// Either a whitespace character, -, ., ,, ;, :, !, ?, ', ), }, [, ", or the end of a line.
static MARKUP_POST: phf::Set<u8> = phf_set! {
 b'-',
 b'.',
 b',',
 b';',
 b':',
 b'!',
 b'?',
 b')',
 b'}',
 b'[',
 b'"',
 b'\'',
    // whitespace chars
 b'\n',
 b' ',
 b'\t',
 b'|',
 b']',
 b'/',
 b'*',
 b'_',
 b'+',
};

// Either a whitespace character, -, (, {, ', ", or the beginning of a line.
static MARKUP_PRE: phf::Set<u8> = phf_set! {
 b'-',
 b'(',
 b'{',
 b'\'',
 b'"',
 // whitespace character
 b' ',
 b'\t',
 // checks for beginning of line
 b'\n',
 // // Non Standard
 b'|',
 b'[',
 b'/',
 b'*',
 b'_',
 b'+',
 b':',
};

// Why add non-standard extenders?
// org mode syntax allows */abc/* to be defined as both bold and italic
// even though * and / are not in PRE/POST, this is because it clamps then
// parses the contents.
//
// my extensions to PRE/POST are more permissive than the spec since it allows
// [/abc/] to be interpreted as markup (the object doesn't have to belong to markup)
// another example is:
//
// /abc _*one*/
//
// this shouldn't contain a bold object, but with these changes it does. I find this behaviour
// to be fairly reasonable imo, and don't mind the more permissive markup syntax.
// if there are other unexpected interactions however then I'll have to find
// the ending delimeter and then parse the contents within (entails reading over the
// contained text twice, not ideal).

/// ## SAFETY:
/// We are given a valid utf8 string to parse with, no need for re-validation
/// with `str::from_utf8`()
///
/// Not measured to see if this is a significant performance hit, but
/// it's a safe assumption to make that we're indexing into valid utf8,
/// otherwise we have an internal bug and we'd be unwrapping immediately
/// afterwards with the safe alternative either way.
#[inline]
pub(crate) fn bytes_to_str(byte_arr: &[u8]) -> &str {
    unsafe { std::str::from_utf8_unchecked(byte_arr) }
}

/// The range of an arbitary item in the source text.
#[derive(Debug, Clone)]
pub struct Match<T> {
    pub start: usize,
    pub end: usize,
    pub obj: T,
}

impl<'a, T> Match<T> {
    #[inline]
    pub fn to_str(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

/// Compares variants of an enum for equality
pub(crate) fn variant_eq<T>(a: &T, b: &T) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub(crate) fn verify_markup(cursor: Cursor, post: bool) -> bool {
    let before_maybe = cursor.peek_rev(1);
    let after_maybe = cursor.peek(1);

    if post {
        // if we're in post, then a character before the markup Must Exist
        !before_maybe.unwrap().is_ascii_whitespace()
            && if let Ok(val) = after_maybe {
                MARKUP_POST.contains(&val)
            } else {
                true
            }
    } else if let Ok(after) = after_maybe {
        !after.is_ascii_whitespace()
            && if let Ok(val) = before_maybe {
                MARKUP_PRE.contains(&val)
            } else {
                // bof is always valid
                true
            }
    } else {
        // if there's no after, cannot be valid markup
        false
    }
}

pub(crate) fn id_escape(potential_id: &str) -> String {
    // minor over-allocation in some cases, but I expect most
    // id recepients to be light on the shenanigans
    let mut ret = String::with_capacity(potential_id.len());
    for chr in potential_id.chars() {
        if chr == ' ' {
            ret.push('-');
        } else if chr == '_' || chr == '-' {
            ret.push(chr);
        } else if chr.is_alphanumeric() {
            // unicode lowercases can span multiple characters
            for val in chr.to_lowercase() {
                ret.push(val);
            }
        }
    }
    ret
}

/// Shorthand for extracting a [`crate::Expr`] from a [`crate::Parser`].
///
/// # Example
///
/// ```rust
/// use org_rust_parser as org_parser;
///
/// use org_parser::{Expr, expr_in_pool, parse_org};
/// use org_parser::element::Heading;
///
/// let ret_parse = parse_org("* Hello world!\n");
/// let heading_expr: &Heading = expr_in_pool!(ret_parse, Heading).unwrap();
/// ```
#[macro_export]
macro_rules! expr_in_pool {
    ($parsed: ident, $name: ident) => {
        $parsed.pool.iter().find_map(|x| {
            if let Expr::$name(i) = &x.obj {
                Some(i)
            } else {
                None
            }
        })
    };
}

/// Shorthand for extracting a [`crate::Node`] from a [`crate::Parser`].
///
/// # Example
///
/// ```rust
/// use org_rust_parser as org_parser;
///
/// use org_parser::{Expr, node_in_pool, parse_org, Node};
///
/// let ret_parse = parse_org("* Hello world!\n");
/// let heading_expr: &Node = node_in_pool!(ret_parse, Heading).unwrap();
/// ```
#[macro_export]
macro_rules! node_in_pool {
    ($parsed: ident, $name: ident) => {
        $parsed.pool.iter().find_map(|x| {
            if let Expr::$name(i) = &x.obj {
                Some(x)
            } else {
                None
            }
        })
    };
}
