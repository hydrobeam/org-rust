use crate::types::{Match, MatchError, Node, Result};
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
pub fn bytes_to_str<'a>(byte_arr: &'a [u8]) -> &'a str {
    unsafe { std::str::from_utf8_unchecked(&byte_arr) }
}

// SAFETY: byte_arr came from a valid utf_8 string and we check only on valid
// utf8 segments. so the resulting string is valid utf8
// use unsafe to skip the utf8 check since we'd just be unwrap()ing anyways
//
// realistically not a big performance hit but no reason to pay the cost
// unecessarily
pub fn fn_until<'a>(
    byte_arr: &'a [u8],
    index: usize,
    func: impl Fn(u8) -> bool,
) -> Result<Match<&'a str>> {
    // TODO: don't unwrap
    // arr [1, 2, 3]
    // arr.position(|x| x == 2) => 1
    let ret = byte_arr[index..]
        .iter()
        .position(|x| func(*x))
        .ok_or(MatchError::EofError)?;

    Ok(Match {
        obj: bytes_to_str(&byte_arr[index..=ret]),
        start: index,
        end: ret + 1,
    })
}

pub fn word<'a, 'word>(
    byte_arr: &'a [u8],
    index: usize,
    word: &'word str,
) -> Result<Match<&'a str>> {
    if byte_arr[index..].starts_with(word.as_bytes()) {
        let start = index;
        let end = index + word.len();
        Ok(Match {
            obj: bytes_to_str(&byte_arr[start..end]),
            start,
            end,
        })
    } else {
        Err(MatchError::InvalidLogic)
    }
}

#[inline(always)]
// pub fn variant_eq(a: Rc<RefCell<Match<Node>>>, b: &Node) -> bool {
pub fn variant_eq<'t>(a: &Node<'t>, b: &Node<'t>) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

// pub fn both_plain<'t>(a: &Node<'t>, b: &Node<'t>) -> bool {
//     variant_eq(a, &Node::Plain("_")) && variant_eq(b, &Node::Plain("_"))
// }

pub fn verify_markup(byte_arr: &[u8], index: usize, post: bool) -> bool {
    // REVIEW: consider  {pre/ap}pending bof/eof to not need to check
    // and to handle improperly terminated files
    // concern: this might require a copy?

    // can fail, file can start with markup
    let before = byte_arr.get(index - 1);
    // can't fail, file must end in a newline
    let after = byte_arr[index + 1];

    if post {
        // if we're in post, then a character before the markup Must Exist
        MARKUP_POST.contains(&after) && !before.unwrap().is_ascii_whitespace()
    } else {
        if let Some(val) = before {
            MARKUP_PRE.contains(val) && !after.is_ascii_whitespace()
        } else {
            // bof is always valid
            !after.is_ascii_whitespace()
        }
    }
}
