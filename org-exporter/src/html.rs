use core::fmt;

use std::fmt::Result;
use std::fmt::Write;

use latex2mathml::{latex_to_mathml, DisplayStyle};
use memchr::memchr3_iter;
use org_parser::element::Affiliated;
use org_parser::element::Block;
use org_parser::element::{CheckBox, ListKind, TableRow};

use crate::org_macros::macro_handle;
use crate::types::Exporter;
use org_parser::node_pool::NodeID;
use org_parser::object::{LatexFragment, PathReg, PlainOrRec};
use org_parser::parse_org;
use org_parser::types::{Expr, Parser};

macro_rules! tag_form {
    ($buf: tt, $tag:expr, $node:ident, $contents:block) => {
        write!($buf, "<{}", $tag)?;

        if let Some(tag_contents) = $node.id_target.as_ref() {
            write!($buf, r#" id="{tag_contents}""#)?;
        }

        write!($buf, ">")?;
        $contents;

        write!($buf, "</{}>", $tag)?;
    };
    ($buf: tt, $tag:expr, $node:ident, $contents:block, $($class:expr),+) => {
        write!($buf, "<{}", $tag)?;

        if let Some(tag_contents) = $node.id_target.as_ref() {
            write!($buf, r#" id="{tag_contents}""#)?;
        }

        $(
        write!($buf, r#" class="{}""#, $class)?;
        )+

        write!($buf, ">")?;

        $contents;

        write!($buf, "</{}>", $tag)?;
    };
    ($buf: tt, $tag:expr, $node:ident, $contents:expr) => {
        write!($buf, "<{}", $tag)?;

        if let Some(tag_contents) = $node.id_target.as_ref() {
            write!($buf, r#" id="{tag_contents}""#)?;
        }

        write!($buf, ">{}</{}>", $contents, $tag)?;
    };
    ($buf: tt, $tag:expr, $node:ident, $contents:expr, $($class:expr),+) => {
        write!($buf, "<{}", $tag)?;

        if let Some(tag_contents) = $node.id_target.as_ref() {
            write!($buf, r#" id="{tag_contents}""#)?;
        }

        $(
        write!($buf, r#" class="{}""#, $class)?;
        )+

        write!($buf, ">{}</{}>", $contents, $tag)?;
    };
}
pub struct Html<'buf> {
    buf: &'buf mut dyn fmt::Write,
}

pub(crate) struct HtmlEscape<'a>(pub &'a str);

impl<'a> fmt::Display for HtmlEscape<'a> {
    // design based on:
    // https://lise-henry.github.io/articles/optimising_strings.html
    // we can iterate over bytes since it's not possible for
    // an ascii character to appear in the codepoint of another larger char
    // if we see an ascii, then it's guaranteed to be valid
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
        let mut prev_pos = 0;
        // there are other characters we could escape, but memchr caps out at 3
        // the really important one is `<`, and then also probably &
        // throwing in `>` for good measure
        // based on:
        // https://mina86.com/2021/no-you-dont-need-to-escape-that/
        // there are invariants in the parsing (i hope) that should make
        // using memchr3 okay. if not, consider using jetscii for more byte blasting

        let mut escape_bytes = memchr3_iter(b'<', b'&', b'>', self.0.as_bytes());

        while let Some(ret) = escape_bytes.next() {
            write!(f, "{}", &self.0[prev_pos..ret])?;

            match self.0.as_bytes()[ret] {
                b'<' => write!(f, r"&lt;")?,
                b'>' => write!(f, r"&gt;")?,
                b'&' => write!(f, r"&amp;")?,
                _ => unreachable!(),
            }
            prev_pos = ret + 1;
        }

        write!(f, "{}", &self.0[prev_pos..])
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
        let node_obj = &parser.pool[*node_id];
        match &node_obj.obj {
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

                tag_form!(self, format_args!("h{heading_number}"), node_obj, {
                    if let Some(title) = &inner.title {
                        for id in &title.1 {
                            self.export_rec(id, parser)?;
                        }
                    }
                });

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
                        tag_form!(
                            self,
                            "div",
                            node_obj,
                            {
                                for id in contents {
                                    self.export_rec(id, parser)?;
                                }
                            },
                            "center"
                        );
                    }
                    Block::Quote {
                        parameters,
                        contents,
                    } => {
                        tag_form!(self, "blockquote", node_obj, {
                            for id in contents {
                                self.export_rec(id, parser)?;
                            }
                        });
                    }
                    Block::Special {
                        parameters,
                        contents,
                        name,
                    } => {
                        tag_form!(
                            self,
                            "div",
                            node_obj,
                            {
                                for id in contents {
                                    self.export_rec(id, parser)?;
                                }
                            },
                            name
                        );
                    }

                    // Lesser blocks
                    Block::Comment {
                        parameters,
                        contents,
                    } => {
                        writeln!(self, "<!--{}-->", contents)?;
                    }
                    Block::Example {
                        parameters,
                        contents,
                    } => {
                        // TODO: correct whitespace
                        tag_form!(self, "pre", node_obj, HtmlEscape(contents), "example");

                        // writeln!(self, "<pre class=example>\n{}</pre>", HtmlEscape(contents))?;
                    }
                    Block::Export {
                        parameters,
                        contents,
                    } => {
                        if let Some(params) = parameters {
                            if params.contains("html") {
                                writeln!(self, "{}", contents)?;
                            }
                        }
                    }
                    Block::Src {
                        parameters,
                        contents,
                    } => {
                        // TODO: work with the language parameter
                        tag_form!(self, "pre", node_obj, HtmlEscape(contents), "src");
                        // writeln!(self, "<pre class=src>\n{}</pre>", HtmlEscape(contents))?;
                    }
                    Block::Verse {
                        parameters,
                        contents,
                    } => {
                        // FIXME: apparently verse blocks contain objects...
                        tag_form!(self, "p", node_obj, HtmlEscape(contents), "verse");
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
                write!(self, "<a href=\"{}\">", HtmlEscape(&path_link))?;
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
                tag_form!(self, "p", node_obj, {
                    for id in &inner.0 {
                        self.export_rec(id, parser)?;
                    }
                });
            }

            Expr::Italic(inner) => {
                tag_form!(self, "em", node_obj, {
                    for id in &inner.0 {
                        self.export_rec(id, parser)?;
                    }
                });
            }
            Expr::Bold(inner) => {
                tag_form!(self, "b", node_obj, {
                    for id in &inner.0 {
                        self.export_rec(id, parser)?;
                    }
                });
            }
            Expr::StrikeThrough(inner) => {
                tag_form!(self, "del", node_obj, {
                    for id in &inner.0 {
                        self.export_rec(id, parser)?;
                    }
                });
            }
            Expr::Underline(inner) => {
                tag_form!(self, "u", node_obj, {
                    for id in &inner.0 {
                        self.export_rec(id, parser)?;
                    }
                });
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
                tag_form!(self, "code", node_obj, HtmlEscape(inner.0));
            }
            Expr::Code(inner) => {
                tag_form!(self, "code", node_obj, HtmlEscape(inner.0));
            }
            Expr::Comment(inner) => {
                write!(self, "<!--{}-->", inner.0)?;
            }
            Expr::InlineSrc(inner) => {
                tag_form!(self, "code", node_obj, HtmlEscape(inner.body), inner.lang);
                // if let Some(args) = inner.headers {
                //     write!(self, "[{args}]")?;
                // }
                // write!(self, "{{{}}}", inner.body)?;
            }
            Expr::Keyword(inner) => {
                // todo!()
                // match inner {
                //     Keyword::Basic { key, val } => {
                //         if ORG_AFFILIATED_KEYWORDS.contains(key) {
                //             todo!()
                //         }
                //     }
                //     Keyword::Macro(_) => todo!(),
                //     Keyword::Affilliated(_) => todo!(),
                // }
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
                if let Some(tag) = inner.tag {
                    tag_form!(self, "dt", node_obj, HtmlEscape(tag));
                    tag_form!(self, "dd", node_obj, {
                        for id in &inner.children {
                            self.export_rec(id, parser)?;
                        }
                    });
                } else {
                    write!(self, "<li")?;

                    if let Some(counter) = inner.counter_set {
                        write!(self, " value={}", counter)?;
                    }

                    if let Some(check) = &inner.check_box {
                        write!(
                            self,
                            " class={}",
                            match check {
                                CheckBox::Intermediate => "trans",
                                CheckBox::Off => "off",
                                CheckBox::On => "on",
                            }
                        )?;
                    }

                    write!(self, ">")?;

                    for id in &inner.children {
                        self.export_rec(id, parser)?;
                    }

                    writeln!(self, "</li>")?;
                }
            }
            Expr::PlainList(inner) => {
                tag_form!(
                    self,
                    match inner.kind {
                        ListKind::Unordered => "ul",
                        ListKind::Descriptive => "dl",
                        ListKind::Ordered(_) => "ol",
                    },
                    node_obj,
                    {
                        for id in &inner.children {
                            self.export_rec(id, parser)?;
                        }
                    }
                );
            }
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
                tag_form!(self, "table", node_obj, {
                    for id in &inner.children {
                        self.export_rec(id, parser)?;
                    }
                });
            }

            Expr::TableRow(inner) => {
                match inner {
                    TableRow::Rule => { /*skip*/ }
                    TableRow::Standard(stands) => {
                        tag_form!(self, "tr", node_obj, {
                            for id in stands.iter() {
                                self.export_rec(id, parser)?;
                            }
                        });
                    }
                }
            }
            Expr::TableCell(inner) => {
                tag_form!(self, "td", node_obj, {
                    for id in &inner.0 {
                        self.export_rec(id, parser)?;
                    }
                });
            }
            Expr::Emoji(inner) => {
                write!(self, "{}", inner.mapped_item)?;
            }
            Expr::Superscript(inner) => {
                tag_form!(self, "sub", node_obj, {
                    match &inner.0 {
                        PlainOrRec::Plain(inner) => {
                            write!(self, "{}", HtmlEscape(inner))?;
                        }
                        PlainOrRec::Rec(inner) => {
                            for id in inner {
                                self.export_rec(id, parser)?;
                            }
                        }
                    }
                });
            }
            Expr::Subscript(inner) => {
                tag_form!(self, "sub", node_obj, {
                    match &inner.0 {
                        PlainOrRec::Plain(inner) => {
                            write!(self, "{}", HtmlEscape(inner))?;
                        }
                        PlainOrRec::Rec(inner) => {
                            for id in inner {
                                self.export_rec(id, parser)?;
                            }
                        }
                    }
                });
            }
            Expr::Target(inner) => {
                tag_form!(self, "span", node_obj, HtmlEscape(inner.0));
            }
            Expr::Macro(macro_call) => {
                macro_handle(parser, macro_call, self)?;
            }
            Expr::Drawer(inner) => {
                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }
            }
            Expr::ExportSnippet(inner) => {
                if inner.backend == "html" {
                    write!(self, "{}", inner.contents)?;
                }
            }
            Expr::Affiliated(inner) => match inner {
                Affiliated::Name(id) => {}
                Affiliated::Caption(id, contents) => todo!(),
                Affiliated::Attr {
                    child_id,
                    backend,
                    val,
                } => todo!(),
            },
            Expr::MacroDef(_) => {}
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
{{{poem(cool,three)}}}
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

    #[test]
    fn correct_cache() -> Result {
        let a = Html::export(
            r"
- one
- two

\begin{align}
abc &+ 10\\
\end{align}
",
        )?;
        println!("{a}");

        Ok(())
    }

    #[test]
    fn html_unicode() -> Result {
        let a = Html::export(
            r"a é😳
",
        )?;

        assert_eq!(
            a,
            r"<p>
a é😳
</p>
"
        );

        Ok(())
    }

    #[test]
    fn list_counter_set() -> Result {
        let a = Html::export(
            r"
1. [@4] wordsss??
",
        )?;

        assert_eq!(
            a,
            r"<ol><li value=4><p>wordsss??</p></li>
</ol>"
        );
        Ok(())
    }
}
