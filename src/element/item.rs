use crate::constants::{COLON, HYPHEN, LBRACK, NEWLINE, PERIOD, PLUS, RBRACK, RPAREN, SPACE, STAR};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{Expr, MatchError, ParseOpts, Parseable, Result};
use crate::utils::{bytes_to_str, fn_until, skip_ws, Match};

#[derive(Debug, Clone)]
pub struct Item<'a> {
    pub bullet: BulletKind,
    // An instance of the pattern [@COUNTER]
    pub counter_set: Option<CounterKind>,
    pub check_box: Option<CheckBox>,
    pub tag: Option<&'a str>,
    pub children: Vec<NodeID>,
}

impl<'a> Parseable<'a> for Item<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        // Will only ever really get called via Plainlist.

        let mut curr_ind = index;

        let bullet_match = BulletKind::parse(byte_arr, curr_ind)?;
        let bullet = bullet_match.obj;
        curr_ind = bullet_match.end;

        let counter_set: Option<CounterKind> =
            if let Ok(counter_match) = parse_counter_set(byte_arr, curr_ind) {
                curr_ind = counter_match.end;
                Some(counter_match.obj)
            } else {
                None
            };

        let check_box: Option<CheckBox> =
            if let Ok(check_box_match) = CheckBox::parse(byte_arr, curr_ind) {
                curr_ind = check_box_match.end;
                Some(check_box_match.obj)
            } else {
                None
            };

        let tag: Option<&str> = if let Ok(tag_match) = parse_tag(byte_arr, curr_ind) {
            curr_ind = tag_match.end;
            Some(tag_match.obj)
        } else {
            None
        };

        let reserve_id = pool.reserve_id();
        let mut children: Vec<NodeID> = Vec::new();
        let mut blank_obj: Option<NodeID> = None;

        // if the last element was a \n, that means we're starting on a new line
        // so we are Not on a list line.
        parse_opts.list_line = byte_arr[curr_ind - 1] != NEWLINE;

        while let Ok(element_id) =
            parse_element(pool, byte_arr, curr_ind, Some(reserve_id), parse_opts)
        {
            let pool_loc = &pool[element_id];
            match &pool_loc.obj {
                Expr::BlankLine => {
                    if blank_obj.is_some() {
                        break;
                    } else {
                        blank_obj = Some(element_id);
                    }
                }
                Expr::Item(_) => {
                    break;
                }
                _ => {
                    if let Some(blank_id) = blank_obj {
                        children.push(blank_id);
                    }
                    children.push(element_id);
                }
            }
            parse_opts.list_line = false;
            curr_ind = pool_loc.end;
        }

        Ok(pool.alloc_with_id(
            Self {
                bullet,
                counter_set,
                check_box,
                tag,
                children,
            },
            index,
            curr_ind,
            parent,
            reserve_id,
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BulletKind {
    Unordered,
    // Either the pattern COUNTER. or COUNTER)
    Ordered(CounterKind),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CounterKind {
    Letter(u8),
    Number(u8),
}

impl BulletKind {
    pub(crate) fn parse(byte_arr: &[u8], index: usize) -> Result<Match<BulletKind>> {
        match byte_arr[index] {
            STAR | HYPHEN | PLUS => {
                if byte_arr
                    .get(index + 1)
                    .ok_or(MatchError::EofError)?
                    .is_ascii_whitespace()
                {
                    Ok(Match {
                        start: index,
                        end: index + 2,
                        obj: BulletKind::Unordered,
                    })
                } else {
                    return Err(MatchError::InvalidLogic);
                }
            }
            chr if chr.is_ascii_alphanumeric() => {
                let num_match = fn_until(byte_arr, index, |chr| {
                    !chr.is_ascii_alphanumeric()
                    // effectively these ↓
                    // || chr == PERIOD || chr == RPAREN
                })?;

                let idx = num_match.end;
                if !(byte_arr[idx] == PERIOD || byte_arr[idx] == RPAREN) {
                    Err(MatchError::InvalidLogic)?
                }
                let bullet_kind = if num_match.len() == 1 {
                    let temp = num_match.obj.as_bytes()[0];
                    if temp.is_ascii_alphabetic() {
                        BulletKind::Ordered(CounterKind::Letter(temp))
                    } else if temp.is_ascii_digit() {
                        BulletKind::Ordered(CounterKind::Number(temp))
                    } else {
                        Err(MatchError::InvalidLogic)?
                    }
                } else {
                    // must be a number
                    BulletKind::Ordered(CounterKind::Number(
                        num_match.obj.parse().or(Err(MatchError::InvalidLogic))?,
                    ))
                };

                if !byte_arr
                    .get(idx + 1)
                    .ok_or(MatchError::EofError)?
                    .is_ascii_whitespace()
                {
                    Err(MatchError::InvalidLogic)?
                }

                Ok(Match {
                    start: index,
                    end: idx + 2,
                    obj: bullet_kind,
                })
            }

            _ => Err(MatchError::InvalidLogic),
        }
    }
}

fn parse_counter_set(byte_arr: &[u8], index: usize) -> Result<Match<CounterKind>> {
    if index == byte_arr.len() {
        Err(MatchError::EofError)?
    }
    let idx = skip_ws(byte_arr, index);

    if byte_arr[idx] != LBRACK && *byte_arr.get(idx + 1).ok_or(MatchError::EofError)? != b'@' {
        Err(MatchError::InvalidLogic)?
    }


    let num_match = fn_until(byte_arr, idx + 2, |chr| {
        !chr.is_ascii_alphanumeric()
        // effectively these ↓
        // || chr == PERIOD || chr == RPAREN
    })?;

    // TODO: errors on eof
    if byte_arr[num_match.end] != RBRACK {
        Err(MatchError::InvalidLogic)?
    }

    let counter_kind = if num_match.len() == 1 {
        let temp = num_match.obj.as_bytes()[0];
        if temp.is_ascii_alphabetic() {
            CounterKind::Letter(temp)
        } else if temp.is_ascii_digit() {
            CounterKind::Number(temp)
        } else {
            Err(MatchError::InvalidLogic)?
        }
    } else {
        // must be a number
        CounterKind::Number(num_match.obj.parse().or(Err(MatchError::InvalidLogic))?)
    };

    Ok(Match {
        start: index,
        end: num_match.end + 1,
        obj: counter_kind,
    })
}

fn parse_tag(byte_arr: &[u8], index: usize) -> Result<Match<&str>> {
    // - [@A] [X] | our tag is here :: remainder
    if index == byte_arr.len() {
        Err(MatchError::EofError)?
    }
    let mut idx = skip_ws(byte_arr, index);
    let end = loop {
        match *byte_arr.get(idx).ok_or(MatchError::EofError)? {
            COLON => {
                if byte_arr[idx - 1].is_ascii_whitespace()
                    && COLON == *byte_arr.get(idx + 1).ok_or(MatchError::EofError)?
                    && byte_arr
                        .get(idx + 2)
                        .ok_or(MatchError::EofError)?
                        .is_ascii_whitespace()
                {
                    break idx + 2;
                }
            }
            NEWLINE => Err(MatchError::EofError)?,
            _ => idx += 1,
        }
    };

    Ok(Match {
        start: index,
        end,
        obj: bytes_to_str(&byte_arr[index..end - 2]).trim(),
    })
}

#[derive(Debug, Clone)]
pub enum CheckBox {
    /// [-]
    Intermediate,
    /// [ ]
    Off,
    /// [X]
    On,
}

impl CheckBox {
    fn parse(byte_arr: &[u8], index: usize) -> Result<Match<CheckBox>> {
        if index == byte_arr.len() {
            Err(MatchError::EofError)?
        }
        let idx = skip_ws(byte_arr, index);
        // we're at a LBRACK in theory here
        // 012
        // [ ]
        if idx + 2 < byte_arr.len() {
            if byte_arr[idx] != LBRACK && byte_arr[idx + 2] != RBRACK {
                return Err(MatchError::EofError);
            }
        } else {
            return Err(MatchError::EofError);
        }

        Ok(Match {
            start: index,
            end: idx + 3,
            obj: match byte_arr[idx + 1].to_ascii_lowercase() {
                b'x' => Self::On,
                SPACE => Self::Off,
                HYPHEN => Self::Intermediate,
                _ => Err(MatchError::InvalidLogic)?,
            },
        })
    }
}
