use core::fmt;
use core::fmt::Result;

use crate::types::Exporter;
use org_parser::element::{BlockContents, BulletKind, CounterKind, Priority, TableRow, Tag};
use org_parser::node_pool::{NodeID, NodePool};
use org_parser::object::LatexFragment;
use org_parser::parse_org;
use org_parser::types::Expr;

pub struct Org<'a> {
    pool: NodePool<'a>,
    indentation_level: u8,
    on_newline: bool,
}

impl<'a> Exporter<'a> for Org<'a> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error> {
        let mut buf = String::new();
        let mut obj = Org {
            pool: parse_org(input).pool,
            indentation_level: 0,
            on_newline: false,
        };

        obj.export_rec(&obj.pool.root_id(), &mut buf)?;
        Ok(buf)
    }

    fn export_buf<'inp, 'buf, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error> {
        let mut obj = Org {
            pool: parse_org(input).pool,
            indentation_level: 0,
            on_newline: false,
        };

        obj.export_rec(&obj.pool.root_id(), buf)?;
        Ok(buf)
    }

    fn export_rec(&mut self, node_id: &NodeID, buf: &mut dyn fmt::Write) -> Result {
        match &self.pool()[*node_id].obj.clone() {
            Expr::Root(inner) => {
                for id in inner {
                    self.export_rec(id, buf)?;
                }
            }
            Expr::Heading(inner) => {
                for _ in 0..inner.heading_level.into() {
                    self.write(buf, "*")?;
                }
                self.write(buf, " ")?;

                if let Some(keyword) = inner.keyword {
                    self.write(buf, &format!("{keyword} "))?;
                }

                if let Some(priority) = &inner.priority {
                    self.write(buf, "[#")?;
                    match priority {
                        Priority::A => self.write(buf, "A")?,
                        Priority::B => self.write(buf, "B")?,
                        Priority::C => self.write(buf, "C")?,
                        Priority::Num(num) => self.write(buf, &format!("{num}"))?,
                    };
                    self.write(buf, "] ")?;
                }

                if let Some(title) = &inner.title {
                    for id in title {
                        self.export_rec(id, buf)?;
                    }
                }

                // fn tag_search<T: Write>(loc: NodeID, pool: &NodePool, self: &mut T) -> Result {
                //     if let Expr::Heading(loc) = &pool[loc].obj {
                //         if let Some(sub_tags) = loc.tags.as_ref() {
                //             for thang in sub_tags.iter().rev() {
                //                 match thang {
                //                     Tag::Raw(val) => self.write(buf, ":{val}")?,
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
                            Tag::Raw(val) => self.write(&mut valid_out, &format!(":{val}"))?,
                            Tag::Loc(_id) => {
                                // do nothing with it
                            }
                        }
                    }
                    // handles the case where a parent heading has no tags
                    if !valid_out.is_empty() {
                        self.write(buf, &format!(" {valid_out}:"))?;
                    }
                }

                self.write(buf, "\n")?;

                if let Some(children) = &inner.children {
                    for id in children {
                        self.export_rec(id, buf)?;
                    }
                }
            }
            Expr::Block(inner) => {
                let val: &str = inner.kind.into();
                self.write(buf, &format!("#+begin_{val}"))?;
                if let Some(params) = inner.parameters {
                    self.write(buf, &format!(" {params}"))?;
                }
                self.write(buf, "\n")?;
                match &inner.contents {
                    BlockContents::Greater(children) => {
                        for id in children {
                            self.export_rec(id, buf)?;
                        }
                        self.write(buf, "\n")?;
                    }
                    BlockContents::Lesser(cont) => {
                        self.write(buf, &format!("{cont}\n"))?;
                    }
                }
                self.write(buf, &format!("#+end_{val}\n"))?;
            }
            Expr::RegularLink(inner) => {
                self.write(buf, "[")?;
                self.write(buf, &format!("[{}]", inner.path))?;
                if let Some(children) = &inner.description {
                    self.write(buf, "[")?;
                    for id in children {
                        self.export_rec(id, buf)?;
                    }
                    self.write(buf, "]")?;
                }
                self.write(buf, "]")?;
            }

            Expr::Paragraph(inner) => {
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "\n")?;
            }

            Expr::Italic(inner) => {
                self.write(buf, "/")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "/")?;
            }
            Expr::Bold(inner) => {
                self.write(buf, "*")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "*")?;
            }
            Expr::StrikeThrough(inner) => {
                self.write(buf, "+")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "+")?;
            }
            Expr::Underline(inner) => {
                self.write(buf, "_")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "_")?;
            }
            Expr::BlankLine => {
                self.write(buf, "\n")?;
            }
            Expr::SoftBreak => {
                self.write(buf, " ")?;
            }
            Expr::Plain(inner) => {
                self.write(buf, &format!("{inner}"))?;
            }
            Expr::Verbatim(inner) => {
                self.write(buf, &format!("={}=", inner.0))?;
            }
            Expr::Code(inner) => {
                self.write(buf, &format!("~{}~", inner.0))?;
            }
            Expr::Comment(inner) => {
                self.write(buf, &format!("# {}\n", inner.0))?;
            }
            Expr::InlineSrc(inner) => {
                self.write(buf, &format!("src_{}", inner.lang))?;
                if let Some(args) = inner.headers {
                    self.write(buf, &format!("[{args}]"))?;
                }
                self.write(buf, &format!("{{{}}}", inner.body))?;
            }
            Expr::Keyword(inner) => {
                self.write(buf, &format!("#+{}: {}", inner.key, inner.val))?;
            }
            Expr::LatexEnv(inner) => {
                self.write(
                    buf,
                    &format!(
                        "\\begin{{{0}}}\n{1}\n\\end{{{0}}}\n",
                        inner.name, inner.contents
                    ),
                )?;
            }
            Expr::LatexFragment(inner) => match inner {
                LatexFragment::Command { name, contents } => {
                    self.write(buf, &format!("\\{name}"))?;
                    if let Some(command_cont) = contents {
                        self.write(buf, &format!("{{{command_cont}}}"))?;
                    }
                }
                LatexFragment::Display(inner) => {
                    self.write(buf, &format!("\\[{inner}\\]"))?;
                }
                LatexFragment::Inline(inner) => {
                    self.write(buf, &format!("\\({inner}\\)"))?;
                }
            },
            Expr::Item(inner) => {
                match inner.bullet {
                    BulletKind::Unordered => {
                        self.write(buf, "-")?;
                    }
                    BulletKind::Ordered(counterkind) => match counterkind {
                        CounterKind::Letter(lettre) => {
                            self.write(buf, &format!("{}.", lettre as char))?;
                        }
                        CounterKind::Number(num) => {
                            self.write(buf, &format!("{num}."))?;
                        }
                    },
                }
                self.write(buf, " ")?;

                if let Some(check) = &inner.check_box {
                    let val: &str = check.into();
                    self.write(buf, &format!("[{val}] "))?;
                }

                if let Some(tag) = inner.tag {
                    self.write(buf, &format!("{tag} :: "))?;
                }

                self.indentation_level += 1;
                for id in &inner.children {
                    self.export_rec(id, buf)?;
                }
                self.indentation_level -= 1;
                if self.indentation_level == 0 {
                    self.on_newline = false;
                }
            }
            Expr::PlainList(inner) => {
                for id in &inner.children {
                    self.export_rec(id, buf)?;
                }
            }
            Expr::PlainLink(inner) => {
                self.write(buf, &format!("{}:{}", inner.protocol, inner.path))?;
            }
            Expr::Entity(inner) => {
                self.write(buf, &format!("{}", inner.mapped_item))?;
            }
            Expr::Table(inner) => {
                let mut build_vec: Vec<Vec<String>> = Vec::with_capacity(inner.rows);
                // HACK: stop the table cells from receiving indentation from newline
                // in lists, manually retrigger it here
                self.write(buf, "")?;

                // set up 2d array
                for id in &inner.children {
                    // TODO: get rid of this clone without refcell
                    // (is it even possible??)
                    match &self.pool()[*id].obj.clone() {
                        Expr::TableRow(row) => {
                            let mut row_vec = vec![];
                            match &row {
                                TableRow::Standard(stans) => {
                                    for id in stans {
                                        let mut cell_buf = String::new();
                                        self.export_rec(id, &mut cell_buf)?;
                                        row_vec.push(cell_buf)
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
                    self.write(buf, "|")?;

                    // is hrule
                    if build_vec[row_ind].is_empty() {
                        for (i, val) in col_widths.iter().enumerate() {
                            // + 2 to account for buffer around cells
                            for _ in 0..(*val + 2) {
                                self.write(buf, "-")?;
                            }

                            if i == inner.cols {
                                self.write(buf, "|")?;
                            } else {
                                self.write(buf, "+")?;
                            }
                        }
                    } else {
                        for col_ind in 0..inner.cols {
                            let cell = build_vec[row_ind].get(col_ind);
                            let diff;

                            // left buffer
                            self.write(buf, " ")?;
                            if let Some(strang) = cell {
                                diff = col_widths[col_ind] - strang.len();
                                self.write(buf, strang)?;
                            } else {
                                diff = col_widths[col_ind];
                            };

                            for _ in 0..diff {
                                self.write(buf, " ")?;
                            }

                            // right buffer + ending
                            self.write(buf, " |")?;
                        }
                    }
                    self.write(buf, "\n")?;
                }
            }

            Expr::TableRow(_) => {
                unreachable!("handled by Expr::Table")
            }
            Expr::TableCell(inner) => {
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
            }
        }

        Ok(())
    }

    fn pool(&self) -> &NodePool<'a> {
        &self.pool
    }

    fn write(&mut self, buf: &mut dyn fmt::Write, s: &str) -> fmt::Result {
        if self.indentation_level > 0 {
            for chunk in s.split_inclusive('\n') {
                if self.on_newline {
                    for _ in 0..self.indentation_level {
                        buf.write_str("  ")?;
                    }
                }
                self.on_newline = chunk.ends_with('\n');
                buf.write_str(s)?;
            }
            // allows us to manually trigger re-indentation
            // used in Table
            // HACK
            if s.is_empty() {
                if self.on_newline {
                    for _ in 0..self.indentation_level {
                        buf.write_str("  ")?;
                    }
                }
                self.on_newline = false;
            }

            Ok(())
        } else {
            buf.write_str(s)
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
}
