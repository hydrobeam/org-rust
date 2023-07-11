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
        cursor.is_index_valid()?;
        cursor.skip_ws();
        cursor.word("|")?;

        // implies horizontal rule
        // |-
        if cursor.try_curr()? == HYPHEN {
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
            // REVIEW: use try_curr in case of table ending at eof?
            if cursor.curr() == NEWLINE {
                cursor.next();
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
    use crate::parse_org;

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
    #[should_panic]
    // we don't handle the eof case for table cells /shrug/
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

    // #[test]
    // #[should_panic]
    // fn table_no_start() {
    //     let input = r"|";

    //     let pool = parse_org(input);

    //     pool.pool.root().print_tree(&pool);
    // }
}
