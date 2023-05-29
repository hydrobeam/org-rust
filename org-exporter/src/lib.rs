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
}

impl<'a, T: fmt::Write> Exporter<'a, T> for Org<'a, T> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error> {
        let mut obj = Org {
            buf: String::new(),
            pool: parse_org(input),
            indentation_level: 0,
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
                    write!(self.buf, "*")?;
                }
                write!(self.buf, " ")?;

                if let Some(keyword) = inner.keyword {
                    write!(self.buf, "{keyword} ")?;
                }

                if let Some(priority) = &inner.priority {
                    write!(self.buf, "[#")?;
                    match priority {
                        Priority::A => write!(self.buf, "A")?,
                        Priority::B => write!(self.buf, "B")?,
                        Priority::C => write!(self.buf, "C")?,
                        Priority::Num(num) => write!(self.buf, "{}", num)?,
                    };
                    write!(self.buf, "] ")?;
                }

                if let Some(title) = &inner.title {
                    for id in title {
                        self.export_rec(id)?;
                    }
                }

                // fn tag_search<T: Write>(loc: NodeID, pool: &NodePool, self.buf: &mut T) -> Result {
                //     if let Expr::Heading(loc) = &pool[loc].obj {
                //         if let Some(sub_tags) = loc.tags.as_ref() {
                //             for thang in sub_tags.iter().rev() {
                //                 match thang {
                //                     Tag::Raw(val) => write!(self.buf, ":{val}")?,
                //                     Tag::Loc(id) => {
                //                         tag_search(*id, pool, self.buf)?;
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
                        write!(self.buf, " {valid_out}:")?;
                    }
                }

                writeln!(self.buf)?;

                if let Some(children) = &inner.children {
                    for id in children {
                        self.export_rec(id)?;
                    }
                }
            }
            Expr::Block(inner) => {
                let val: &str = inner.kind.into();
                write!(self.buf, "#+begin_{val}")?;
                if let Some(params) = inner.parameters {
                    write!(self.buf, " {}", params)?;
                }
                write!(self.buf, "\n")?;
                match &inner.contents {
                    BlockContents::Greater(children) => {
                        for id in children {
                            self.export_rec(id)?;
                        }
                        writeln!(self.buf)?;
                    }
                    BlockContents::Lesser(cont) => {
                        writeln!(self.buf, "{cont}")?;
                    }
                }
                write!(self.buf, "#+end_{val}\n")?;
            }
            Expr::RegularLink(inner) => {
                write!(self.buf, "[")?;
                write!(self.buf, "[{}]", inner.path)?;
                if let Some(children) = &inner.description {
                    write!(self.buf, "[")?;
                    for id in children {
                        self.export_rec(id)?;
                    }
                    write!(self.buf, "]")?;
                }
                write!(self.buf, "]")?;
            }

            Expr::Paragraph(inner) => {
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                writeln!(self.buf)?;
            }

            Expr::Italic(inner) => {
                write!(self.buf, "/")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self.buf, "/")?;
            }
            Expr::Bold(inner) => {
                write!(self.buf, "*")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self.buf, "*")?;
            }
            Expr::StrikeThrough(inner) => {
                write!(self.buf, "+")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self.buf, "+")?;
            }
            Expr::Underline(inner) => {
                write!(self.buf, "_")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self.buf, "_")?;
            }
            Expr::BlankLine => {
                writeln!(self.buf)?;
            }
            Expr::SoftBreak => {
                write!(self.buf, " ")?;
            }
            Expr::Plain(inner) => {
                write!(self.buf, "{inner}")?;
            }
            Expr::MarkupEnd(inner) => {
                unreachable!()
            }
            Expr::Verbatim(inner) => {
                write!(self.buf, "={}=", inner.0)?;
            }
            Expr::Code(inner) => {
                write!(self.buf, "~{}~", inner.0)?;
            }
            Expr::Comment(inner) => {
                writeln!(self.buf, "# {}", inner.0)?;
            }
            Expr::InlineSrc(inner) => {
                write!(self.buf, "src_{}", inner.lang)?;
                if let Some(args) = inner.headers {
                    write!(self.buf, "[{args}]")?;
                }
                write!(self.buf, "{{{}}}", inner.body)?;
            }
            Expr::Keyword(inner) => {
                writeln!(self.buf, "#+{}: {}", inner.key, inner.val)?;
            }
            Expr::LatexEnv(inner) => {
                write!(
                    self.buf,
                    "\\begin{{{0}}}\n{1}\n\\end{{{0}}}\n",
                    inner.name, inner.contents
                )?;
            }
            Expr::LatexFragment(inner) => match inner {
                LatexFragment::Command { name, contents } => {
                    write!(self.buf, "\\{name}")?;
                    if let Some(command_cont) = contents {
                        write!(self.buf, "{{{command_cont}}}")?;
                    }
                }
                LatexFragment::Display(inner) => {
                    write!(self.buf, "\\[{inner}\\]")?;
                }
                LatexFragment::Inline(inner) => {
                    write!(self.buf, "\\({inner}\\)")?;
                }
            },
            Expr::Item(inner) => {
                match inner.bullet {
                    BulletKind::Unordered => {
                        write!(self.buf, "-")?;
                    }
                    BulletKind::Ordered(counterkind) => match counterkind {
                        CounterKind::Letter(lettre) => {
                            write!(self.buf, "{}", lettre as char)?;
                        }
                        CounterKind::Number(num) => {
                            write!(self.buf, "{num}")?;
                        }
                    },
                }
                write!(self.buf, " ")?;

                if let Some(check) = &inner.check_box {
                    let val: &str = check.into();
                    write!(self.buf, "[{val}] ")?;
                }

                if let Some(tag) = inner.tag {
                    write!(self.buf, "{tag} :: ")?;
                }

                for id in &inner.children {
                    self.export_rec(id)?;
                    write!(self.buf, "  ")?;
                }
            }
            Expr::PlainList(inner) => {
                for id in &inner.children {
                    self.export_rec(id)?;
                }
            }
            Expr::PlainLink(inner) => {
                write!(self.buf, "{}:{}", inner.protocol, inner.path)?;
            }
            Expr::Entity(inner) => {
                write!(self.buf, "{}", inner.mapped_item)?;
            }
            Expr::Table(inner) => {
                for id in &inner.children {
                    self.export_rec(id)?;
                }
            }

            Expr::TableRow(inner) => {
                match inner {
                    TableRow::Standard(stans) => {
                        write!(self.buf, "|")?;
                        for id in stans {
                            self.export_rec(id)?;
                        }
                    }
                    TableRow::Rule => {
                        // TODO: figure out alignment
                        write!(self.buf, "|-")?;
                    }
                }
                writeln!(self.buf)?;
            }
            Expr::TableCell(inner) => {
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self.buf, "|")?;
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

impl<'a, T: fmt::Write> Org<'a, T> {}

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
        Ok(())
    }
}
