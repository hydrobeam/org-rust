use crate::constants::{HYPHEN, NEWLINE, VBAR};
use crate::node_pool::{NodeID, NodePool};
use crate::parse::{parse_element, parse_object};
use crate::types::{Cursor, Expr, MarkupKind, MatchError, ParseOpts, Parseable, Result};

#[derive(Debug, Clone)]
pub struct Table {
    pub rows: usize,
    pub cols: usize,
    pub children: Vec<NodeID>,
}

#[derive(Debug, Clone)]
pub struct TableCell(pub Vec<NodeID>);

#[derive(Debug, Clone)]
pub enum TableRow {
    Rule, // hrule
    Standard(Vec<NodeID>),
}

impl<'a> Parseable<'a> for Table {
    fn parse(
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        // we are a table now
        parse_opts.markup.insert(MarkupKind::Table);
        let reserve_id = pool.reserve_id();
        let mut children: Vec<NodeID> = Vec::new();
        let mut rows = 0;
        let mut cols = 0;
        while let Ok(row_id) = TableRow::parse(pool, cursor, Some(reserve_id), parse_opts) {
            let obj = &pool[row_id];

            children.push(row_id);
            rows += 1;
            if let Expr::TableRow(TableRow::Standard(node_ids)) = &obj.obj {
                cols = cols.max(node_ids.len());
            }

            cursor.index = pool[row_id].end;
        }

        Ok(pool.alloc_with_id(
            Self {
                rows,
                cols,
                children,
            },
            start,
            cursor.index,
            parent,
            reserve_id,
        ))
    }
}

impl<'a> Parseable<'a> for TableRow {
    fn parse(
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        // TODO: doesn't play well with lists
        // should break if the indentation is not even for the next element in the list
        // but shouldn't break otherwise
        cursor.skip_ws();
        if cursor.try_curr()? != VBAR {
            return Err(MatchError::InvalidLogic)?;
        }

        // implies horizontal rule
        // |-
        if cursor.peek(1)? == HYPHEN {
            // adv_till_byte handles eof
            cursor.adv_till_byte(b'\n');
            // cursor.index + 1 to start at the next | on the next line
            return Ok(pool.alloc(Self::Rule, start, cursor.index + 1, parent));
        }
        // skip VBAR
        cursor.next();

        let mut children: Vec<NodeID> = Vec::new();
        while let Ok(table_cell_id) = TableCell::parse(pool, cursor, parent, parse_opts) {
            let node_item = &pool[table_cell_id];
            children.push(table_cell_id);

            cursor.index = node_item.end;
            // REVIEW: use try_curr in case of table ending at eof?
            if cursor.curr() == NEWLINE {
                cursor.next();
                break;
            }
        }

        return Ok(pool.alloc(Self::Standard(children), start, cursor.index, parent));
    }
}

impl<'a> Parseable<'a> for TableCell {
    fn parse(
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        let mut content_vec: Vec<NodeID> = Vec::new();
        while let Ok(id) = parse_object(pool, cursor, parent, parse_opts) {
            cursor.index = pool[id].end;
            if let Expr::MarkupEnd(MarkupKind::Table) = &pool[id].obj {
                break;
            } else {
                content_vec.push(id);
            }
        }

        // set parents of children
        // TODO: abstract this? stolen from markup.rs
        let new_id = pool.reserve_id();
        for id in content_vec.iter_mut() {
            pool[*id].parent = Some(new_id)
        }

        // get rid of alignment spaces, deleting the object if it becomes empty
        if let Some(last_id) = content_vec.last() {
            let last_item = &mut pool[*last_id];
            if let Expr::Plain(plains) = last_item.obj {
                let repl_str = plains.trim_end();
                if repl_str.trim_end().is_empty() {
                    content_vec.pop();
                } else {
                    last_item.obj = Expr::Plain(repl_str);
                }
            }
        }

        return Ok(pool.alloc_with_id(Self(content_vec), start, cursor.index, parent, new_id));
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_org;

    #[test]
    fn basic_table() {
        let input = r"
|one|two|
|three|four|

";
        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn table_eof() {
        let input = r"
|one|two|
|three|four|
";
        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn table_no_nl() {
        let input = r"
|one|two
|three|four

";
        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn table_with_hrule() {
        let input = r"
|one|two
|--------|
|three|four

";
        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn table_markup_1() {
        let input = r"
|one|tw *o*                                      |
|three|four|
";
        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn table_empty_cells() {
        let input = r"
||a|
|b||
";
        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }

    /// test that alignment spaces are removed
    #[test]
    fn table_aligned_cells() {
        let input = r"
|one two |three|
|s       |     |
";

        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn table_uneven_cols() {
        let input = r"
|one two |three|||||
|s       |     |
";

        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }

    #[test]
    fn table_indented() {
        let input = r"
word
        |one two |three|
        |s       |     |
        |four | five|
word

";

        let pool = parse_org(input);
        pool.root().print_tree(&pool);
    }

    #[test]
    fn table_indented_list() {
        let input = r"
- one
   - two
        |one two |three|
        |s       |     |
        |four | five|
- three
";

        let pool = parse_org(input);

        pool.root().print_tree(&pool);
    }
}
