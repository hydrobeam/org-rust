use crate::constants::{COLON, HYPHEN, LBRACK, NEWLINE, PERIOD, PLUS, RBRACK, RPAREN, SPACE, STAR};
use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{Cursor, Expr, MatchError, ParseOpts, Parseable, Parser, Result};
use crate::utils::Match;

#[derive(Debug, Clone)]
pub struct Item<'a> {
    pub bullet: BulletKind,
    // An instance of the pattern [@COUNTER]
    pub counter_set: Option<&'a str>,
    pub check_box: Option<CheckBox>,
    pub tag: Option<&'a str>,
    pub children: Vec<NodeID>,
}

impl<'a> Parseable<'a> for Item<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        // Will only ever really get called via Plainlist.

        let start = cursor.index;

        let bullet_match = BulletKind::parse(cursor)?;
        let bullet = bullet_match.obj;
        cursor.move_to(bullet_match.end);

        let counter_set: Option<&'a str> = if let Ok(counter_match) = parse_counter_set(cursor) {
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

        let tag: Option<&str> = if let BulletKind::Unordered = bullet {
            if let Ok(tag_match) = parse_tag(cursor) {
                cursor.move_to(tag_match.end);
                Some(tag_match.obj)
            } else {
                None
            }
        } else {
            None
        };

        let reserve_id = parser.pool.reserve_id();
        let mut children: Vec<NodeID> = Vec::new();
        let mut blank_obj: Option<NodeID> = None;

        // if the last element was a \n, that means we're starting on a new line
        // so we are Not on a list line.
        cursor.skip_ws();
        if let Ok(item) = cursor.try_curr() {
            if item == NEWLINE {
                cursor.next()
            } else {
                parse_opts.list_line = true;
            }
        }

        // used to restore index to the previous position in the event of two
        // blank lines
        let mut prev_ind = cursor.index;

        while let Ok(element_id) = parse_element(parser, cursor, Some(reserve_id), parse_opts) {
            let pool_loc = &parser.pool[element_id];
            match &pool_loc.obj {
                Expr::BlankLine => {
                    if blank_obj.is_some() {
                        cursor.index = prev_ind;
                        break;
                    } else {
                        blank_obj = Some(element_id);
                        prev_ind = cursor.index;
                    }
                }
                Expr::Item(_) => {
                    break;
                }
                _ => {
                    if let Some(blank_id) = blank_obj {
                        children.push(blank_id);
                        blank_obj = None;
                    }
                    children.push(element_id);
                }
            }
            parse_opts.list_line = false;
            cursor.move_to(pool_loc.end);
        }

        Ok(parser.alloc_with_id(
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
        // -\n is valid, so we don't want to skip past the newline
        // since -    \n is also valid
        // is valid
        let start = cursor.index;
        match cursor.curr() {
            STAR | HYPHEN | PLUS => {
                if cursor.peek(1)?.is_ascii_whitespace() {
                    Ok(Match {
                        start,
                        end: cursor.index + if cursor.peek(1)? == NEWLINE { 1 } else { 2 },
                        obj: BulletKind::Unordered,
                    })
                } else {
                    Err(MatchError::InvalidLogic)
                }
            }
            chr if chr.is_ascii_alphanumeric() => {
                let num_match = cursor.fn_while(|chr| {
                    chr.is_ascii_alphanumeric()
                    // effectively these ↓
                    // || chr == PERIOD || chr == RPAREN
                })?;

                cursor.index = num_match.end;

                if !((cursor.curr() == PERIOD || cursor.curr() == RPAREN)
                    && cursor.peek(1)?.is_ascii_whitespace())
                {
                    return Err(MatchError::InvalidLogic);
                }

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

                Ok(Match {
                    start,
                    end: cursor.index + if cursor.peek(1)? == NEWLINE { 1 } else { 2 },
                    obj: bullet_kind,
                })
            }

            _ => Err(MatchError::InvalidLogic),
        }
    }
}

// - [@4]
fn parse_counter_set(mut cursor: Cursor<'_>) -> Result<Match<&str>> {
    let start = cursor.index;
    cursor.skip_ws();
    cursor.word("[@")?;

    let num_match = cursor.fn_while(|chr| chr.is_ascii_alphanumeric())?;

    cursor.index = num_match.end;

    // cursor.curr() is valid because num_match above ensures we're at a valid point
    if cursor.curr() != RBRACK {
        Err(MatchError::InvalidLogic)?;
    }

    let counter_kind = if num_match.len() == 1 {
        let temp = num_match.obj.as_bytes()[0];
        if temp.is_ascii_alphanumeric() {
            num_match.obj
        } else {
            return Err(MatchError::InvalidLogic);
        }
    } else {
        // must be a number
        if num_match
            .obj
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit())
        {
            num_match.obj
        } else {
            return Err(MatchError::InvalidLogic);
        }
    };

    Ok(Match {
        start,
        end: cursor.index + 1,
        obj: counter_kind,
    })
}

fn parse_tag<'a>(mut cursor: Cursor<'a>) -> Result<Match<&'a str>> {
    // - [@A] [X] | our tag is here :: remainder
    let start = cursor.index;
    cursor.curr_valid()?;
    cursor.skip_ws();

    let end = loop {
        match cursor.try_curr()? {
            COLON => {
                if cursor[cursor.index - 1].is_ascii_whitespace()
                    && COLON == cursor.peek(1)?
                    && cursor.peek(2)?.is_ascii_whitespace()
                {
                    break cursor.index + 2;
                } else {
                    cursor.next();
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckBox {
    /// [-]
    Intermediate,
    /// [ ]
    Off,
    /// \[X\]
    On,
}

impl From<&CheckBox> for &str {
    fn from(value: &CheckBox) -> Self {
        match value {
            CheckBox::Intermediate => "-",
            CheckBox::Off => " ",
            CheckBox::On => "X",
        }
    }
}

impl CheckBox {
    fn parse(mut cursor: Cursor) -> Result<Match<CheckBox>> {
        let start = cursor.index;
        cursor.skip_ws();
        // we're at a LBRACK in theory here
        // 012
        // [ ]

        if cursor.try_curr()? != LBRACK || cursor.peek(2)? != RBRACK {
            return Err(MatchError::InvalidLogic);
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

#[cfg(test)]
mod tests {
    use crate::{expr_in_pool, parse_org};

    use super::*;
    #[test]
    fn checkbox() {
        let input = "- [X]";
        let ret = parse_org(input);
        let item = expr_in_pool!(ret, Item).unwrap();
        assert_eq!(item.check_box, Some(CheckBox::On))
    }

    #[test]
    fn counter_set() {
        let input = "- [@1]";
        let ret = parse_org(input);
        let item = expr_in_pool!(ret, Item).unwrap();
        assert_eq!(item.counter_set, Some("1"));

        let input = "- [@43]";
        let ret = parse_org(input);
        let item = expr_in_pool!(ret, Item).unwrap();
        assert_eq!(item.counter_set, Some("43"))
    }

    #[test]
    fn no_newline_hyphen() {
        let input = "-";
        let ret = parse_org(input);
        let item = expr_in_pool!(ret, Plain).unwrap();
        assert_eq!(item, &"-");
    }
    #[test]
    fn hyphen_space() {
        let input = "- ";
        let ret = parse_org(input);
        let item = expr_in_pool!(ret, Item).unwrap();
    }

    #[test]
    fn hyphen_lbrack() {
        let input = "- [";
        let ret = parse_org(input);
        let plain = expr_in_pool!(ret, Plain).unwrap();
        assert_eq!(plain, &"[");
    }
    #[test]
    fn hyphen_ltag() {
        let input = "- [@";
        let ret = parse_org(input);
        let plain = expr_in_pool!(ret, Plain).unwrap();
        assert_eq!(plain, &"[@");
    }
    #[test]
    fn item_ordered_start() {
        let input = "1. ";
        let ret = parse_org(input);
        let item = expr_in_pool!(ret, Item).unwrap();
        assert!(matches!(
            item.bullet,
            BulletKind::Ordered(CounterKind::Number(1))
        ));

        let input = "17. ";
        let ret = parse_org(input);
        let item = expr_in_pool!(ret, Item).unwrap();
        assert!(matches!(
            item.bullet,
            BulletKind::Ordered(CounterKind::Number(17))
        ));

        let input = "a. ";
        let ret = parse_org(input);
        let item = expr_in_pool!(ret, Item).unwrap();
        assert!(matches!(
            item.bullet,
            BulletKind::Ordered(CounterKind::Letter(b'a'))
        ));
    }
}
