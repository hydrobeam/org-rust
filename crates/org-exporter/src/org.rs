use std::borrow::Cow;
use std::fmt;
use std::fmt::Write;

use crate::include::include_handle;
use crate::org_macros::macro_handle;
use crate::types::{Exporter, ExporterInner};
use org_parser::element::{Block, BulletKind, CounterKind, Priority, TableRow, Tag};
use org_parser::object::{LatexFragment, PlainOrRec};

use org_parser::{parse_org, Expr, NodeID, Parser};

/// Org-Mode Content Exporter
///
/// This backend might seem a little unncessary, but it's fairly useful as a sanity check
/// for the parser.
///
/// It also carries out some modifications to the source such as prettifying tables and resolving
/// macros
pub struct Org<'buf> {
    buf: &'buf mut dyn fmt::Write,
    indentation_level: u8,
    on_newline: bool,
}

impl<'buf> Exporter<'buf> for Org<'buf> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error> {
        let mut buf = String::new();
        Org::export_buf(input, &mut buf)?;
        Ok(buf)
    }

    fn export_buf<'inp, T: fmt::Write>(input: &'inp str, buf: &'buf mut T) -> fmt::Result {
        let parsed = parse_org(input);
        Org::export_tree(&parsed, buf)
    }

    fn export_tree<'inp, T: fmt::Write>(parsed: &Parser, buf: &'buf mut T) -> fmt::Result {
        let mut obj = Org {
            buf,
            indentation_level: 0,
            on_newline: false,
        };

        obj.export_rec(&parsed.pool.root_id(), &parsed)
    }
}

impl<'buf> ExporterInner<'buf> for Org<'buf> {
    fn export_macro_buf<'inp, T: fmt::Write>(input: &'inp str, buf: &'buf mut T) -> fmt::Result {
        let parsed = org_parser::parse_macro_call(input);

        let mut obj = Org {
            buf,
            indentation_level: 0,
            on_newline: false,
        };

        obj.export_rec(&parsed.pool.root_id(), &parsed)
    }

    fn export_rec(&mut self, node_id: &NodeID, parser: &Parser) -> fmt::Result {
        match &parser.pool[*node_id].obj {
            Expr::Root(inner) => {
                for id in inner {
                    self.export_rec(id, parser)?;
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
                        self.export_rec(id, parser)?;
                    }
                }

                // fn tag_search<T: Write>(loc: NodeID, pool: &NodePool, self: &mut T) -> Result {
                //     if let Expr::Heading(loc) = &pool[loc].obj {
                //         if let Some(sub_tags) = loc.tags.as_ref() {
                //             for thang in sub_tags.iter().rev() {
                //                 match thang {
                //                     Tag::Raw(val) => write!(self, ":{val}")?,
                //                     Tag::Loc(id, parser) => {
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

                writeln!(self)?;

                if let Some(children) = &inner.children {
                    for id in children {
                        self.export_rec(id, parser)?;
                    }
                }
            }
            Expr::Block(inner) => {
                match inner {
                    // Greater Blocks
                    Block::Center {
                        parameters,
                        contents,
                    } => {
                        write!(self, "#+begin_center")?;
                        for (key, val) in parameters {
                            write!(self, " :{} {}", key, val)?;
                        }
                        writeln!(self)?;
                        for id in contents {
                            self.export_rec(id, parser)?;
                        }
                        writeln!(self, "#+end_center")?;
                    }
                    Block::Quote {
                        parameters,
                        contents,
                    } => {
                        writeln!(self, "#+begin_quote")?;
                        for (key, val) in parameters {
                            write!(self, " :{} {}", key, val)?;
                        }
                        writeln!(self)?;
                        for id in contents {
                            self.export_rec(id, parser)?;
                        }
                        writeln!(self, "#+end_quote")?;
                    }
                    Block::Special {
                        parameters,
                        contents,
                        name,
                    } => {
                        write!(self, "#+begin_{name}")?;
                        for (key, val) in parameters {
                            write!(self, " :{} {}", key, val)?;
                        }
                        writeln!(self)?;
                        for id in contents {
                            self.export_rec(id, parser)?;
                        }
                        writeln!(self, "#+end_{name}")?;
                    }

                    // Lesser blocks
                    Block::Comment {
                        parameters,
                        contents,
                    } => {
                        write!(self, "#+begin_comment")?;
                        for (key, val) in parameters {
                            write!(self, " :{} {}", key, val)?;
                        }
                        write!(self, "\n{contents}")?;
                        writeln!(self, "#+end_comment")?;
                    }
                    Block::Example {
                        parameters,
                        contents,
                    } => {
                        write!(self, "#+begin_example")?;
                        for (key, val) in parameters {
                            write!(self, " :{} {}", key, val)?;
                        }
                        write!(self, "\n{contents}")?;
                        writeln!(self, "#+end_example")?;
                    }
                    Block::Export {
                        backend,
                        parameters,
                        contents,
                    } => {
                        let back = if let Some(word) = backend { word } else { "" };
                        write!(self, "#+begin_export {}", back)?;
                        for (key, val) in parameters {
                            write!(self, " :{} {}", key, val)?;
                        }
                        write!(self, "\n{contents}")?;
                        writeln!(self, "#+end_export")?;
                    }
                    Block::Src {
                        language,
                        parameters,
                        contents,
                    } => {
                        let lang = if let Some(word) = language { word } else { "" };
                        write!(self, "#+begin_export {}", lang)?;
                        for (key, val) in parameters {
                            write!(self, " :{} {}", key, val)?;
                        }
                        write!(self, "\n{contents}")?;
                        writeln!(self, "#+end_src")?;
                    }
                    Block::Verse {
                        parameters,
                        contents,
                    } => {
                        write!(self, "#+begin_verse")?;
                        for (key, val) in parameters {
                            write!(self, " :{} {}", key, val)?;
                        }
                        write!(self, "\n{contents}")?;
                        writeln!(self, "#+end_verse")?;
                    }
                }
            }
            Expr::RegularLink(inner) => {
                write!(self, "[")?;
                write!(self, "[{}]", inner.path.obj)?;
                if let Some(children) = &inner.description {
                    write!(self, "[")?;
                    for id in children {
                        self.export_rec(id, parser)?;
                    }
                    write!(self, "]")?;
                }
                write!(self, "]")?;
            }

            Expr::Paragraph(inner) => {
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                writeln!(self)?;
            }

            Expr::Italic(inner) => {
                write!(self, "/")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                write!(self, "/")?;
            }
            Expr::Bold(inner) => {
                write!(self, "*")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                write!(self, "*")?;
            }
            Expr::StrikeThrough(inner) => {
                write!(self, "+")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                write!(self, "+")?;
            }
            Expr::Underline(inner) => {
                write!(self, "_")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                write!(self, "_")?;
            }
            Expr::BlankLine => {
                writeln!(self)?;
            }
            Expr::SoftBreak => {
                write!(self, " ")?;
            }
            Expr::LineBreak => {
                write!(self, r#"\\"#)?;
            }
            Expr::HorizontalRule => {
                writeln!(self, "-----")?;
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
                writeln!(self, "# {}", inner.0)?;
            }
            Expr::InlineSrc(inner) => {
                write!(self, "src_{}", inner.lang)?;
                if let Some(args) = inner.headers {
                    write!(self, "[{args}]")?;
                }
                write!(self, "{{{}}}", inner.body)?;
            }
            Expr::Keyword(inner) => {
                if inner.key.to_ascii_lowercase() == "include" {
                    // FIXME: proper error handling
                    include_handle(inner.val, self).unwrap();
                }
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
                    write!(self, r#"\{name}"#)?;
                    if let Some(command_cont) = contents {
                        write!(self, "{{{command_cont}}}")?;
                    }
                }
                LatexFragment::Display(inner) => {
                    write!(self, r"\[{inner}\]")?;
                }
                LatexFragment::Inline(inner) => {
                    write!(self, r#"\({inner}\)"#)?;
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

                if let Some(counter_set) = inner.counter_set {
                    write!(self, "[@{counter_set}]")?;
                }

                if let Some(check) = &inner.check_box {
                    let val: &str = check.into();
                    write!(self, "[{val}] ")?;
                }

                if let Some(tag) = inner.tag {
                    write!(self, "{tag} :: ")?;
                }

                self.indentation_level += 1;
                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }
                self.indentation_level -= 1;
                if self.indentation_level == 0 {
                    self.on_newline = false;
                }
            }
            Expr::PlainList(inner) => {
                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }
            }
            Expr::PlainLink(inner) => {
                write!(self, "[[{}:{}]]", inner.protocol, inner.path)?;
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
                    match &parser.pool[*id].obj {
                        Expr::TableRow(row) => {
                            let mut row_vec = vec![];
                            match &row {
                                TableRow::Standard(stans) => {
                                    for id in stans {
                                        let mut cell_buf = String::new();
                                        let mut new_obj = Org {
                                            buf: &mut cell_buf,
                                            indentation_level: self.indentation_level,
                                            on_newline: self.on_newline,
                                        };
                                        new_obj.export_rec(id, parser)?;
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
                    for row in &build_vec {
                        curr_max = curr_max.max(row.get(col_ind).map_or_else(|| 0, |v| v.len()));
                    }
                    col_widths.push(curr_max);
                }

                for row in &build_vec {
                    write!(self, "|")?;

                    // is hrule
                    if row.is_empty() {
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
                        for (col_ind, col_width) in col_widths.iter().enumerate() {
                            let cell = row.get(col_ind);
                            let diff;

                            // left buffer
                            write!(self, " ")?;
                            if let Some(strang) = cell {
                                diff = col_width - strang.len();
                                write!(self, "{strang}")?;
                            } else {
                                diff = *col_width;
                            };

                            for _ in 0..diff {
                                write!(self, " ")?;
                            }

                            // right buffer + ending
                            write!(self, " |")?;
                        }
                    }
                    writeln!(self)?;
                }
            }

            Expr::TableRow(_) => {
                unreachable!("handled by Expr::Table")
            }
            Expr::TableCell(inner) => {
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
            }
            Expr::Emoji(inner) => {
                write!(self, "{}", inner.mapped_item)?;
            }
            Expr::Superscript(inner) => match &inner.0 {
                PlainOrRec::Plain(inner) => {
                    write!(self, "^{{{inner}}}")?;
                }
                PlainOrRec::Rec(inner) => {
                    write!(self, "^{{")?;
                    for id in inner {
                        self.export_rec(id, parser)?;
                    }

                    write!(self, "}}")?;
                }
            },
            Expr::Subscript(inner) => match &inner.0 {
                PlainOrRec::Plain(inner) => {
                    write!(self, "_{{{inner}}}")?;
                }
                PlainOrRec::Rec(inner) => {
                    write!(self, "_{{")?;
                    for id in inner {
                        self.export_rec(id, parser)?;
                    }

                    write!(self, "}}")?;
                }
            },
            Expr::Target(inner) => {
                write!(self, "<<{}>>", inner.0)?;
            }
            Expr::Macro(macro_call) => {
                if let Ok(macro_contents) = macro_handle(parser, macro_call) {
                    match macro_contents {
                        Cow::Owned(p) => {
                            Org::export_macro_buf(&p, self)?;
                        }
                        Cow::Borrowed(r) => {
                            write!(self, "{r}")?;
                        }
                    }
                }
            }
            Expr::Drawer(inner) => {
                writeln!(self, ":{}:", inner.name)?;
                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }
                writeln!(self, ":end:")?;
            }
            Expr::ExportSnippet(inner) => {
                if inner.backend == "org" {
                    write!(self, "{}", inner.contents)?;
                }
            }
            Expr::Affiliated(_) => {}
            Expr::MacroDef(_) => {}
            Expr::FootnoteDef(inner) => {
                write!(self, r"[fn:{}] ", inner.label)?;

                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }
            }
            Expr::FootnoteRef(inner) => {
                write!(self, r"[fn:")?;
                if let Some(label) = inner.label {
                    write!(self, "{label}")?;
                }
                if let Some(descr) = &inner.children {
                    write!(self, ":")?;
                    for id in descr {
                        self.export_rec(id, parser)?;
                    }
                }
                write!(self, "]")?;
            }
        }

        Ok(())
    }

    fn backend_name() -> &'static str {
        "org"
    }
}

impl<'buf> fmt::Write for Org<'buf> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
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

    use fmt::Result;
    use pretty_assertions::assert_eq;

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

    #[test]
    fn superscript() -> Result {
        let a = Org::export(r"sample_text^{\gamma}")?;
        assert_eq!(
            a,
            r"sample_{text}^{γ}
"
        );

        let b = Org::export(
            r"sample_text^bunchoftextnowhite!,lkljas
 after",
        )?;

        assert_eq!(
            b,
            r"sample_{text}^{bunchoftextnowhite}!,lkljas  after
"
        );

        let c = Org::export(r"nowhere ^texto")?;

        assert_eq!(
            c,
            r"nowhere ^texto
"
        );

        Ok(())
    }

    #[test]
    fn subscript() -> Result {
        let a = Org::export(r"sample_text_{\gamma}")?;
        assert_eq!(
            a,
            r"sample_{text}_{γ}
"
        );

        let b = Org::export(
            r"sample_{text}_bunchoftextnowhite!,lkljas
 after",
        )?;

        assert_eq!(
            b,
            r"sample_{text}_{bunchoftextnowhite}!,lkljas  after
"
        );

        let c = Org::export(r"nowhere _texto")?;

        assert_eq!(
            c,
            r"nowhere _texto
"
        );

        Ok(())
    }

    #[test]
    fn plain_link() -> Result {
        let a = Org::export("https://cool.com abc rest")?;

        assert_eq!(
            a,
            "[[https://cool.com]] abc rest
"
        );
        Ok(())
    }

    #[test]
    fn newline_literal_markup() -> Result {
        let a = Org::export(
            r"- test =if ~literal $interpreters \[handle newline \(properly {{{in(a lists
- text that isn't disappearing!
",
        )?;

        assert_eq!(
            a,
            r"- test =if ~literal $interpreters \[handle newline \(properly {{{in(a lists
- text that isn't disappearing!
"
        );

        Ok(())
    }

    #[test]
    fn lblock_plus_list() -> Result {
        let a = Org::export(
            r"
-
   #+begin_src


hiiiiiiiiiiiiiiiiiii

meowwwwwwwwww
   #+end_src

-
",
        )?;
        println!("{a}");

        Ok(())
    }

    #[test]
    fn markup_enclosed_in_bracks() -> Result {
        let a = Org::export(r"[_enclosed text here_]")?;

        assert_eq!(
            a,
            "[_enclosed text here_]
"
        );

        Ok(())
    }

    #[test]
    fn drawer() -> Result {
        let a = Org::export(
            r"
:NAME:

*words*

||||abcds|



* abc one two three

four
:end:
",
        )?;
        assert_eq!(
            a,
            r"
:NAME:

*words*

|  |  |  | abcds |



* abc one two three

four
:end:

"
        );

        Ok(())
    }
}
