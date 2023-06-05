use core::fmt;

use std::fmt::Result;
use std::fmt::Write;

use latex2mathml::{latex_to_mathml, DisplayStyle};
use org_parser::element::{BlockKind, CheckBox, ListKind};

use crate::org_macros::macro_handle;
use crate::types::Exporter;
use org_parser::element::{BlockContents, TableRow};
use org_parser::node_pool::NodeID;
use org_parser::object::{LatexFragment, PathReg, PlainOrRec};
use org_parser::parse_org;
use org_parser::types::{Expr, Parser};

pub struct Html<'buf> {
    buf: &'buf mut dyn fmt::Write,
    // parser: Parser<'a>,
    // pool: &'a NodePool<'a>,
    // targets: &'a BTreeMap<&'a str, &'a str>,
}

pub(crate) struct HtmlEscape<'a>(pub &'a str);

impl<'a> fmt::Display for HtmlEscape<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
        for byte in self.0.as_bytes() {
            match *byte {
                b'<' => write!(f, r"&lt;")?,
                b'>' => write!(f, r"&gt;")?,
                b'&' => write!(f, r"&amp;")?,
                b'"' => write!(f, r"&quot;")?,
                b'\'' => write!(f, r"&#39;")?,
                byte => write!(f, "{}", byte as char)?,
            }
        }

        Ok(())
    }
}

impl<'a, 'buf> Exporter<'a, 'buf> for Html<'buf> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error> {
        let mut buf = String::new();
        let parsed = parse_org(input);
        let mut obj = Html { buf: &mut buf };

        obj.export_rec(&parsed.pool.root_id(), &parsed)?;
        Ok(buf)
    }

    fn export_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error> {
        let parsed = parse_org(input);
        let mut obj = Html { buf };

        obj.export_rec(&parsed.pool.root_id(), &parsed)?;
        Ok(buf)
    }

    fn export_rec(&mut self, node_id: &NodeID, parser: &Parser) -> Result {
        match &parser.pool[*node_id].obj {
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
                    self.export_rec(id, parser)?;
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
                        parser.targets.get(title.0).unwrap(),
                    )?;
                    for id in &title.1 {
                        self.export_rec(id, parser)?;
                    }

                    // must exist if we are a heading
                } else {
                    write!(self, "<h{heading_number}>")?;
                }

                writeln!(self, "</h{heading_number}>")?;

                if let Some(children) = &inner.children {
                    for id in children {
                        self.export_rec(id, parser)?;
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
                                        self.export_rec(id, parser)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{}", HtmlEscape(cont))?;
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
                                        self.export_rec(id, parser)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{}", HtmlEscape(cont))?;
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
                                        self.export_rec(id, parser)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{}", HtmlEscape(cont))?;
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
                                        self.export_rec(id, parser)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{}", HtmlEscape(cont))?;
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
                                        self.export_rec(id, parser)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{}", cont)?;
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
                                        self.export_rec(id, parser)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{}", HtmlEscape(cont))?;
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
                                        self.export_rec(id, parser)?;
                                    }
                                }
                                BlockContents::Lesser(cont) => {
                                    writeln!(self, "{}", HtmlEscape(cont))?;
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
                        for (match_targ, ret) in parser.targets.iter() {
                            if match_targ.starts_with(a) {
                                rita = format!("#{}", ret);
                                break;
                            }
                        }
                        // TODO: how to handle non-existing links
                        rita
                    }
                };
                write!(self, "<a href={}>", HtmlEscape(&path_link))?;
                if let Some(children) = &inner.description {
                    for id in children {
                        self.export_rec(id, parser)?;
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
                    self.export_rec(id, parser)?;
                }
                writeln!(self, "\n</p>")?;
            }

            Expr::Italic(inner) => {
                write!(self, "<em>")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                write!(self, "</em>")?;
            }
            Expr::Bold(inner) => {
                write!(self, "<b>")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                write!(self, "</b>")?;
            }
            Expr::StrikeThrough(inner) => {
                write!(self, "<del>")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                write!(self, "</del>")?;
            }
            Expr::Underline(inner) => {
                write!(self, "<u>")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                write!(self, "</u>")?;
                // write!(self, "<span class=underline>")?;
                // for id in &inner.0 {
                //     self.export_rec(id, parser)?;
                // }
                // write!(self, "</span>")?;
            }
            Expr::BlankLine => {
                // write!(self, "\n")?;
            }
            Expr::SoftBreak => {
                write!(self, " ")?;
            }
            Expr::LineBreak => {
                writeln!(self, "\n<br>")?;
            }
            Expr::HorizontalRule => {
                writeln!(self, "\n<hr>")?;
            }
            Expr::Plain(inner) => {
                write!(self, "{}", HtmlEscape(inner))?;
            }
            Expr::Verbatim(inner) => {
                write!(self, "<code>{}</code>", HtmlEscape(inner.0))?;
            }
            Expr::Code(inner) => {
                write!(self, "<code>{}</code>", HtmlEscape(inner.0))?;
            }
            Expr::Comment(inner) => {
                write!(self, "<!--{}-->", inner.0)?;
            }
            Expr::InlineSrc(inner) => {
                write!(
                    self,
                    "<code class={}>{}</code>",
                    inner.lang,
                    HtmlEscape(inner.body)
                )?;
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
                        inner.name,
                        HtmlEscape(inner.contents)
                    ),
                    DisplayStyle::Block,
                )
                .unwrap();
                writeln!(self, "{ret}")?;
            }
            Expr::LatexFragment(inner) => match inner {
                LatexFragment::Command { name, contents } => {
                    let mut pot_cont = String::new();
                    write!(pot_cont, r#"\{name}"#)?;
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
                    self.export_rec(id, parser)?;
                }

                writeln!(self, "</li>")?;
            }
            Expr::PlainList(inner) => match inner.kind {
                ListKind::Unordered | ListKind::Descriptive => {
                    writeln!(self, "<ul>")?;
                    for id in &inner.children {
                        self.export_rec(id, parser)?;
                    }
                    writeln!(self, "</ul>")?;
                }
                ListKind::Ordered(_) => {
                    writeln!(self, "<ol>")?;
                    for id in &inner.children {
                        self.export_rec(id, parser)?;
                    }
                    writeln!(self, "</ol>")?;
                }
            },
            Expr::PlainLink(inner) => {
                write!(
                    self,
                    "<a href={0}:{1}>{0}:{1}</a>",
                    inner.protocol, inner.path
                )?;
            }
            Expr::Entity(inner) => {
                write!(self, "{}", inner.mapped_item)?;
            }
            Expr::Table(inner) => {
                writeln!(self, "<table>")?;

                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }

                writeln!(self, "</table>")?;
            }

            Expr::TableRow(inner) => {
                match inner {
                    TableRow::Rule => { /*skip*/ }
                    TableRow::Standard(stands) => {
                        write!(self, "<tr>")?;
                        for id in stands.iter() {
                            self.export_rec(id, parser)?;
                        }
                        writeln!(self, "</tr>")?;
                    }
                }
            }
            Expr::TableCell(inner) => {
                write!(self, "<td>")?;
                for id in &inner.0 {
                    self.export_rec(id, parser)?;
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
                        self.export_rec(id, parser)?;
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
                        self.export_rec(id, parser)?;
                    }

                    write!(self, "</sub>")?;
                }
            },
            Expr::Target(inner) => {
                write!(self, "<span id={0}>{0}</span>", HtmlEscape(inner.0))?;
            }
            Expr::Macro(macro_call) => {
                macro_handle(parser, macro_call, self)?;
            }
        }

        Ok(())
    }
}

impl<'buf> fmt::Write for Html<'_> {
    fn write_str(&mut self, s: &str) -> Result {
        self.buf.write_str(s)
    }
}

mod tests {
    use super::*;

    #[test]
    fn combined_macros() -> fmt::Result {
        let a = Html::export(
            r"#+macro: poem hiii $1 $2 text
{{{poem(cool, three)}}}
",
        )?;

        assert_eq!(
            a,
            r"<p>
hiii cool three text
</p>
"
        );
        // println!("{a}");

        Ok(())
    }

    #[test]
    fn keyword_macro() -> Result {
        let a = Html::export(
            r"
     #+title: hiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii
{{{keyword(title)}}}
",
        )?;

        println!("{a}");
        Ok(())
    }

    #[test]
    fn defined_keyword_macro() -> Result {
        let a = Html::export(r" {{{keyword(email)}}}")?;

        println!("{a}");
        Ok(())
    }

    #[test]
    fn line_break() -> Result {
        let a = Html::export(
            r" abc\\
",
        )?;

        assert_eq!(
            a,
            r"<p>
abc
<br>

</p>
"
        );

        let n = Html::export(
            r" abc\\   q
",
        )?;

        assert_eq!(
            n,
            r"<p>
abc\\   q
</p>
"
        );
        Ok(())
    }

    #[test]
    fn horizontal_rule() -> Result {
        let a = Html::export(
            r"-----
",
        )?;

        let b = Html::export(
            r"                -----
",
        )?;

        let c = Html::export(
            r"      -------------------------
",
        )?;

        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a, c);

        let nb = Html::export(
            r"                ----
",
        )?;

        assert_eq!(
            nb,
            r"<p>
----
</p>
"
        );

        Ok(())
    }
}
