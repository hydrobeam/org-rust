use std::collections::BTreeMap;

use core::fmt;

use core::fmt::Result;
use std::fmt::Write;

use latex2mathml::{latex_to_mathml, DisplayStyle};
use org_parser::element::{BlockKind, CheckBox, ListKind};

use crate::types::Exporter;
use org_parser::element::{BlockContents, TableRow};
use org_parser::node_pool::{NodeID, NodePool};
use org_parser::object::{LatexFragment, PathReg, PlainOrRec};
use org_parser::parse_org;
use org_parser::types::Expr;

pub struct Html<'a, 'buf> {
    buf: &'buf mut dyn fmt::Write,
    pool: &'a NodePool<'a>,
    targets: &'a BTreeMap<&'a str, &'a str>,
}

impl<'a, 'buf> Exporter<'a, 'buf> for Html<'a, 'buf> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error> {
        let mut buf = String::new();
        let parsed = parse_org(input);
        let mut obj = Html {
            buf: &mut buf,
            pool: &parsed.pool,
            targets: &parsed.targets,
        };

        obj.export_rec(&obj.pool.root_id())?;
        Ok(buf)
    }

    fn export_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error> {
        let parsed = parse_org(input);
        let mut obj = Html {
            buf,
            pool: &parsed.pool,
            targets: &parsed.targets,
        };

        obj.export_rec(&obj.pool.root_id())?;
        Ok(buf)
    }

    fn export_rec(&mut self, node_id: &NodeID) -> Result {
        match &self.pool[*node_id].obj.clone() {
            Expr::Root(inner) => {
                //                 self.write(
                //                     buf,
                //                     r#"
                // <!doctype html>
                // <html lang="en">

                // <head>
                //     <meta charset="UTF-8" />
                //     <title>Document</title>
                // </head>

                // <body>
                // "#,
                //                 )?;
                for id in inner {
                    self.export_rec(id)?;
                }

                //                 self.write(
                //                     buf,
                //                     r"
                // </body>

                // </html>
                // ",
                //                 )?;
            }
            Expr::Heading(inner) => {
                let heading_number: u8 = inner.heading_level.into();

                if let Some(title) = &inner.title {
                    write!(
                        self,
                        "<h{heading_number} id={}>",
                        self.targets.get(title.0).unwrap(),
                    )?;
                    for id in &title.1 {
                        self.export_rec(id)?;
                    }

                    // must exist if we are a heading
                } else {
                    write!(self, "<h{heading_number}>")?;
                }

                writeln!(self, "</h{heading_number}>")?;

                if let Some(children) = &inner.children {
                    for id in children {
                        self.export_rec(id)?;
                    }
                }
            }
            Expr::Block(inner) => {
                let _val: &str = inner.kind.into();

                match inner.kind {
                    BlockKind::Center => {
                        writeln!(self, "<div class=center>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{cont}")?;
                                }
                            };
                            Ok(())
                        }()?;

                        writeln!(self, "</div>")?;
                    }
                    BlockKind::Quote => {
                        writeln!(self, "<div class=quote>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{cont}")?;
                                }
                            };
                            Ok(())
                        }()?;
                        writeln!(self, "</div>")?;
                    }
                    BlockKind::Special(name) => {
                        writeln!(self, "<div class={name}>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{cont}")?;
                                }
                            };
                            Ok(())
                        }()?;
                        writeln!(self, "</div>")?;
                    }
                    BlockKind::Comment => {}
                    BlockKind::Example => {
                        writeln!(self, "<pre class=example>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{cont}")?;
                                }
                            };
                            Ok(())
                        }()?;
                        writeln!(self, "</pre>")?;
                    }
                    BlockKind::Export => {
                        writeln!(self, "<pre class=example>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{cont}")?;
                                }
                            };
                            Ok(())
                        }()?;
                        writeln!(self, "</pre>")?;
                    }
                    BlockKind::Src => {
                        writeln!(self, "<pre class=src>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{cont}")?;
                                }
                            };
                            Ok(())
                        }()?;
                        writeln!(self, "</pre>")?;
                    }
                    BlockKind::Verse => {
                        writeln!(self, "<pre class=src>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{cont}")?;
                                }
                            };
                            Ok(())
                        }()?;
                        writeln!(self, "</pre>")?;
                    }
                }
            }
            Expr::RegularLink(inner) => {
                let path_link: String = match inner.path {
                    PathReg::PlainLink(a) => a.into(),
                    PathReg::Id(a) => format!("#{}", a),
                    PathReg::CustomId(a) => format!("#{}", a),
                    PathReg::Coderef(_) => todo!(),
                    PathReg::Unspecified(a) => {
                        let mut rita = String::new();
                        for (match_targ, ret) in self.targets.iter() {
                            if match_targ.starts_with(a) {
                                rita = format!("#{}", ret.to_string());
                                break;
                            }
                        }
                        // TODO: how to handle non-existing links
                        rita
                    }
                };
                write!(self, "<a href={}>", path_link)?;

                if let Some(children) = &inner.description {
                    for id in children {
                        self.export_rec(id)?;
                    }
                } else {
                    write!(
                        self,
                        "{}",
                        match inner.path {
                            PathReg::PlainLink(a) => a.into(),
                            PathReg::Id(a) => format!("{a}"),
                            PathReg::CustomId(a) => format!("{a}"),
                            PathReg::Coderef(_) => todo!(),
                            PathReg::Unspecified(a) => format!("{a}"),
                        },
                    )?;
                }
                write!(self, "</a>")?;
            }

            Expr::Paragraph(inner) => {
                writeln!(self, "<p>")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                writeln!(self, "\n</p>")?;
            }

            Expr::Italic(inner) => {
                write!(self, "<em>")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "</em>")?;
            }
            Expr::Bold(inner) => {
                write!(self, "<b>")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "</b>")?;
            }
            Expr::StrikeThrough(inner) => {
                write!(self, "<del>")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "</del>")?;
            }
            Expr::Underline(inner) => {
                write!(self, "<u>")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                write!(self, "</u>")?;
                // write!(self, "<span class=underline>")?;
                // for id in &inner.0 {
                //     self.export_rec(id)?;
                // }
                // write!(self, "</span>")?;
            }
            Expr::BlankLine => {
                // write!(self, "\n")?;
            }
            Expr::SoftBreak => {
                write!(self, " ")?;
            }
            Expr::Plain(inner) => {
                write!(self, "{inner}")?;
            }
            Expr::Verbatim(inner) => {
                write!(self, "<code>{}</code>", inner.0)?;
            }
            Expr::Code(inner) => {
                write!(self, "<code>{}</code>", inner.0)?;
            }
            Expr::Comment(inner) => {
                write!(self, "<!--{}-->", inner.0)?;
            }
            Expr::InlineSrc(inner) => {
                write!(self, "<code class={}>{}</code>", inner.lang, inner.body)?;
                // if let Some(args) = inner.headers {
                //     write!(self, "[{args}]")?;
                // }
                // write!(self, "{{{}}}", inner.body)?;
            }
            Expr::Keyword(_inner) => {
                // write!(self, "#+{}: {}", inner.key, inner.val)?;
            }
            Expr::LatexEnv(inner) => {
                let ret = latex_to_mathml(
                    &format!(
                        r"\begin{{{0}}}
{1}
\end{{{0}}}
",
                        inner.name, inner.contents
                    ),
                    DisplayStyle::Block,
                )
                .unwrap();
                writeln!(self, "{ret}")?;
            }
            Expr::LatexFragment(inner) => match inner {
                LatexFragment::Command { name, contents } => {
                    let mut pot_cont = String::new();
                    write!(pot_cont, "{name}")?;
                    if let Some(command_cont) = contents {
                        write!(pot_cont, "{{{command_cont}}}")?;
                    }
                    write!(
                        self,
                        "{}",
                        &latex_to_mathml(&pot_cont, DisplayStyle::Inline).unwrap(),
                    )?;
                }
                LatexFragment::Display(inner) => {
                    writeln!(
                        self,
                        "{}",
                        &latex_to_mathml(inner, DisplayStyle::Block).unwrap()
                    )?;
                }
                LatexFragment::Inline(inner) => {
                    write!(
                        self,
                        "{}",
                        &latex_to_mathml(inner, DisplayStyle::Inline).unwrap()
                    )?;
                }
            },
            Expr::Item(inner) => {
                let tag_val = if let Some(tag) = inner.tag {
                    format!(" id={tag}")
                } else {
                    "".to_string()
                };

                let class_val = if let Some(check) = &inner.check_box {
                    let val = match check {
                        CheckBox::Intermediate => "trans",
                        CheckBox::Off => "off",
                        CheckBox::On => "on",
                    };
                    format!(" class={val}")
                } else {
                    "".to_string()
                };

                write!(self, "<li{class_val}{tag_val}>")?;

                for id in &inner.children {
                    self.export_rec(id)?;
                }

                writeln!(self, "</li>")?;
            }
            Expr::PlainList(inner) => match inner.kind {
                ListKind::Unordered | ListKind::Descriptive => {
                    writeln!(self, "<ul>")?;
                    for id in &inner.children {
                        self.export_rec(id)?;
                    }
                    writeln!(self, "</ul>")?;
                }
                ListKind::Ordered(_) => {
                    writeln!(self, "<ol>")?;
                    for id in &inner.children {
                        self.export_rec(id)?;
                    }
                    writeln!(self, "</ol>")?;
                }
            },
            Expr::PlainLink(inner) => {
                write!(self, "<a href={0}:{1}>{0}:{1}</a>", inner.protocol, inner.path)?;
            }
            Expr::Entity(inner) => {
                write!(self, "{}", inner.mapped_item)?;
            }
            Expr::Table(inner) => {
                writeln!(self, "<table>")?;

                for id in &inner.children {
                    self.export_rec(id)?;
                }

                writeln!(self, "</table>")?;
            }

            Expr::TableRow(inner) => {
                match inner {
                    TableRow::Rule => { /*skip*/ }
                    TableRow::Standard(stands) => {
                        write!(self, "<tr>")?;
                        for id in stands.iter() {
                            self.export_rec(id)?;
                        }
                        writeln!(self, "</tr>")?;
                    }
                }
            }
            Expr::TableCell(inner) => {
                write!(self, "<td>")?;
                for id in &inner.0 {
                    self.export_rec(id)?;
                }
                writeln!(self, "</td>")?;
            }
            Expr::Emoji(inner) => {
                write!(self, "{}", inner.mapped_item)?;
            }
            Expr::Superscript(inner) => match &inner.0 {
                PlainOrRec::Plain(inner) => {
                    write!(self, "<sup>{inner}</sup>")?;
                }
                PlainOrRec::Rec(inner) => {
                    write!(self, "<sup>")?;
                    for id in inner {
                        self.export_rec(id)?;
                    }

                    write!(self, "</sup>")?;
                }
            },
            Expr::Subscript(inner) => match &inner.0 {
                PlainOrRec::Plain(inner) => {
                    write!(self, "<sub>{inner}</sub>")?;
                }
                PlainOrRec::Rec(inner) => {
                    write!(self, "<sub>")?;
                    for id in inner {
                        self.export_rec(id)?;
                    }

                    write!(self, "</sub>")?;
                }
            },
        }

        Ok(())
    }
}

impl<'a, 'buf> fmt::Write for Html<'_, '_> {
    fn write_str(&mut self, s: &str) -> Result {
        self.buf.write_str(s)
    }
}

mod tests {}
