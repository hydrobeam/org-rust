use crate::node_pool::{NodeID, NodePool};
use crate::parse::parse_element;
use crate::types::{Expr, Node, ParseOpts, Parseable, Result};

use crate::element::Item;

use super::{BulletKind, CounterKind};

#[derive(Debug, Clone)]
pub struct PlainList {
    pub children: Vec<NodeID>,
    pub kind: ListKind,
    // identation_level: u8,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ListKind {
    Unordered,
    Ordered(CounterKind),
    Descriptive,
}

impl<'a> Parseable<'a> for PlainList {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        // parse opts will provide us the appropriate indentation level

        parse_opts.from_list = true;
        let original_item_id = Item::parse(pool, byte_arr, index, parent, parse_opts)?;
        let reserve_id = pool.reserve_id();

        let item_node = &mut pool[original_item_id];
        let kind = if let Expr::Item(item) = &item_node.obj {
            find_kind(item)
        } else {
            unreachable!()
        };
        item_node.parent = Some(reserve_id);

        let mut children: Vec<NodeID> = Vec::new();
        children.push(original_item_id);
        let mut idx = item_node.end;
        while let Ok(element_id) = Item::parse(pool, byte_arr, idx, Some(reserve_id), parse_opts) {
            let got_obj = &pool[element_id];
            match &got_obj.obj {
                Expr::Item(item) => {
                    let item_kind = find_kind(item);
                    if item_kind != kind {
                        break;
                    } else {
                        children.push(element_id);
                        idx = got_obj.end;
                    }
                }
                Expr::PlainList(_) => break,
                _ => unreachable!(),
            }
        }
        Ok(pool.alloc_with_id(Self { children, kind }, index, idx, parent, reserve_id))
    }
}

fn find_kind(item: &Item) -> ListKind {
    if let Some(tag) = item.tag {
        ListKind::Descriptive
    } else if let BulletKind::Ordered(counter_kind) = item.bullet {
        ListKind::Ordered(counter_kind)
    } else {
        ListKind::Unordered
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_list() {
        let input = r"
- one
";

        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }
}
