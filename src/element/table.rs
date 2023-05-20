use crate::constants::{HYPHEN, VBAR};
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

impl<'a> Parseable<'a> for TableCell {
    fn parse(
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        // skip starting |
        cursor.next();

        let mut content_vec: Vec<NodeID> = Vec::new();
        while let Ok(id) = parse_object(pool, cursor, parent, parse_opts) {
            cursor.index = pool[id].end;
            if let Expr::MarkupEnd(MarkupKind::Table) = pool[id].obj {
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
        return Ok(pool.alloc_with_id(Self(content_vec), start, cursor.index, parent, new_id));
    }
}

#[derive(Debug, Clone)]
pub enum TableRow {
    Rule, // hrule
    Standard(Vec<NodeID>),
}

impl<'a> Parseable<'a> for TableRow {
    fn parse(
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        // implies horizontal rule
        // |-
        if cursor.peek(1)? == HYPHEN {
            // adv_till_byte handles eof
            cursor.adv_till_byte(b'\n');
            // cursor.index + 1 to start at the next | on the next line
            return Ok(pool.alloc(Self::Rule, start, cursor.index + 1, parent));
        }

        let mut children: Vec<NodeID> = Vec::new();
        while let Ok(table_cell_id) = TableCell::parse(pool, cursor, parent, parse_opts) {
            let node_item = &pool[table_cell_id];
            children.push(table_cell_id);

            cursor.index = node_item.end;
        }

        return Ok(pool.alloc(Self::Standard(children), start, cursor.index, parent));
    }
}

impl<'a> Parseable<'a> for Table {
    fn parse(
        pool: &mut NodePool<'a>,
        mut cursor: Cursor<'a>,
        parent: Option<NodeID>,
        mut parse_opts: ParseOpts,
    ) -> Result<NodeID> {
        let start = cursor.index;

        if cursor.curr() != VBAR {
            return Err(MatchError::InvalidLogic)?;
        }
        // we are a table now
        parse_opts.markup.insert(MarkupKind::Table);
        let reserve_id = pool.reserve_id();
        let mut children: Vec<NodeID> = Vec::new();
        let mut rows = 0;
        let mut cols = 0;
        while let Ok(row_id) = TableRow::parse(pool, cursor, Some(reserve_id), parse_opts) {
            let obj = &pool[row_id];

            children.push(row_id);
            cols += 1;
            if let Expr::TableRow(TableRow::Standard(node_ids)) = &obj.obj {
                rows = rows.max(node_ids.len());
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
