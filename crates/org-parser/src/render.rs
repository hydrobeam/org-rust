//! Text rendering of a parsed document (primarily for testing/debugging).

use std::collections::HashMap;
use std::fmt::{Debug, Write};

use crate::element::Affiliated;
use crate::node_pool::NodeID;
use crate::types::{Expr, Parser};

/// Every `NodeID` an expression owns, in source order.
fn child_ids(expr: &Expr) -> Vec<NodeID> {
    let mut ids: Vec<NodeID> = Vec::new();
    if let Expr::Heading(h) = expr
        && let Some((_, title)) = &h.title
    {
        ids.extend(title);
    }
    if let Some(children) = expr.children() {
        ids.extend(children);
    }
    match expr {
        Expr::RegularLink(l) => ids.extend(l.caption),
        Expr::Affiliated(Affiliated::Name(id)) => ids.extend(id),
        Expr::Affiliated(Affiliated::Caption(id)) => ids.push(*id),
        Expr::Affiliated(Affiliated::Attr { child_id, .. }) => ids.extend(child_id),
        _ => {}
    }
    ids
}

fn render_node(parser: &Parser, id: NodeID, depth: usize, out: &mut String) {
    let node = &parser.pool[id];
    // https://doc.rust-lang.org/std/fmt/#width for the indent trick
    let _ = writeln!(
        out,
        "{:indent$}{}..{} {:?}",
        "",
        node.start,
        node.end,
        node.obj,
        indent = depth * 2
    );
    for child in child_ids(&node.obj) {
        render_node(parser, child, depth + 1, out);
    }
}

/// Writes `title:` followed by one `key sep value` line per entry, sorted by key so the
/// output doesn't depend on `HashMap` iteration order. Writes nothing for an empty map.
fn render_section<V: Debug>(out: &mut String, title: &str, sep: &str, map: &HashMap<&str, V>) {
    if map.is_empty() {
        return;
    }
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by_key(|(key, _)| **key);
    let _ = writeln!(out, "{title}:");
    for (key, val) in entries {
        let _ = writeln!(out, "  {key:?} {sep} {val:?}");
    }
}


impl Parser<'_> {
    /// Renders the parsed tree as indented text.
    ///
    /// Each line is one node: its span in the source (`start..end`) followed by its
    /// contents, indented by depth. Child lists inside a node's contents show the
    /// [`NodeID`]s of its children. Only nodes reachable from the root are shown.
    ///
    /// The tree is followed by the parser's `targets`, `keywords` and `macros` tables, each
    /// under its own heading and sorted by key. A table with no entries is omitted.
    ///
    /// ```rust
    /// use org_rust_parser as org_parser;
    ///
    /// let parsed = org_parser::parse_org("hello /world/\n");
    /// let tree = parsed.render_tree();
    ///
    /// assert!(tree.contains("Italic"));
    ///
    /// let parsed = org_parser::parse_org("#+title: hi\n");
    /// assert!(parsed.render_tree().contains("keywords:\n  \"title\" = \"hi\""));
    /// ```
    pub fn render_tree(&self) -> String {
        let mut out = String::new();
        render_node(self, self.pool.root_id(), 0, &mut out);
        render_section(&mut out, "targets", "->", &self.targets);
        render_section(&mut out, "keywords", "=", &self.keywords);
        render_section(&mut out, "macros", "=", &self.macros);
        out
    }
}
