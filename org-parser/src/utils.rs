use crate::constants::DOLLAR;
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
pub(crate) fn bytes_to_str(byte_arr: &[u8]) -> &str {
    unsafe { std::str::from_utf8_unchecked(byte_arr) }
}

#[derive(Debug, Clone)]
pub struct Match<T> {
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
    let before = cursor.peek_rev(1);

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
            && if let Ok(val) = before {
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

pub(crate) fn verify_latex_frag(cursor: Cursor, post: bool) -> bool {
    let before = cursor.peek_rev(1);
    let after_maybe = cursor.peek(1);

    if post {
        let before_val = before.unwrap();
        // if we're in post, then a character before the markup Must Exist
        (!before_val.is_ascii_whitespace() && !matches!(before_val, b'.' | b',' | b'$'))
            && if let Ok(after) = after_maybe {
                after.is_ascii_punctuation() || after.is_ascii_whitespace()
            } else {
                // no after => valid
                true
            }
    } else if let Ok(after) = after_maybe {
        !after.is_ascii_whitespace()
            && !matches!(after, b'.' | b',' | b';' | b'$')
            && if let Ok(val) = before {
                val != DOLLAR
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
    let pre = cursor.peek_rev(1);
    // pretty much never going to overflow
    let post = cursor.peek(3);

    let Ok(inner) = cursor.peek( 1) else {
        return false;
    };

    !(inner.is_ascii_whitespace() || matches!(inner, b'.' | b',' | b'?' | b';' | b'"'))
        // both could be dne
        && if let Ok(after) = post {
            after.is_ascii_punctuation() || after.is_ascii_whitespace()
        } else {
            true
        }
        && if let Ok(before) = pre {
            before != DOLLAR
        } else {
            true
        }
}
