use crate::constants::{DOLLAR, HYPHEN, PLUS, SPACE, STAR};
use crate::types::{MatchError, Result};
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
/// with str::from_utf8()
///
/// Not measured to see if this is a significant performance hit, but
/// it's a safe assumption to make that we're indexing into valid utf8,
/// otherwise we have an internal bug and we'd be unwrapping immediately
/// afterwards with the safe alternative either way.
#[inline]
pub fn bytes_to_str(byte_arr: &[u8]) -> &str {
    unsafe { std::str::from_utf8_unchecked(byte_arr) }
}

pub(crate) struct Match {
    pub start: usize,
    pub end: usize,
}

impl<'a> Match {
    #[inline]
    pub fn to_str(&self, byte_arr: &'a [u8]) -> &'a str {
        bytes_to_str(&byte_arr[self.start..self.end])
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

// SAFETY: byte_arr came from a valid utf_8 string and we check only on valid
// utf8 segments. so the resulting string is valid utf8
// use unsafe to skip the utf8 check since we'd just be unwrap()ing anyways
//
// realistically not a big performance hit but no reason to pay the cost
// unecessarily
//
//
//
/// Apply `func` until it returns true
///
/// # Example
///
/// use orgparse::utils::fn_until;
///
/// ```ignore
/// let ret = fn_until("qqqnnn".as_bytes(), 1, |chr: u8| chr != b'q');
/// assert_eq!(ret.start, 1);
/// assert_eq!(ret.end, 3);
/// ```
pub(crate) fn fn_until(byte_arr: &[u8], index: usize, func: impl Fn(u8) -> bool) -> Result<Match> {
    // arr [1, 2, 3]
    // arr.position(|x| x == 2) => 1
    let ret = byte_arr[index..]
        .iter()
        .position(|x| func(*x))
        .ok_or(MatchError::EofError)?
        + index;

    Ok(Match {
        start: index,
        end: ret,
    })
}

pub(crate) fn word(byte_arr: &[u8], index: usize, word: &str) -> Result<Match> {
    if byte_arr[index..].starts_with(word.as_bytes()) {
        let start = index;
        let end = index + word.len();
        Ok(Match { start, end })
    } else {
        Err(MatchError::InvalidLogic)
    }
}

// TODO: recognize tabs too maybe?
pub fn skip_ws(byte_arr: &[u8], index: usize) -> usize {
    let mut idx = index;
    while byte_arr[idx] == SPACE {
        idx += 1;
    }
    idx
}

#[inline(always)]
// pub fn variant_eq(a: Rc<RefCell<Match<Node>>>, b: &Node) -> bool {
pub fn variant_eq<T>(a: &T, b: &T) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub(crate) fn verify_markup(byte_arr: &[u8], index: usize, post: bool) -> bool {
    // handle access this way in case of underflow
    let before = index.checked_sub(1).and_then(|num| byte_arr.get(num));

    // pretty much never going to overflow
    let after_maybe = byte_arr.get(index + 1);

    if post {
        // if we're in post, then a character before the markup Must Exist
        !before.unwrap().is_ascii_whitespace()
            && if let Some(ref val) = after_maybe {
                MARKUP_POST.contains(val)
            } else {
                true
            }
    } else {
        if let Some(after) = after_maybe {
            !after.is_ascii_whitespace()
                && if let Some(ref val) = before {
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
}

pub(crate) fn verify_latex_frag(byte_arr: &[u8], index: usize, post: bool) -> bool {
    // handle access this way in case of underflow
    let before = index.checked_sub(1).and_then(|num| byte_arr.get(num));
    // pretty much never going to overflow
    let after_maybe = byte_arr.get(index + 1);

    if post {
        // if we're in post, then a character before the markup Must Exist
        (!before.unwrap().is_ascii_whitespace() && !matches!(before.unwrap(), b'.' | b',' | b'$'))
            && if let Some(ref after) = after_maybe {
                after.is_ascii_punctuation() || after.is_ascii_whitespace()
            } else {
                // no after => valid
                true
            }
    } else {
        if let Some(after) = after_maybe {
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
}

pub(crate) fn verify_single_char_latex_frag(byte_arr: &[u8], index: usize) -> bool {
    // distances:
    // 10123
    // p$i$c
    //
    // we are at the dollar

    // handle access this way in case of underflow
    let pre = index.checked_sub(1).and_then(|num| byte_arr.get(num));
    // pretty much never going to overflow
    let post = byte_arr.get(index + 3);

    let inner = if let Some(inside) = byte_arr.get(index + 1) {
        inside
    } else {
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

pub(crate) fn is_list_start(byte: u8) -> bool {
    byte == HYPHEN || byte == STAR || byte == PLUS || byte.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fn_until() {
        let ret = fn_until("qqqnnn".as_bytes(), 1, |chr: u8| chr != b'q').unwrap();
        assert_eq!(ret.start, 1);
        assert_eq!(ret.end, 3);
    }
}
