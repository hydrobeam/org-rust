use crate::constants::{COLON, HYPHEN, LBRACK, NEWLINE, PERIOD, PLUS, RBRACK, RPAREN, SPACE, STAR};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{Cursor, Expr, MatchError, ParseOpts, Parseable, Result};
use crate::utils::Match;

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
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        // Will only ever really get called via Plainlist.

        let start = cursor.index;

        let bullet_match = BulletKind::parse(cursor)?;
        let bullet = bullet_match.obj;
        cursor.move_to(bullet_match.end);

        let counter_set: Option<CounterKind> = if let Ok(counter_match) = parse_counter_set(cursor)
        {
            cursor.move_to(counter_match.end);
            Some(counter_match.obj)
        } else {
            None
        };

        let check_box: Option<CheckBox> = if let Ok(check_box_match) = CheckBox::parse(cursor) {
            cursor.move_to(check_box_match.end);
            Some(check_box_match.obj)
        } else {
            None
        };

        let tag: Option<&str> = if let Ok(tag_match) = parse_tag(cursor) {
            cursor.move_to(tag_match.end);
            Some(tag_match.obj)
        } else {
            None
        };

        let reserve_id = pool.reserve_id();
        let mut children: Vec<NodeID> = Vec::new();
        let mut blank_obj: Option<NodeID> = None;

        // if the last element was a \n, that means we're starting on a new line
        // so we are Not on a list line.
        parse_opts.list_line = cursor[cursor.index - 1] != NEWLINE;

        while let Ok(element_id) = parse_element(pool, cursor, Some(reserve_id), parse_opts) {
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
            cursor.move_to(pool_loc.end);
        }

        Ok(pool.alloc_with_id(
            Self {
                bullet,
                counter_set,
                check_box,
                tag,
                children,
            },
            start,
            cursor.index,
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
    pub(crate) fn parse(mut cursor: Cursor) -> Result<Match<BulletKind>> {
        let start = cursor.index;
        match cursor.curr() {
            STAR | HYPHEN | PLUS => {
                if cursor.peek(1)?.is_ascii_whitespace() {
                    Ok(Match {
                        start,
                        end: cursor.index + 2,
                        obj: BulletKind::Unordered,
                    })
                } else {
                    return Err(MatchError::InvalidLogic);
                }
            }
            chr if chr.is_ascii_alphanumeric() => {
                let num_match = cursor.fn_while(|chr| {
                    chr.is_ascii_alphanumeric()
                    // effectively these ↓
                    // || chr == PERIOD || chr == RPAREN
                })?;


                cursor.index = num_match.end;

                if !(cursor.curr() == PERIOD || cursor.curr() == RPAREN) {
                    return Err(MatchError::InvalidLogic);
                }
                cursor.next();

                let bullet_kind = if num_match.len() == 1 {
                    let temp = num_match.obj.as_bytes()[0];
                    if temp.is_ascii_alphabetic() {
                        BulletKind::Ordered(CounterKind::Letter(temp))
                    } else if temp.is_ascii_digit() {
                        BulletKind::Ordered(CounterKind::Number(temp - 48))
                    } else {
                        Err(MatchError::InvalidLogic)?
                    }
                } else {
                    // must be a number
                    BulletKind::Ordered(CounterKind::Number(
                        num_match.obj.parse().or(Err(MatchError::InvalidLogic))?,
                    ))
                };

                if cursor.peek(1)?.is_ascii_whitespace() {
                    return Err(MatchError::InvalidLogic);
                }

                Ok(Match {
                    start,
                    end: cursor.index + 2,
                    obj: bullet_kind,
                })
            }

            _ => Err(MatchError::InvalidLogic),
        }
    }
}

fn parse_counter_set(mut cursor: Cursor) -> Result<Match<CounterKind>> {
    let start = cursor.index;
    cursor.is_index_valid()?;
    cursor.skip_ws();

    if cursor.curr() != LBRACK && cursor.peek(1)? != b'@' {
        Err(MatchError::InvalidLogic)?
    }

    let num_match = cursor.fn_while(|chr| {
        chr.is_ascii_alphanumeric()
        // effectively these ↓
        // || chr == PERIOD || chr == RPAREN
    })?;
    cursor.move_to(num_match.end);

    // TODO: errors on eof
    if cursor.curr() != RBRACK {
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
        start,
        end: cursor.index + 1,
        obj: counter_kind,
    })
}

fn parse_tag(mut cursor: Cursor) -> Result<Match<&str>> {
    // - [@A] [X] | our tag is here :: remainder
    let start = cursor.index;
    cursor.is_index_valid()?;
    cursor.skip_ws();

    let end = loop {
        match cursor.try_curr()? {
            COLON => {
                if cursor[cursor.index - 1].is_ascii_whitespace()
                    && COLON == cursor.peek(1)?
                    && cursor.peek(2)?.is_ascii_whitespace()
                {
                    break cursor.index + 2;
                }
            }
            NEWLINE => Err(MatchError::EofError)?,
            _ => cursor.next(),
        }
    };

    Ok(Match {
        start,
        end,
        obj: cursor.clamp_backwards(start).trim(),
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
    fn parse(mut cursor: Cursor) -> Result<Match<CheckBox>> {
        let start = cursor.index;
        cursor.is_index_valid()?;
        cursor.skip_ws();
        // we're at a LBRACK in theory here
        // 012
        // [ ]
        if cursor.curr() != LBRACK && cursor.peek(2)? != RBRACK {
            return Err(MatchError::EofError);
        }

        Ok(Match {
            start,
            end: cursor.index + 3,
            obj: match cursor[cursor.index + 1].to_ascii_lowercase() {
                b'x' => Self::On,
                SPACE => Self::Off,
                HYPHEN => Self::Intermediate,
                _ => Err(MatchError::InvalidLogic)?,
            },
        })
    }
}
