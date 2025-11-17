use crate::constants::{HYPHEN, NEWLINE};
use crate::node_pool::NodeID;
use crate::object::TableCell;
use crate::types::{Cursor, Expr, MarkupKind, ParseOpts, Parseable, Parser, Result};

/// A table consisting of a collection of [`TableRow`]s
///
/// | one | two |
/// | three | four |
#[derive(Debug, Clone)]
pub struct Table {
    pub rows: usize,
    pub cols: usize,
    pub children: Vec<NodeID>,
    pub caption: Option<NodeID>, // will be filled by parent caption if exists
}

/// A row of a [`Table`] consisting of [`TableCell`]s or a [`TableRow::Rule`].
///
/// A [`TableRow::Rule`] occurs when a row begins with a hyphen:
///
/// ```text
/// |1|2|
/// |---|
/// |3|4|
/// ```
///
/// This table's rows are:
///
/// ```text
/// TableRow::Standard(TableCell, TableCell)
/// TableRow::Rule
/// TableRow::Standard(TableCell, TableCell)
/// ```
#[derive(Debug, Clone)]
pub enum TableRow {
    Rule, // hrule
    Standard(Vec<NodeID>),
}

impl<'a> Parseable<'a> for Table {
    fn parse(
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        // we are a table now
        parse_opts.markup.insert(MarkupKind::Table);
        let reserve_id = parser.pool.reserve_id();
        let mut children: Vec<NodeID> = Vec::new();
        let mut rows = 0;
        let mut cols = 0;
        while let Ok(row_id) = TableRow::parse(parser, cursor, Some(reserve_id), parse_opts) {
            let obj = &parser.pool[row_id];

            children.push(row_id);
            rows += 1;
            if let Expr::TableRow(TableRow::Standard(node_ids)) = &obj.obj {
                cols = cols.max(node_ids.len());
            }

            cursor.index = parser.pool[row_id].end;
        }

        Ok(parser.alloc_with_id(
            Self {
                rows,
                cols,
                children,
                caption: None,
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
        parser: &mut Parser<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        // TODO: doesn't play well with lists
        // should break if the indentation is not even for the next element in the list
        // but shouldn't break otherwise
        cursor.curr_valid()?;
        cursor.skip_ws();
        cursor.word("|")?;

        // we deliberately avoid erroring out of the function from here
        // in the case of "|EOF" since we need to allocate a node to tell the parent
        // that we've successfully read past the "|"
        // otherwise, Table will allocate in the cache with zero progress, and cause an infinite loop

        // implies horizontal rule
        // |-
        if let Ok(val) = cursor.try_curr()
            && val == HYPHEN
        {
            // adv_till_byte handles eof
            cursor.adv_till_byte(b'\n');
            // cursor.index + 1 to start at the next | on the next line
            return Ok(parser
                .pool
                .alloc(Self::Rule, start, cursor.index + 1, parent));
        }

        let mut children: Vec<NodeID> = Vec::new();
        while let Ok(table_cell_id) = TableCell::parse(parser, cursor, parent, parse_opts) {
            let node_item = &parser.pool[table_cell_id];
            children.push(table_cell_id);

            cursor.index = node_item.end;
            if let Ok(val) = cursor.try_curr() {
                if val == NEWLINE {
                    cursor.next();
                    break;
                }
            } else {
                break;
            }
        }

        Ok(parser
            .pool
            .alloc(Self::Standard(children), start, cursor.index, parent))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Expr, expr_in_pool, parse_org};

    #[test]
    fn basic_table() {
        let input = r"
|one|two|
|three|four|
";
        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn table_eof_1() {
        let input = r"
|one|two|
|three|four|
";
        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn table_eof_2() {
        let input = r"
|one|two|
|three|four|";
        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn table_no_nl() {
        let input = r"
|one|two
|three|four

";
        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn table_with_hrule() {
        let input = r"
|one|two
|--------|
|three|four

";
        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn table_markup_1() {
        let input = r"
|one|tw *o*                                      |
|three|four|
";
        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn table_empty_cells() {
        let input = r"
||a|
|b||
";
        let pool = parse_org(input);

        pool.print_tree();
    }

    /// test that alignment spaces are removed
    #[test]
    fn table_aligned_cells() {
        let input = r"
|one two |three|
|s       |     |
";

        let pool = parse_org(input);

        pool.print_tree();
    }

    #[test]
    fn table_uneven_cols() {
        let input = r"
|one two |three|||||
|s       |     |
";

        let pool = parse_org(input);

        pool.print_tree();
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
        pool.print_tree();
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

        pool.print_tree();
    }

    #[test]
    fn table_no_start() {
        let input = r"|";

        let pool = parse_org(input);
        let tab = expr_in_pool!(pool, Table).unwrap();
        assert_eq!(tab.rows, 1);
    }
}
