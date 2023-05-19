use crate::constants::{DOLLAR, HYPHEN, PLUS, SPACE, STAR};
use crate::types::{Cursor, MatchError, Result};
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
 b'\n'
};

/// ## SAFETY:
/// We are given a valid utf8 string to parse with, no need for re-validation
/// with `str::from_utf8`()
///
/// Not measured to see if this is a significant performance hit, but
/// it's a safe assumption to make that we're indexing into valid utf8,
/// otherwise we have an internal bug and we'd be unwrapping immediately
/// afterwards with the safe alternative either way.
#[inline]
pub fn bytes_to_str(byte_arr: &[u8]) -> &str {
    unsafe { std::str::from_utf8_unchecked(byte_arr) }
}

#[derive(Debug)]
pub(crate) struct Match<T> {
    pub start: usize,
    pub end: usize,
    pub obj: T,
}

impl<'a, T> Match<T> {
    #[inline]
    pub fn to_str(&self, byte_arr: &'a [u8]) -> &'a str {
        bytes_to_str(&byte_arr[self.start..self.end])
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

// pub fn variant_eq(a: Rc<RefCell<Match<Node>>>, b: &Node) -> bool {
pub(crate) fn variant_eq<T>(a: &T, b: &T) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub(crate) fn verify_markup(cursor: Cursor, post: bool) -> bool {
    // handle access this way in case of underflow
    let before = cursor.index.checked_sub(1).and_then(|num| cursor.get(num));

    // pretty much never going to overflow
    let after_maybe = cursor.get(cursor.index + 1);

    if post {
        // if we're in post, then a character before the markup Must Exist
        !before.unwrap().is_ascii_whitespace()
            && if let Some(val) = after_maybe {
                MARKUP_POST.contains(val)
            } else {
                true
            }
    } else if let Some(after) = after_maybe {
        !after.is_ascii_whitespace()
            && if let Some(val) = before {
                MARKUP_PRE.contains(val)
            } else {
                // bof is always valid
                true
            }
    } else {
        // if there's no after, cannot be valid markup
        false
    }
}

pub(crate) fn verify_latex_frag(cursor: Cursor, post: bool) -> bool {
    // handle access this way in case of underflow
    let before = cursor.index.checked_sub(1).and_then(|num| cursor.get(num));
    // pretty much never going to overflow
    let after_maybe = cursor.get(cursor.index + 1);

    if post {
        // if we're in post, then a character before the markup Must Exist
        (!before.unwrap().is_ascii_whitespace() && !matches!(before.unwrap(), b'.' | b',' | b'$'))
            && if let Some(after) = after_maybe {
                after.is_ascii_punctuation() || after.is_ascii_whitespace()
            } else {
                // no after => valid
                true
            }
    } else if let Some(after) = after_maybe {
        !after.is_ascii_whitespace()
            && !matches!(after, b'.' | b',' | b';' | b'$')
            && if let Some(val) = before {
                *val != DOLLAR
            } else {
                // bof is valid
                true
            }
    } else {
        // if there's no after, cannot be valid markup
        false
    }
}

pub(crate) fn verify_single_char_latex_frag(cursor: Cursor) -> bool {
    // distances:
    // 10123
    // p$i$c
    //
    // we are at the dollar

    // handle access this way in case of underflow
    let pre = cursor.index.checked_sub(1).and_then(|num| cursor.get(num));
    // pretty much never going to overflow
    let post = cursor.get(cursor.index + 3);

    let Some(inner) = cursor.get(cursor.index + 1) else {
        return false;
    };

    !(inner.is_ascii_whitespace() || matches!(inner, b'.' | b',' | b'?' | b';' | b'"'))
        // both could be dne
        && if let Some(after) = post {
            after.is_ascii_punctuation() || after.is_ascii_whitespace()
        } else {
            true
        }
        && if let Some(before) = pre {
            *before != DOLLAR
        } else {
            true
        }
}
