pub mod types;

use core::fmt;
use core::fmt::Result;
use core::fmt::Write;

use org_parser::element::{BlockContents, BulletKind, CounterKind, Priority, TableRow, Tag};
use org_parser::node_pool::{NodeID, NodePool};
use org_parser::object::LatexFragment;
use org_parser::parse_org;
use org_parser::types::Expr;
use types::Exporter;

pub struct Org<'a, T: fmt::Write> {
    buf: T,
    pool: NodePool<'a>,
    indentation_level: u8,
    on_newline: bool,
}

impl<'a, T: fmt::Write> Exporter<'a, T> for Org<'a, T> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error> {
        let mut obj = Org {
            buf: String::new(),
            pool: parse_org(input),
            indentation_level: 0,
            on_newline: false,
        };

        obj.export_rec(&obj.pool.root_id())?;
        Ok(obj.buf)
    }

    fn export_buf<'inp, 'buf>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error> {
        let mut obj = Org {
            buf,
            pool: parse_org(input),
            indentation_level: 0,
            on_newline: false,
        };

        obj.export_rec(&obj.pool.root_id())?;
        Ok(obj.buf)
    }

    fn export_rec(&mut self, node_id: &NodeID) -> Result {
        match &self.pool()[*node_id].obj.clone() {
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
                        Priority::Num(num) => write!(self, "{}", num)?,
                    };
                    write!(self, "] ")?;
                }

                if let Some(title) = &inner.title {
                    for id in title {
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
                            Tag::Raw(val) => write!(valid_out, ":{val}")?,
                            Tag::Loc(id) => {
                                // tag_search(*id, pool, &mut valid_out)?;
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
                        self.export_rec(id)?;
                    }
                }
            }
            Expr::Block(inner) => {
                let val: &str = inner.kind.into();
                write!(self, "#+begin_{val}")?;
                if let Some(params) = inner.parameters {
                    write!(self, " {}", params)?;
                }
                write!(self, "\n")?;
                match &inner.contents {
                    BlockContents::Greater(children) => {
                        for id in children {
                            self.export_rec(id)?;
                        }
                        writeln!(self)?;
                    }
                    BlockContents::Lesser(cont) => {
                        writeln!(self, "{cont}")?;
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
                writeln!(self)?;
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
                writeln!(self)?;
            }
            Expr::SoftBreak => {
                write!(self, " ")?;
            }
            Expr::Plain(inner) => {
                write!(self, "{inner}")?;
            }
            Expr::MarkupEnd(inner) => {
                unreachable!()
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
                writeln!(self, "#+{}: {}", inner.key, inner.val)?;
            }
            Expr::LatexEnv(inner) => {
                write!(
                    self,
                    "\\begin{{{0}}}\n{1}\n\\end{{{0}}}\n",
                    inner.name, inner.contents
                )?;
            }
            Expr::LatexFragment(inner) => match inner {
                LatexFragment::Command { name, contents } => {
                    write!(self, "\\{name}")?;
                    if let Some(command_cont) = contents {
                        write!(self, "{{{command_cont}}}")?;
                    }
                }
                LatexFragment::Display(inner) => {
                    write!(self, "\\[{inner}\\]")?;
                }
                LatexFragment::Inline(inner) => {
                    write!(self, "\\({inner}\\)")?;
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
                for id in &inner.children {
                    self.export_rec(id)?;
                }
            }

            Expr::TableRow(inner) => {
                match inner {
                    TableRow::Standard(stans) => {
                        write!(self, "|")?;
                        for id in stans {
                            self.export_rec(id)?;
                        }
                    }
                    TableRow::Rule => {
                        // TODO: figure out alignment
                        write!(self, "|-")?;
                    }
                }
                writeln!(self)?;
            }
            Expr::TableCell(inner) => {
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "|")?;
            }
        }

        Ok(())
    }

    fn buf(&mut self) -> &mut T {
        &mut self.buf
    }

    fn pool(&self) -> &NodePool<'a> {
        &self.pool
    }
}

impl<'a, T: fmt::Write> Write for Org<'a, T> {
    fn write_str(&mut self, s: &str) -> Result {
        if self.indentation_level > 0 {
            for chunk in s.split_inclusive('\n') {
                if self.on_newline {
                    for _ in 0..self.indentation_level {
                        self.buf().write_str("  ")?;
                    }
                }

                self.on_newline = chunk.ends_with('\n');
                self.buf().write_str(s)?;
            }

            Ok(())
        } else {
            self.buf().write_str(s)
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

    //     #[test]
    //     fn test_list_export() -> Result {
    //         let mut out = String::new();
    //         export_org(
    //             r"
    // + one two three
    // four five six

    //    + two
    // + three
    // + four
    // +five
    // ",
    //             &mut out,
    //         )?;

    //         println!("{out}");
    //         Ok(())
    //     }

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
        let a = Org::<String>::export(
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
        let a = Org::<String>::export(
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
        let a = Org::<String>::export(
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
}
