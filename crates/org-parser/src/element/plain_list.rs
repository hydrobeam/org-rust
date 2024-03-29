use crate::node_pool::NodeID;
use crate::parse::parse_element;
use crate::types::{Cursor, Expr, ParseOpts, Parseable, Parser, Result};

use crate::element::Item;
use crate::utils::variant_eq;

use super::{BulletKind, CounterKind};

#[derive(Debug, Clone)]
pub struct PlainList {
    pub children: Vec<NodeID>,
    pub kind: ListKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ListKind {
    Unordered,
    Ordered(CounterKind),
    Descriptive,
}

impl<'a> Parseable<'a> for PlainList {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        // parse opts will provide us the appropriate indentation level

        // prevents nested lists from adding unecessary levels of indentation
        let start = cursor.index;

        if !parse_opts.from_list {
            parse_opts.indentation_level += 1;
            parse_opts.from_list = true;
        }

        let original_item_id = Item::parse(parser, cursor, parent, parse_opts)?;
        let reserve_id = parser.pool.reserve_id();

        let item_node = &mut parser.pool[original_item_id];
        let kind = if let Expr::Item(item) = &item_node.obj {
            find_kind(item)
        } else {
            unreachable!()
        };
        item_node.parent = Some(reserve_id);

        let mut children: Vec<NodeID> = Vec::new();
        children.push(original_item_id);

        cursor.index = item_node.end;

        while let Ok(element_id) = parse_element(parser, cursor, Some(reserve_id), parse_opts) {
            let got_obj = &parser.pool[element_id];
            match &got_obj.obj {
                Expr::Item(item) => {
                    let item_kind = find_kind(item);
                    // makes it so that a list that encounters a different bullet list
                    // starts a new list
                    if !variant_eq(&item_kind, &kind) {
                        // HACK: modifying the cache directly is janky
                        // but we need to do this so that we dont run into a cached item
                        parser.cache.remove(&got_obj.start);
                        break;
                    } else {
                        children.push(element_id);
                        cursor.index = got_obj.end;
                    }
                }
                _ => {
                    break;
                }
            }
        }
        Ok(parser.alloc_with_id(
            Self { children, kind },
            start,
            cursor.index,
            parent,
            reserve_id,
        ))
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
    use crate::{parse_org, Expr};

    #[test]
    fn basic_list() {
        let input = r"
- one
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_two_items() {
        let input = r"
- one
- two
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_continued_item() {
        let input = r"
- one
 abcdef
- two
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_space_ending() {
        let input = r"
- one
 abcdef
- two
- three
- four


- five
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_inconsistent_types() {
        let input = r"
- one
 abcdef
1. two
2. three
3. four
- five
- six
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_elements_breaking_flow() {
        let input = r"
- one
 abcdef
- four
this aint a list baby
#+begin_src python
not a list too
#+end_src


- five
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_contained_elements() {
        let input = r"
- one
      abcd
      eif
  #+begin_example
  we are eating so good?
  #+end_example

- two
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn nested_lists_basic() {
        let input = r"
- one
 - two
- three
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_empty() {
        let input = r"
-
-
-
-
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_numbered_empty() {
        let input = r"
1.
2.
3.
4.
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn nested_list_2() {
        let input = r"
- one
  - two
    - three
   - four
- five
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn nested_list_3() {
        let input = r"
- one
  - two
    - three
  - four
- five
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn nested_list_4() {
        let input = r"
1. item 1
2. [X] item 2
   - some tag :: item 2.1
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn blank_list() {
        let input = r"1. item 1
   abcdef

   next one two three four five

   more thangs more thangs more thangs
   more thangs

2. [X] item 2
   - aome tag :: item 2.1
";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn combined_list() {
        let input = r"
- zero
- one

a*
";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn indent_list_prop() {
        let input = r"
- one
- two
  - qq


 heyy
";
        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn list_no_item_with_sub_element() {
        let input = r"-   [X]
      |a|a|a|a|

-

";

        let pool = parse_org(input);
        pool.print_tree();
    }

    #[test]
    fn mixed_list() {
        let input = r#"1. one
- two
"#;

        let pool = parse_org(input);
        assert_eq!(
            pool.pool
                .iter()
                .filter(|x| matches!(x.obj, Expr::PlainList(_)))
                .count(),
            2
        );
    }
}
