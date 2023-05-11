use crate::constants::{HYPHEN, PLUS, STAR};
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
}

// SAFETY: byte_arr came from a valid utf_8 string and we check only on valid
// utf8 segments. so the resulting string is valid utf8
// use unsafe to skip the utf8 check since we'd just be unwrap()ing anyways
//
// realistically not a big performance hit but no reason to pay the cost
// unecessarily
pub(crate) fn fn_until(byte_arr: &[u8], index: usize, func: impl Fn(u8) -> bool) -> Result<Match> {
    // TODO: don't unwrap
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

#[inline(always)]
// pub fn variant_eq(a: Rc<RefCell<Match<Node>>>, b: &Node) -> bool {
pub fn variant_eq<T>(a: &T, b: &T) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub fn verify_markup(byte_arr: &[u8], index: usize, post: bool) -> bool {
    // REVIEW: consider  {pre/ap}pending bof/eof to not need to check
    // and to handle improperly terminated files
    // concern: this might require a copy?

    let before = if index == 0 {
        None
    } else {
        Some(byte_arr[index - 1])
    };

    let after = if index + 1 >= byte_arr.len() {
        None
    } else {
        Some(byte_arr[index + 1])
    };

    if post {
        // if we're in post, then a character before the markup Must Exist
        if let Some(ref val) = after {
            MARKUP_POST.contains(val) && !before.unwrap().is_ascii_whitespace()
        } else {
            !before.unwrap().is_ascii_whitespace()
        }
    } else if let Some(ref val) = before {
        MARKUP_PRE.contains(val) && !after.unwrap().is_ascii_whitespace()
    } else {
        // bof is always valid
        !after.unwrap().is_ascii_whitespace()
    }
}

pub(crate) fn is_list_start(byte: u8) -> bool {
    byte == HYPHEN || byte == STAR || byte == PLUS || byte.is_ascii_digit()
}
