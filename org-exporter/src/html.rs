use std::collections::BTreeMap;

use core::fmt;

use core::fmt::Result;
use std::fmt::Write;

use latex2mathml::{latex_to_mathml, DisplayStyle};
use org_parser::element::{BlockKind, CheckBox, ListKind};

use crate::types::Exporter;
use org_parser::element::{BlockContents, BulletKind, CounterKind, Priority, TableRow, Tag};
use org_parser::node_pool::{NodeID, NodePool};
use org_parser::object::{LatexFragment, PathReg};
use org_parser::parse_org;
use org_parser::types::Expr;

pub struct Html<'a> {
    pool: NodePool<'a>,
    targets: BTreeMap<&'a str, &'a str>,
}

impl<'a> Exporter<'a> for Html<'a> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error> {
        let mut buf = String::new();
        let parsed = parse_org(input);
        let mut obj = Html {
            pool: parsed.pool,
            targets: parsed.targets,
        };

        obj.export_rec(&obj.pool.root_id(), &mut buf)?;
        Ok(buf)
    }

    fn export_buf<'inp, 'buf, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error> {
        let parsed = parse_org(input);
        let mut obj = Html {
            pool: parsed.pool,
            targets: parsed.targets,
        };

        obj.export_rec(&obj.pool.root_id(), buf)?;
        Ok(buf)
    }

    fn export_rec(&mut self, node_id: &NodeID, buf: &mut dyn fmt::Write) -> Result {
        match &self.pool()[*node_id].obj.clone() {
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
                    self.export_rec(id, buf)?;
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
                    self.write(
                        buf,
                        &format!(
                            "<h{heading_number} id={}>",
                            self.targets.get(title.0).unwrap(),
                        ),
                    )?;
                    for id in &title.1 {
                        self.export_rec(id, buf)?;
                    }

                    // must exist if we are a heading
                } else {
                    self.write(buf, &format!("<h{heading_number}>"))?;
                }

                self.write(buf, &format!("</h{heading_number}>"))?;

                if let Some(children) = &inner.children {
                    for id in children {
                        self.export_rec(id, buf)?;
                    }
                }
            }
            Expr::Block(inner) => {
                let val: &str = inner.kind.into();

                match inner.kind {
                    BlockKind::Center => {
                        self.write(buf, "<div class=center>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id, buf)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    self.write(buf, cont)?;
                                }
                            };
                            Ok(())
                        }()?;

                        self.write(buf, "</div>")?;
                    }
                    BlockKind::Quote => {
                        self.write(buf, "<div class=quote>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id, buf)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    self.write(buf, cont)?;
                                }
                            };
                            Ok(())
                        }()?;
                        self.write(buf, "</div>")?;
                    }
                    BlockKind::Special(name) => {
                        self.write(buf, "<div class={name}>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id, buf)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    self.write(buf, cont)?;
                                }
                            };
                            Ok(())
                        }()?;
                        self.write(buf, "</div>")?;
                    }
                    BlockKind::Comment => {}
                    BlockKind::Example => {
                        self.write(buf, "<pre class=example>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id, buf)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    self.write(buf, cont)?;
                                }
                            };
                            Ok(())
                        }()?;
                        self.write(buf, "</pre>")?;
                    }
                    BlockKind::Export => {
                        self.write(buf, "<pre class=example>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id, buf)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    self.write(buf, cont)?;
                                }
                            };
                            Ok(())
                        }()?;
                        self.write(buf, "</pre>")?;
                    }
                    BlockKind::Src => {
                        self.write(buf, "<pre class=src>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id, buf)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    self.write(buf, cont)?;
                                }
                            };
                            Ok(())
                        }()?;
                        self.write(buf, "</pre>")?;
                    }
                    BlockKind::Verse => {
                        self.write(buf, "<pre class=src>")?;
                        || -> Result {
                            match &inner.contents {
                                BlockContents::Greater(children) => {
                                    for id in children {
                                        self.export_rec(id, buf)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    self.write(buf, cont)?;
                                }
                            };
                            Ok(())
                        }()?;
                        self.write(buf, "</pre>")?;
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
                        if rita.is_empty() {
                            panic!("no matching link");
                        } else {
                            rita
                        }
                    }
                };
                self.write(buf, &format!("<a href={}>", path_link))?;

                if let Some(children) = &inner.description {
                    for id in children {
                        self.export_rec(id, buf)?;
                    }
                } else {
                    self.write(
                        buf,
                        &format!(
                            "{}",
                            match inner.path {
                                PathReg::PlainLink(a) => a.into(),
                                PathReg::Id(a) => format!("{a}"),
                                PathReg::CustomId(a) => format!("{a}"),
                                PathReg::Coderef(_) => todo!(),
                                PathReg::Unspecified(a) => format!("{a}"),
                            }
                        ),
                    )?;
                }
                self.write(buf, "</a>")?;
            }

            Expr::Paragraph(inner) => {
                self.write(buf, "<p>")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "</p>")?;
            }

            Expr::Italic(inner) => {
                self.write(buf, "<em>")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "</em>")?;
            }
            Expr::Bold(inner) => {
                self.write(buf, "<b>")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "</b>")?;
            }
            Expr::StrikeThrough(inner) => {
                self.write(buf, "<del>")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "</del>")?;
            }
            Expr::Underline(inner) => {
                self.write(buf, "<u>")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "</u>")?;
                // self.write(buf, "<span class=underline>")?;
                // for id in &inner.0 {
                //     self.export_rec(id, buf)?;
                // }
                // self.write(buf, "</span>")?;
            }
            Expr::BlankLine => {
                // self.write(buf, "\n")?;
            }
            Expr::SoftBreak => {
                self.write(buf, " ")?;
            }
            Expr::Plain(inner) => {
                self.write(buf, inner)?;
            }
            Expr::Verbatim(inner) => {
                self.write(buf, &format!("<code>{}</code>", inner.0))?;
            }
            Expr::Code(inner) => {
                self.write(buf, &format!("<code>{}</code>", inner.0))?;
            }
            Expr::Comment(inner) => {
                self.write(buf, &format!("<!--{}-->", inner.0))?;
            }
            Expr::InlineSrc(inner) => {
                self.write(
                    buf,
                    &format!("<code class={}>{}</code>", inner.lang, inner.body),
                )?;
                // if let Some(args) = inner.headers {
                //     self.write(buf, &format!("[{args}]"))?;
                // }
                // self.write(buf, &format!("{{{}}}", inner.body))?;
            }
            Expr::Keyword(inner) => {
                // self.write(buf, &format!("#+{}: {}", inner.key, inner.val))?;
            }
            Expr::LatexEnv(inner) => {
                let ret = latex_to_mathml(
                    &format!(
                        "\\begin{{{0}}}\n{1}\n\\end{{{0}}}\n",
                        inner.name, inner.contents
                    ),
                    DisplayStyle::Block,
                )
                .unwrap();
                self.write(buf, &ret)?;
            }
            Expr::LatexFragment(inner) => match inner {
                LatexFragment::Command { name, contents } => {
                    let mut pot_cont = String::new();
                    pot_cont.write_str(&format!("\\{name}"))?;
                    if let Some(command_cont) = contents {
                        pot_cont.write_str(&format!("{{{command_cont}}}"))?;
                    }
                    self.write(
                        buf,
                        &latex_to_mathml(&pot_cont, DisplayStyle::Inline).unwrap(),
                    )
                    .unwrap();
                }
                LatexFragment::Display(inner) => {
                    self.write(buf, &latex_to_mathml(inner, DisplayStyle::Block).unwrap())?;
                }
                LatexFragment::Inline(inner) => {
                    self.write(buf, &latex_to_mathml(inner, DisplayStyle::Inline).unwrap())?;
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

                match inner.bullet {
                    BulletKind::Unordered => {
                        self.write(buf, &format!("<li{class_val}{tag_val}>"))?;
                    }
                    BulletKind::Ordered(_) => {
                        self.write(buf, &format!("<li{class_val}{tag_val}>"))?;
                    }

                    // match counterkind {
                    //     CounterKind::Letter(lettre) => {
                    //         self.write(buf, &format!("{}.", lettre as char))?;
                    //     }
                    //     CounterKind::Number(num) => {
                    //         self.write(buf, &format!("{num}."))?;
                    //     }
                    // },
                }

                for id in &inner.children {
                    self.export_rec(id, buf)?;
                }

                self.write(buf, "</li>")?;
            }
            Expr::PlainList(inner) => match inner.kind {
                ListKind::Unordered | ListKind::Descriptive => {
                    self.write(buf, "<ul>")?;
                    for id in &inner.children {
                        self.export_rec(id, buf)?;
                    }
                    self.write(buf, "</ul>")?;
                }
                ListKind::Ordered(_) => {
                    self.write(buf, "<ol>")?;
                    for id in &inner.children {
                        self.export_rec(id, buf)?;
                    }
                    self.write(buf, "</ol>")?;
                }
            },
            Expr::PlainLink(inner) => {
                self.write(buf, &format!("{}:{}", inner.protocol, inner.path))?;
            }
            Expr::Entity(inner) => {
                self.write(buf, &format!("{}", inner.mapped_item))?;
            }
            Expr::Table(inner) => {
                self.write(buf, "<table>")?;

                for id in &inner.children {
                    self.export_rec(id, buf)?;
                }

                self.write(buf, "</table>")?;
            }

            Expr::TableRow(inner) => {
                match inner {
                    TableRow::Rule => { /*skip*/ }
                    TableRow::Standard(stands) => {
                        self.write(buf, "<tr>")?;
                        for id in stands.iter() {
                            self.export_rec(id, buf)?;
                        }
                        self.write(buf, "</tr>")?;
                    }
                }
            }
            Expr::TableCell(inner) => {
                self.write(buf, "<td>")?;
                for id in &inner.0 {
                    self.export_rec(id, buf)?;
                }
                self.write(buf, "</td>")?;
            }
        }

        Ok(())
    }

    fn pool(&self) -> &NodePool<'a> {
        &self.pool
    }

    fn write(&mut self, buf: &mut dyn fmt::Write, s: &str) -> fmt::Result {
        buf.write_str(s)
    }
}

mod tests {
    use super::*;
}
