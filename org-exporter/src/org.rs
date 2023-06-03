use std::fmt;
use std::fmt::Result;

use std::fmt::Write;

use crate::types::Exporter;
use org_parser::element::{BlockContents, BulletKind, CounterKind, Priority, TableRow, Tag};
use org_parser::node_pool::{NodeID, NodePool};
use org_parser::object::Emoji;
use org_parser::object::LatexFragment;
use org_parser::parse_org;
use org_parser::types::Expr;

pub struct Org<'a, 'buf> {
    buf: &'buf mut dyn fmt::Write,
    pool: &'a NodePool<'a>,
    indentation_level: u8,
    on_newline: bool,
}

impl<'a, 'buf> Exporter<'a, 'buf> for Org<'a, 'buf> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error> {
        let mut buf = String::new();
        let porg = parse_org(input);
        let mut obj = Org {
            buf: &mut buf,
            pool: &porg.pool,
            indentation_level: 0,
            on_newline: false,
        };

        obj.export_rec(&obj.pool.root_id())?;
        Ok(buf)
    }

    fn export_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error> {
        let porg = parse_org(input);
        let mut obj = Org {
            buf,
            pool: &porg.pool,
            indentation_level: 0,
            on_newline: false,
        };

        obj.export_rec(&obj.pool.root_id())?;
        Ok(buf)
    }

    fn export_rec(&mut self, node_id: &NodeID) -> Result {
        match &self.pool[*node_id].obj.clone() {
            Expr::Root(inner) => {
                for id in inner {
                    self.export_rec(id)?;
                }
            }
            Expr::Heading(inner) => {
                for _ in 0..inner.heading_level.into() {
                    write!(self, "*")?;
                }
                write!(self, " ")?;

                if let Some(keyword) = inner.keyword {
                    write!(self, "{keyword} ")?;
                }

                if let Some(priority) = &inner.priority {
                    write!(self, "[#")?;
                    match priority {
                        Priority::A => write!(self, "A")?,
                        Priority::B => write!(self, "B")?,
                        Priority::C => write!(self, "C")?,
                        Priority::Num(num) => write!(self, "{num}")?,
                    };
                    write!(self, "] ")?;
                }

                if let Some(title) = &inner.title {
                    for id in &title.1 {
                        self.export_rec(id)?;
                    }
                }

                // fn tag_search<T: Write>(loc: NodeID, pool: &NodePool, self: &mut T) -> Result {
                //     if let Expr::Heading(loc) = &pool[loc].obj {
                //         if let Some(sub_tags) = loc.tags.as_ref() {
                //             for thang in sub_tags.iter().rev() {
                //                 match thang {
                //                     Tag::Raw(val) => write!(self, ":{val}")?,
                //                     Tag::Loc(id) => {
                //                         tag_search(*id, pool, self)?;
                //                     }
                //                 }
                //             }
                //         }
                //     }
                //     Ok(())
                // }

                if let Some(tags) = &inner.tags {
                    let mut valid_out = String::new();
                    for tag in tags.iter().rev() {
                        match tag {
                            Tag::Raw(val) => write!(&mut valid_out, ":{val}")?,
                            Tag::Loc(_id) => {
                                // do nothing with it
                            }
                        }
                    }
                    // handles the case where a parent heading has no tags
                    if !valid_out.is_empty() {
                        write!(self, " {valid_out}:")?;
                    }
                }

                write!(self, "\n")?;

                if let Some(children) = &inner.children {
                    for id in children {
                        self.export_rec(id)?;
                    }
                }
            }
            Expr::Block(inner) => {
                let val: &str = inner.kind.into();
                write!(self, "#+begin_{val}")?;
                if let Some(params) = inner.parameters {
                    write!(self, " {params}")?;
                }
                write!(self, "\n")?;
                match &inner.contents {
                    BlockContents::Greater(children) => {
                        for id in children {
                            self.export_rec(id)?;
                        }
                        write!(self, "\n")?;
                    }
                    BlockContents::Lesser(cont) => {
                        write!(self, "{cont}\n")?;
                    }
                }
                write!(self, "#+end_{val}\n")?;
            }
            Expr::RegularLink(inner) => {
                write!(self, "[")?;
                write!(self, "[{}]", inner.path)?;
                if let Some(children) = &inner.description {
                    write!(self, "[")?;
                    for id in children {
                        self.export_rec(id)?;
                    }
                    write!(self, "]")?;
                }
                write!(self, "]")?;
            }

            Expr::Paragraph(inner) => {
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "\n")?;
            }

            Expr::Italic(inner) => {
                write!(self, "/")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "/")?;
            }
            Expr::Bold(inner) => {
                write!(self, "*")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "*")?;
            }
            Expr::StrikeThrough(inner) => {
                write!(self, "+")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "+")?;
            }
            Expr::Underline(inner) => {
                write!(self, "_")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "_")?;
            }
            Expr::BlankLine => {
                write!(self, "\n")?;
            }
            Expr::SoftBreak => {
                write!(self, " ")?;
            }
            Expr::Plain(inner) => {
                write!(self, "{inner}")?;
            }
            Expr::Verbatim(inner) => {
                write!(self, "={}=", inner.0)?;
            }
            Expr::Code(inner) => {
                write!(self, "~{}~", inner.0)?;
            }
            Expr::Comment(inner) => {
                write!(self, "# {}\n", inner.0)?;
            }
            Expr::InlineSrc(inner) => {
                write!(self, "src_{}", inner.lang)?;
                if let Some(args) = inner.headers {
                    write!(self, "[{args}]")?;
                }
                write!(self, "{{{}}}", inner.body)?;
            }
            Expr::Keyword(inner) => {
                write!(self, "#+{}: {}", inner.key, inner.val)?;
            }
            Expr::LatexEnv(inner) => {
                write!(
                    self,
                    r"\begin{{{0}}}
{1}
\end{{{0}}}
",
                    inner.name, inner.contents
                )?;
            }
            Expr::LatexFragment(inner) => match inner {
                LatexFragment::Command { name, contents } => {
                    write!(self, r"\{name}")?;
                    if let Some(command_cont) = contents {
                        write!(self, "{{{command_cont}}}")?;
                    }
                }
                LatexFragment::Display(inner) => {
                    write!(self, r"\[{inner}\]")?;
                }
                LatexFragment::Inline(inner) => {
                    write!(self, r"\({inner}\)")?;
                }
            },
            Expr::Item(inner) => {
                match inner.bullet {
                    BulletKind::Unordered => {
                        write!(self, "-")?;
                    }
                    BulletKind::Ordered(counterkind) => match counterkind {
                        CounterKind::Letter(lettre) => {
                            write!(self, "{}.", lettre as char)?;
                        }
                        CounterKind::Number(num) => {
                            write!(self, "{num}.")?;
                        }
                    },
                }
                write!(self, " ")?;

                if let Some(check) = &inner.check_box {
                    let val: &str = check.into();
                    write!(self, "[{val}] ")?;
                }

                if let Some(tag) = inner.tag {
                    write!(self, "{tag} :: ")?;
                }

                self.indentation_level += 1;
                for id in &inner.children {
                    self.export_rec(id)?;
                }
                self.indentation_level -= 1;
                if self.indentation_level == 0 {
                    self.on_newline = false;
                }
            }
            Expr::PlainList(inner) => {
                for id in &inner.children {
                    self.export_rec(id)?;
                }
            }
            Expr::PlainLink(inner) => {
                write!(self, "{}:{}", inner.protocol, inner.path)?;
            }
            Expr::Entity(inner) => {
                write!(self, "{}", inner.mapped_item)?;
            }
            Expr::Table(inner) => {
                let mut build_vec: Vec<Vec<String>> = Vec::with_capacity(inner.rows);
                // HACK: stop the table cells from receiving indentation from newline
                // in lists, manually retrigger it here

                for _ in 0..self.indentation_level {
                    self.buf.write_str("  ")?;
                }
                self.on_newline = false;

                // set up 2d array
                for id in &inner.children {
                    match &self.pool[*id].obj {
                        Expr::TableRow(row) => {
                            let mut row_vec = vec![];
                            match &row {
                                TableRow::Standard(stans) => {
                                    for id in stans {
                                        let mut cell_buf = String::new();
                                        let mut new_obj = Org {
                                            buf: &mut cell_buf,
                                            pool: self.pool,
                                            indentation_level: self.indentation_level,
                                            on_newline: self.on_newline,
                                        };
                                        new_obj.export_rec(id)?;
                                        row_vec.push(cell_buf);
                                    }
                                }
                                TableRow::Rule => {
                                    // an empty vec represents an hrule
                                }
                            }
                            build_vec.push(row_vec);
                        }
                        _ => unreachable!(),
                    }
                }

                // we use .get throughout because hrule rows are empty
                // and empty cells don't appear in the table, but we still have
                // to represent them
                //
                // run analysis to find column widths (padding)
                // travel downwards down rows, finding the largest length in each column
                let mut col_widths = Vec::with_capacity(inner.cols);
                for col_ind in 0..inner.cols {
                    let mut curr_max = 0;
                    for row_ind in 0..inner.rows {
                        curr_max =
                            curr_max.max(if let Some(strang) = build_vec[row_ind].get(col_ind) {
                                strang.len()
                            } else {
                                0
                            });
                    }
                    col_widths.push(curr_max);
                }

                for row_ind in 0..inner.rows {
                    write!(self, "|")?;

                    // is hrule
                    if build_vec[row_ind].is_empty() {
                        for (i, val) in col_widths.iter().enumerate() {
                            // + 2 to account for buffer around cells
                            for _ in 0..(*val + 2) {
                                write!(self, "-")?;
                            }

                            if i == inner.cols {
                                write!(self, "|")?;
                            } else {
                                write!(self, "+")?;
                            }
                        }
                    } else {
                        for col_ind in 0..inner.cols {
                            let cell = build_vec[row_ind].get(col_ind);
                            let diff;

                            // left buffer
                            write!(self, " ")?;
                            if let Some(strang) = cell {
                                diff = col_widths[col_ind] - strang.len();
                                write!(self, "{strang}")?;
                            } else {
                                diff = col_widths[col_ind];
                            };

                            for _ in 0..diff {
                                write!(self, " ")?;
                            }

                            // right buffer + ending
                            write!(self, " |")?;
                        }
                    }
                    write!(self, "\n")?;
                }
            }

            Expr::TableRow(_) => {
                unreachable!("handled by Expr::Table")
            }
            Expr::TableCell(inner) => {
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
            }
            Expr::Emoji(inner) => {
                write!(self, "{}", inner.mapped_item)?;
            }
        }

        Ok(())
    }
}

impl<'a, 'buf> fmt::Write for Org<'a, 'buf> {
    fn write_str(&mut self, s: &str) -> Result {
        dbg!(s);
        if self.indentation_level > 0 {
            for chunk in s.split_inclusive('\n') {
                if self.on_newline {
                    for _ in 0..self.indentation_level {
                        self.buf.write_str("  ")?;
                    }
                }
                self.on_newline = chunk.ends_with('\n');
                self.buf.write_str(s)?;
            }

            // allows us to manually trigger re-indentation
            // used in Table
            // HACK

            Ok(())
        } else {
            self.buf.write_str(s)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_org_export() -> Result {
        let mut out_str = String::new();
        Org::export_buf(
            r"** one two
three
*four*

",
            &mut out_str,
        )?;

        assert_eq!(
            out_str,
            r"** one two
three *four*

"
        );
        Ok(())
    }

    #[test]
    fn fancy_list_export() -> Result {
        let a = Org::export(
            r"
    + one two three
    four five six

       + two
    + three
    + four
    +five
",
        )?;

        assert_eq!(
            a,
            r"
- one two three
four five six

- two
- three
- four
+five
"
        );

        Ok(())
    }

    #[test]
    fn test_link_export() -> Result {
        let mut out = String::new();
        Org::export_buf("[[https://swag.org][meowww]]", &mut out)?;

        println!("{out}");
        Ok(())
    }

    #[test]
    fn test_beeg() -> Result {
        let mut out = String::new();

        Org::export_buf(
            r"* DONE [#0] *one* two /three/ /four*       :one:two:three:four:
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this

is a different paragraph
id) =
more subcontent

* [#4] separate andy
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph

is a different paragraph
id) =
more subcontent

* [#4] separate andy
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this

is a different paragraph
id) =
more subcontent

* [#4] separate andy
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this

is a different paragraph
id) =
more subcontent

* [#4] separate andy
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
** a
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
* a
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
",
            &mut out,
        )?;

        // println!("{out}");
        Ok(())
    }

    #[test]
    fn less() -> Result {
        let mut out = String::new();
        Org::export_buf(
            r"* [#1] abc :c:
** [#1] descendant headline :a:b:
*** [#2] inherit the tags
** [#3] different level
",
            &mut out,
        )?;

        assert_eq!(
            out,
            r"* [#1] abc :c:
** [#1] descendant headline :a:b:
*** [#2] inherit the tags
** [#3] different level
"
        );
        println!("{out}");
        Ok(())
    }

    #[test]
    fn list_export() -> Result {
        let a = Org::export(
            r"
- one
  - two
                            - three
               - four
                  - five
- six
 - seven
",
        )?;

        println!("{a}");
        assert_eq!(
            a,
            r"
- one
  - two
    - three
    - four
      - five
- six
  - seven
"
        );

        Ok(())
    }

    #[test]
    fn basic_list_export() -> Result {
        let a = Org::export(
            r"
- one
  - two
- three
 - four
- five
 - six
   - seven
- eight
",
        )?;

        println!("{a}");
        assert_eq!(
            a,
            r"
- one
  - two
- three
  - four
- five
  - six
    - seven
- eight
"
        );

        Ok(())
    }

    #[test]
    fn list_words() -> Result {
        let _b = Org::export_buf("ine", &mut String::new())?;
        let a: String = Org::export(
            r"
1. item 1
   abcdef

   next one two three four five

   more thangs more thangs more thangs
   more thangs

2. [X] item 2
   - aome tag :: item 2.1
",
        )?;

        println!("{a}");

        // TODO: whitespace handling is super janky atm.
        // can't even test output properly caudse whitespace is inserted into
        // blanklines, and emacs removes trailing whitespace

        //         assert_eq!(
        //             a,
        //             r"

        // 1. item 1    abcdef

        //   next one two three four five

        //   more thangs more thangs more thangs    more thangs
        // 2. [X] item 2
        //   - aome tag :: item 2.1
        // "
        //         );

        Ok(())
    }

    #[test]
    fn table_export() -> Result {
        let a = Org::export(
            r"
|one|two|
|three|four|
|five|six|seven|
|eight
",
        )?;

        assert_eq!(
            a,
            r"
| one   | two  |       |
| three | four |       |
| five  | six  | seven |
| eight |      |       |
"
        );

        Ok(())
    }

    #[test]
    fn table_export_hrule() -> Result {
        let a = Org::export(
            r"
|one|two|
|-
|three|four|
|five|six|seven|
|eight
|-
|swagg|long the
|okay| _underline_| ~fake| _fake|
",
        )?;

        println!("{a}");
        assert_eq!(
            a,
            r"
| one   | two          |        |        |
|-------+--------------+--------+--------+
| three | four         |        |        |
| five  | six          | seven  |        |
| eight |              |        |        |
|-------+--------------+--------+--------+
| swagg | long the     |        |        |
| okay  |  _underline_ |  ~fake |  _fake |
"
        );
        // println!("{a}");

        Ok(())
    }

    #[test]
    fn indented_table() -> Result {
        let a = Org::export(
            r"
- zero
    |one|two|
    |-
    |three|four|
    |five|six|seven|
    |eight
    |-
    |swagg|long the
    |okay| _underline_| ~fake| _fake|
- ten
",
        )?;

        assert_eq!(
            a,
            r"
- zero
  | one   | two          |        |        |
  |-------+--------------+--------+--------+
  | three | four         |        |        |
  | five  | six          | seven  |        |
  | eight |              |        |        |
  |-------+--------------+--------+--------+
  | swagg | long the     |        |        |
  | okay  |  _underline_ |  ~fake |  _fake |
- ten
"
        );

        Ok(())
    }

    #[test]
    fn proper_list_indent() -> Result {
        let a = Org::export(
            r"
- one
- four
  - one
  - two
",
        )?;

        assert_eq!(
            a,
            r"
- one
- four
  - one
  - two
"
        );
        Ok(())
    }

    #[test]
    fn heading_list_not() -> Result {
        let a = Org::export(
            r"
- one
- four
* one
",
        )?;

        // make sure * one is not interpreted as another element of the list,
        // instead as a separate heading (if it was another element, we'd have three -'s
        // )
        assert_eq!(
            a,
            r"
- one
- four
* one
"
        );
        Ok(())
    }

    #[test]
    fn proper_link() -> Result {
        let a = Org::export(r"[[abc][one]]")?;

        assert_eq!(
            a,
            r"[[abc][one]]
"
        );

        Ok(())
    }

    #[test]
    fn link_odd() -> Result {
        let a = Org::export("[aayyyy][one]]")?;
        assert_eq!(
            a,
            r"[aayyyy][one]]
"
        );
        Ok(())
    }
}
