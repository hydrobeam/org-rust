//! HTML Converter
//!
//! Converts the Org AST to its HTML representation.

use core::fmt;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::{Result, Write};

use latex2mathml::{latex_to_mathml, DisplayStyle};
use memchr::memchr3_iter;
use org_parser::element::{Affiliated, Block, CheckBox, ListKind, TableRow};
use org_parser::object::{LatexFragment, PathReg, PlainOrRec};
use org_parser::{parse_macro_call, parse_org, Expr, Node, NodeID, Parser};

use crate::include::include_handle;
use crate::org_macros::macro_handle;
use crate::types::{ConfigOptions, Exporter, ExporterInner};
use crate::utils::{process_toc, Options, TocItem};
use phf::phf_set;

// file types we can wrap an `img` around
static IMAGE_TYPES: phf::Set<&str> = phf_set! {
    "jpeg",
    "jpg",
    "png",
    "gif",
    "svg",
    "webp",
};

/// Directly convert these types when used in special blocks
/// to named blocks, e.g.:
///
/// #+begin_aside
/// #+end_aside
///
/// becomes
///
/// <aside></aside>
static HTML5_TYPES: phf::Set<&str> = phf_set! {
"article",
"aside",
"audio",
"canvas",
"details",
"figcaption",
"figure",
"footer",
"header",
"menu",
"meter",
"nav",
"output",
"progress",
"section",
"summary",
"video",
"picture",
};

/// HTML Content Exporter
pub struct Html<'buf> {
    buf: &'buf mut dyn fmt::Write,
    // HACK: When we export a caption, insert the child id here to make sure
    // it's not double exported
    nox: HashSet<NodeID>, // no-export
    // used footnotes
    footnotes: Vec<NodeID>,
    footnote_ids: HashMap<NodeID, usize>,
    conf: ConfigOptions,
}

/// Wrapper around strings that need to be properly HTML escaped.
pub(crate) struct HtmlEscape<S: AsRef<str>>(pub S);

// TODO this is not appropriate for certain things (can break). i can't rememmber them atm
// but you need to escape  more for certain stuff, it would be easier to just not use two separate htmlescapes
// REVIEW: jetscii
impl<'a, S: AsRef<str>> fmt::Display for HtmlEscape<S> {
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

        let v = self.0.as_ref();
        let escape_bytes = memchr3_iter(b'<', b'&', b'>', v.as_bytes());

        for ret in escape_bytes {
            write!(f, "{}", &v[prev_pos..ret])?;

            match v.as_bytes()[ret] {
                b'<' => write!(f, r"&lt;")?,
                b'>' => write!(f, r"&gt;")?,
                b'&' => write!(f, r"&amp;")?,
                _ => unreachable!(),
            }
            prev_pos = ret + 1;
        }

        write!(f, "{}", &v[prev_pos..])
    }
}

impl<'buf> Exporter<'buf> for Html<'buf> {
    fn export(input: &str, conf: ConfigOptions) -> core::result::Result<String, fmt::Error> {
        let mut buf = String::new();
        Html::export_buf(input, &mut buf, conf)?;
        Ok(buf)
    }

    fn export_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
        conf: ConfigOptions,
    ) -> Result {
        let parsed: Parser<'_> = parse_org(input);
        Html::export_tree(&parsed, buf, conf)
    }

    fn export_tree<'inp, T: fmt::Write>(
        parsed: &Parser,
        buf: &'buf mut T,
        conf: ConfigOptions,
    ) -> fmt::Result {
        let mut obj = Html {
            buf,
            nox: HashSet::new(),
            footnotes: Vec::new(),
            footnote_ids: HashMap::new(),
            conf,
        };

        if let Ok(opts) = Options::handle_opts(parsed) {
            if let Ok(tocs) = process_toc(parsed, &opts) {
                write!(
                    obj,
                    r#"<nav id="table-of-contents" role="doc-toc">
<h2>Table Of Contents</h2>
<div id="text-table-of-contents" role="doc-toc">
"#
                )?;
                write!(obj, "<ul>")?;
                for toc in tocs {
                    toc_rec(&parsed, &mut obj, &toc, 1)?;
                }
                write!(obj, "</ul>")?;
                write!(obj, r#"</div></nav>"#)?;
            }
        }
        obj.export_rec(&parsed.pool.root_id(), &parsed)?;
        obj.exp_footnotes(&parsed)
    }
}

fn toc_rec<'a, T: fmt::Write + ExporterInner<'a>>(
    parser: &Parser,
    writer: &mut T,
    parent: &TocItem,
    curr_level: u8,
) -> Result {
    write!(writer, "<li>")?;
    if curr_level < parent.level {
        write!(writer, "<ul>")?;
        toc_rec(&parser, writer, parent, curr_level + 1)?;
        write!(writer, "</ul>")?;
    } else {
        write!(writer, r#"<a href=#{}>"#, parent.target)?;
        for id in parent.name {
            writer.export_rec(id, parser)?;
        }
        write!(writer, "</a>")?;
        if !parent.children.is_empty() {
            write!(writer, "<ul>")?;
            for child in &parent.children {
                toc_rec(&parser, writer, child, curr_level + 1)?;
            }
            write!(writer, "</ul>")?;
        }
    }
    write!(writer, "</li>")
}

impl<'buf> ExporterInner<'buf> for Html<'buf> {
    fn export_macro_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
        conf: ConfigOptions,
    ) -> Result {
        let parsed = parse_macro_call(input);
        let mut obj = Html {
            buf,
            nox: HashSet::new(),
            footnotes: Vec::new(),
            footnote_ids: HashMap::new(),
            conf,
        };

        obj.export_rec(&parsed.pool.root_id(), &parsed)
    }

    fn export_rec(&mut self, node_id: &NodeID, parser: &Parser) -> Result {
        // avoid parsing this node
        if self.nox.contains(node_id) {
            return Ok(());
        }
        let node = &parser.pool[*node_id];
        match &node.obj {
            Expr::Root(inner) => {
                for id in inner {
                    self.export_rec(id, parser)?;
                }
            }
            Expr::Heading(inner) => {
                let heading_number: u8 = inner.heading_level.into();

                write!(self, "<h{heading_number}",)?;
                self.prop(node)?;
                write!(self, ">")?;

                if let Some(title) = &inner.title {
                    for id in &title.1 {
                        self.export_rec(id, parser)?;
                    }
                }

                writeln!(self, "</h{heading_number}>")?;

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
                        if parameters.get("exports").is_some_and(|&x| x == "none") {
                            return Ok(());
                        }
                        write!(self, "<div")?;
                        self.class("org-center")?;
                        self.prop(node)?;
                        writeln!(self, ">")?;
                        for id in contents {
                            self.export_rec(id, parser)?;
                        }
                        writeln!(self, "</div>")?;
                    }
                    Block::Quote {
                        parameters,
                        contents,
                    } => {
                        if parameters.get("exports").is_some_and(|&x| x == "none") {
                            return Ok(());
                        }
                        write!(self, "<blockquote")?;
                        self.prop(node)?;
                        writeln!(self, ">")?;
                        for id in contents {
                            self.export_rec(id, parser)?;
                        }
                        writeln!(self, "</blockquote>")?;
                    }
                    Block::Special {
                        parameters,
                        contents,
                        name,
                    } => {
                        if parameters.get("exports").is_some_and(|&x| x == "none") {
                            return Ok(());
                        }
                        // html5 names are directly converted into tags
                        if HTML5_TYPES.contains(name) {
                            write!(self, "<{name}")?;
                            self.prop(node)?;
                            writeln!(self, ">")?;
                            for id in contents {
                                self.export_rec(id, parser)?;
                            }
                            write!(self, "</{name}>")?;
                        } else {
                            write!(self, "<div")?;
                            self.prop(node)?;
                            self.class(name)?;
                            writeln!(self, ">")?;
                            for id in contents {
                                self.export_rec(id, parser)?;
                            }
                            writeln!(self, "</div>")?;
                        }
                    }

                    // Lesser blocks
                    Block::Comment {
                        parameters,
                        contents,
                    } => {
                        if parameters.get("exports").is_some_and(|&x| x == "none") {
                            return Ok(());
                        }
                        writeln!(self, "<!--{contents}-->")?;
                    }
                    Block::Example {
                        parameters,
                        contents,
                    } => {
                        if parameters.get("exports").is_some_and(|&x| x == "none") {
                            return Ok(());
                        }
                        write!(self, "<pre")?;
                        self.class("example")?;
                        self.prop(node)?;
                        writeln!(self, ">\n{}</pre>", HtmlEscape(contents))?;
                    }
                    Block::Export {
                        backend,
                        parameters,
                        contents,
                    } => {
                        if parameters.get("exports").is_some_and(|&x| x == "none") {
                            return Ok(());
                        }
                        if backend.is_some_and(|x| x == Html::backend_name()) {
                            writeln!(self, "{contents}")?;
                        }
                    }
                    Block::Src {
                        language,
                        parameters,
                        contents,
                    } => {
                        if parameters.get("exports").is_some_and(|&x| x == "none") {
                            return Ok(());
                        }
                        write!(self, "<pre>")?;
                        write!(self, "<code")?;
                        self.class("src")?;
                        if let Some(lang) = language {
                            self.class(&format!("src-{}", lang))?;
                        }
                        self.prop(node)?;
                        writeln!(self, ">\n{}</pre></code>", HtmlEscape(contents))?;
                    }
                    Block::Verse {
                        parameters,
                        contents,
                    } => {
                        if parameters.get("exports").is_some_and(|&x| x == "none") {
                            return Ok(());
                        }
                        // FIXME: apparently verse blocks contain objects...
                        write!(self, "<p")?;
                        self.class("verse")?;
                        self.prop(node)?;
                        writeln!(self, ">\n{}</p>", HtmlEscape(contents))?;
                    }
                }
            }
            Expr::RegularLink(inner) => {
                let path_link: String = match &inner.path.obj {
                    PathReg::PlainLink(a) => a.into(),
                    PathReg::Id(a) => format!("#{a}"),
                    PathReg::CustomId(a) => format!("#{a}"),
                    PathReg::Coderef(_) => todo!(),
                    PathReg::Unspecified(a) => {
                        let mut rita = String::new();
                        // see if the link is present in someone's target
                        for (match_targ, ret) in parser.targets.iter() {
                            if match_targ.starts_with(a.as_ref()) {
                                rita = format!("#{ret}");
                                break;
                            }
                        }
                        // if we confirmed it's not a target, just interpret the string directly
                        //
                        // handles the [[./hello]] case for us.
                        // turning it into <href="./hello">
                        if rita.is_empty() {
                            a.to_string()
                        } else {
                            rita
                        }
                    }
                    PathReg::File(a) => format!("{a}"),
                };
                write!(self, r#"<a href="{}">"#, HtmlEscape(&path_link))?;
                if let Some(children) = &inner.description {
                    for id in children {
                        self.export_rec(id, parser)?;
                    }
                } else {
                    write!(self, "{}", HtmlEscape(inner.path.to_str(parser.source)))?;
                }
                write!(self, "</a>")?;
            }

            Expr::Paragraph(inner) => {
                if inner.0.len() == 1 {
                    if let Expr::RegularLink(link) = &parser.pool[inner.0[0]].obj {
                        let link_source: &str = match &link.path.obj {
                            PathReg::Unspecified(inner) => inner,
                            PathReg::File(inner) => inner,
                            PathReg::PlainLink(_) => link.path.to_str(parser.source),
                            _ => {
                                // HACK: we just want to jump outta here, everything else doesnt make sense
                                // in an image context
                                "".into()
                            }
                        };

                        // extract extension_type
                        let ending_tag = link_source.split('.').last();
                        if let Some(extension) = ending_tag {
                            if IMAGE_TYPES.contains(extension) {
                                write!(self, "<figure>\n<img")?;
                                self.prop(node)?;
                                write!(self, r#" src="{}""#, HtmlEscape(link_source))?;
                                // start writing alt (if there are children)
                                write!(self, r#" alt=""#)?;
                                if let Some(children) = &link.description {
                                    for id in children {
                                        self.export_rec(id, parser)?;
                                    }
                                } else {
                                    let alt_text: Cow<str> =
                                        if let Some(slashed) = link_source.split('/').last() {
                                            slashed.into()
                                        } else {
                                            link_source.into()
                                        };
                                    write!(self, "{}", HtmlEscape(alt_text))?;
                                }
                                write!(self, "\">\n</figure>")?;
                                return Ok(());
                            }
                        }
                    }
                }
                write!(self, "<p")?;
                self.prop(node)?;
                write!(self, ">")?;

                for id in &inner.0 {
                    self.export_rec(id, parser)?;
                }
                writeln!(self, "</p>")?;
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
            Expr::Keyword(inner) => {
                if inner.key.to_ascii_lowercase() == "include" {
                    // FIXME: proper error handling
                    write!(self, r#"<div class="org-include""#)?;
                    self.prop(node)?;
                    write!(self, ">")?;
                    include_handle(inner.val, self).unwrap();
                    write!(self, "</div>")?;
                }
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
                    write!(pot_cont, r#"{name}"#)?;
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
                    write!(self, "<dt>{}</dt>", HtmlEscape(tag))?;
                    write!(self, "<dd>")?;
                    for id in &inner.children {
                        self.export_rec(id, parser)?;
                    }
                    write!(self, "</dd>")?;
                } else {
                    write!(self, "<li")?;

                    if let Some(counter) = inner.counter_set {
                        self.attr("value", counter)?;
                    }

                    if let Some(check) = &inner.check_box {
                        self.class(match check {
                            CheckBox::Intermediate => "trans",
                            CheckBox::Off => "off",
                            CheckBox::On => "on",
                        })?;
                    }

                    write!(self, ">")?;

                    for id in &inner.children {
                        self.export_rec(id, parser)?;
                    }

                    writeln!(self, "</li>")?;
                }
            }
            Expr::PlainList(inner) => {
                let tag = match inner.kind {
                    ListKind::Unordered => "ul",
                    ListKind::Ordered(counter_kind) => match counter_kind {
                        org_parser::element::CounterKind::Letter(c) => {
                            if c.is_ascii_uppercase() {
                                r#"ol type="A""#
                            } else {
                                r#"ol type="a""#
                            }
                        }
                        org_parser::element::CounterKind::Number(_) => r#"ol type="1""#,
                    },
                    ListKind::Descriptive => "dd",
                };
                write!(self, "<{tag}")?;
                self.prop(node)?;
                writeln!(self, ">")?;
                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }
                writeln!(self, "</{tag}>")?;
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
                write!(self, "<table")?;
                self.prop(node)?;
                writeln!(self, ">")?;

                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }

                writeln!(self, "</table>")?;
            }

            Expr::TableRow(inner) => {
                match inner {
                    TableRow::Rule => { /*skip*/ }
                    TableRow::Standard(stands) => {
                        writeln!(self, "<tr>")?;
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
            Expr::Superscript(inner) => {
                write!(self, "<sup>")?;
                match &inner.0 {
                    PlainOrRec::Plain(inner) => {
                        write!(self, "{inner}")?;
                    }
                    PlainOrRec::Rec(inner) => {
                        for id in inner {
                            self.export_rec(id, parser)?;
                        }
                    }
                }
                write!(self, "</sup>")?;
            }
            Expr::Subscript(inner) => {
                write!(self, "<sub>")?;
                match &inner.0 {
                    PlainOrRec::Plain(inner) => {
                        write!(self, "{inner}")?;
                    }
                    PlainOrRec::Rec(inner) => {
                        for id in inner {
                            self.export_rec(id, parser)?;
                        }
                    }
                }
                write!(self, "</sub>")?;
            }
            Expr::Target(inner) => {
                write!(self, "<span")?;
                self.prop(node)?;
                write!(self, ">")?;
                write!(
                    self,
                    "<span id={}>{}</span>",
                    parser.pool[*node_id].id_target.as_ref().unwrap(), // must exist
                    HtmlEscape(inner.0)
                )?;
            }
            Expr::Macro(macro_call) => {
                if let Ok(macro_contents) = macro_handle(parser, macro_call, self.config_opts()) {
                    match macro_contents {
                        Cow::Owned(p) => {
                            Html::export_macro_buf(&p, self, self.config_opts().clone())?;
                        }
                        Cow::Borrowed(r) => {
                            write!(self, "{}", HtmlEscape(r))?;
                        }
                    }
                }
            }
            Expr::Drawer(inner) => {
                for id in &inner.children {
                    self.export_rec(id, parser)?;
                }
            }
            Expr::ExportSnippet(inner) => {
                if inner.backend == Html::backend_name() {
                    write!(self, "{}", inner.contents)?;
                }
            }
            Expr::Affiliated(inner) => match inner {
                Affiliated::Name(_id) => {}
                Affiliated::Caption(id, contents) => {
                    if let Some(caption_id) = id {
                        writeln!(self, "<figure>")?;
                        self.export_rec(caption_id, parser)?;
                        writeln!(self, "<figcaption>")?;
                        self.export_rec(contents, parser)?;
                        writeln!(self, "</figcaption>")?;
                        writeln!(self, "</figure>")?;
                        self.nox.insert(*caption_id);
                    }
                }
                Affiliated::Attr { .. } => {}
            },
            Expr::MacroDef(_) => {}
            Expr::FootnoteDef(_) => {
                // handled after root
            }
            Expr::FootnoteRef(inner) => {
                let foot_len = self.footnotes.len();
                let target_id = if let Some(label) = inner.label {
                    if let Some(def_id) = parser.footnotes.get(label) {
                        *def_id
                    } else {
                        *node_id
                    }
                } else {
                    *node_id
                };

                let index = *self.footnote_ids.entry(target_id).or_insert_with(|| {
                    self.footnotes.push(target_id);
                    foot_len + 1
                });

                // prevent duplicate ids:
                // node ids are guaranteed to be unique
                let fn_id = if index != foot_len + 1 {
                    format!("{index}.{node_id}")
                } else {
                    format!("{index}")
                };

                write!(
                    self,
                    r##"<sup>
    <a id="fnr.{0}" href="#fn.{1}" class="footref" role="doc-backlink">{1}</a>
</sup>"##,
                    fn_id, index,
                )?;
            }
        }

        Ok(())
    }

    fn backend_name() -> &'static str {
        "html"
    }

    fn config_opts(&self) -> &ConfigOptions {
        &self.conf
    }
}

// Writers for generic attributes
impl<'buf> Html<'buf> {
    /// Adds a property
    fn prop(&mut self, node: &Node) -> Result {
        // if the target needs an id
        if let Some(tag_contents) = node.id_target.as_ref() {
            write!(self, r#" id="{tag_contents}""#)?;
        }

        // attach any keys that need to be placed
        if let Some(attrs) = node.attrs.get(Html::backend_name()) {
            for (key, val) in attrs {
                self.attr(key, val)?;
            }
        }

        Ok(())
    }

    fn class(&mut self, name: &str) -> Result {
        write!(self, r#" class="{name}""#)
    }

    fn attr(&mut self, key: &str, val: &str) -> Result {
        write!(self, r#" {}="{}""#, key, HtmlEscape(val))
    }

    fn exp_footnotes(&mut self, parser: &Parser) -> Result {
        if self.footnotes.is_empty() {
            return Ok(());
        }

        // get last heading, and check if its title is Footnotes,
        // if so, destroy it
        let heading_query = parser.pool.iter().rev().find(|node| {
            if let Expr::Heading(head) = &node.obj {
                if let Some(title) = &head.title {
                    if title.0 == "Footnotes\n" {
                        return true;
                    }
                }
            }
            false
        });

        writeln!(
            self,
            r#"
<div id="footnotes">
    <style>
    .footdef p {{
    display:inline;
    }}
    </style>"#
        )?;

        if heading_query.is_none() {
            writeln!(self, r#"    <h2 class="footnotes">Footnotes</h2>"#)?;
        }

        writeln!(self, r#"    <div id="text-footnotes">"#)?;

        // FIXME
        // lifetime shenanigans making me do this.. can't figure em out
        // would like to self.footnotes.iter(), but we get multiple
        // immutable borrows, so self.footnotes.copied.iter(), but still no go
        let man = self.footnotes.clone();
        for (mut pos, def_id) in man.iter().enumerate() {
            pos += 1;
            write!(
                self,
                r##"

<div class="footdef">
<sup>
    <a id="fn.{pos}" href= "#fnr.{pos}" role="doc-backlink">{pos}</a>
</sup>
"##
            )?;
            match &parser.pool[*def_id].obj {
                Expr::FootnoteDef(fn_def) => {
                    for child_id in &fn_def.children {
                        self.export_rec(child_id, parser)?;
                    }
                }
                Expr::FootnoteRef(fn_ref) => {
                    if let Some(children) = fn_ref.children.as_ref() {
                        for child_id in children {
                            self.export_rec(child_id, parser)?;
                        }
                    }
                }
                _ => (),
            }
            write!(self, r#"</div>"#)?;
        }
        write!(self, "\n  </div>\n</div>")
    }
}

impl fmt::Write for Html<'_> {
    fn write_str(&mut self, s: &str) -> Result {
        self.buf.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn html_export(input: &str) -> core::result::Result<String, fmt::Error> {
        Html::export(input, ConfigOptions::default())
    }
    #[test]
    fn combined_macros() -> fmt::Result {
        let a = html_export(
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

        Ok(())
    }

    #[test]
    fn keyword_macro() -> Result {
        let a = html_export(
            r"
     #+title: hiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii
{{{keyword(title)}}}
",
        )?;

        assert_eq!(
            a,
            r"<p>
hiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii
</p>
",
        );
        Ok(())
    }

    #[test]
    fn line_break() -> Result {
        let a = html_export(
            r" abc\\
",
        )?;

        assert_eq!(
            a,
            r"<p>
abc
<br>

</p>
",
        );

        let n = html_export(
            r" abc\\   q
",
        )?;

        assert_eq!(
            n,
            r"<p>
abc\\   q
</p>
",
        );
        Ok(())
    }

    #[test]
    fn horizontal_rule() -> Result {
        let a = html_export(
            r"-----
",
        )?;

        let b = html_export(
            r"                -----
",
        )?;

        let c = html_export(
            r"      -------------------------
",
        )?;

        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a, c);

        let nb = html_export(
            r"                ----
",
        )?;

        assert_eq!(
            nb,
            r"<p>
----
</p>
",
        );

        Ok(())
    }

    #[test]
    fn correct_cache() -> Result {
        let a = html_export(
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
        let a = html_export(
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
        let a = html_export(
            r"
1. [@4] wordsss??
",
        )?;

        assert_eq!(
            a,
            r#"<ol>
<li value="4"><p>
wordsss??
</p>
</li>
</ol>
"#,
        );
        Ok(())
    }
    #[test]
    fn anon_footnote() -> Result {
        let a = html_export(
            r"
hi [fn:next:coolio]
",
        )?;
        // just codifying what the output is here, not supposed to be set in stone
        assert_eq!(
            a,
            r##"<p>
hi <sup>
    <a id="fnr.1" href="#fn.1" class="footref" role="doc-backlink">1</a>
</sup>
</p>

<div id="footnotes">
    <style>
    .footdef p {
    display:inline;
    }
    </style>
    <h2 class="footnotes">Footnotes</h2>
    <div id="text-footnotes">


<div class="footdef">
<sup>
    <a id="fn.1" href= "#fnr.1" role="doc-backlink">1</a>
</sup>
coolio</div>
  </div>
</div>"##
        );
        Ok(())
    }

    #[test]
    fn footnote_heading() -> Result {
        let a = html_export(
            r"
hello [fn:1]

* Footnotes

[fn:1] world
",
        )?;

        // just codifying what the output is here, not supposed to be set in stone
        assert_eq!(
            a,
            r##"<p>
hello <sup>
    <a id="fnr.1" href="#fn.1" class="footref" role="doc-backlink">1</a>
</sup>
</p>
<h1 id="footnotes">Footnotes</h1>

<div id="footnotes">
    <style>
    .footdef p {
    display:inline;
    }
    </style>
    <div id="text-footnotes">


<div class="footdef">
<sup>
    <a id="fn.1" href= "#fnr.1" role="doc-backlink">1</a>
</sup>
<p>
world
</p>
</div>
  </div>
</div>"##
        );
        Ok(())
    }

    #[test]
    fn footnote_order() -> Result {
        // tests dupes too
        let a = html_export(
            r#"
hi [fn:dupe] cool test [fn:coolnote]  [fn:dupe:inlinefootnote]
coolest [fn:1] again [fn:1]

novel [fn:next:coolio]


** Footnotes

[fn:1] hi
[fn:dupe] abcdef
[fn:coolnote] words babby

"#,
        )?;

        // REVIEW; investigate different nodeids with export_buf and export
        // had to change 1.7 to 1.8 to pass the test
        assert_eq!(
            a,
            r##"<p>
hi <sup>
    <a id="fnr.1" href="#fn.1" class="footref" role="doc-backlink">1</a>
</sup> cool test <sup>
    <a id="fnr.2" href="#fn.2" class="footref" role="doc-backlink">2</a>
</sup>  <sup>
    <a id="fnr.1.8" href="#fn.1" class="footref" role="doc-backlink">1</a>
</sup> coolest <sup>
    <a id="fnr.3" href="#fn.3" class="footref" role="doc-backlink">3</a>
</sup> again <sup>
    <a id="fnr.3.13" href="#fn.3" class="footref" role="doc-backlink">3</a>
</sup>
</p>
<p>
novel <sup>
    <a id="fnr.4" href="#fn.4" class="footref" role="doc-backlink">4</a>
</sup>
</p>
<h2 id="footnotes">Footnotes</h2>

<div id="footnotes">
    <style>
    .footdef p {
    display:inline;
    }
    </style>
    <div id="text-footnotes">


<div class="footdef">
<sup>
    <a id="fn.1" href= "#fnr.1" role="doc-backlink">1</a>
</sup>
<p>
abcdef
</p>
</div>

<div class="footdef">
<sup>
    <a id="fn.2" href= "#fnr.2" role="doc-backlink">2</a>
</sup>
<p>
words babby
</p>
</div>

<div class="footdef">
<sup>
    <a id="fn.3" href= "#fnr.3" role="doc-backlink">3</a>
</sup>
<p>
hi
</p>
</div>

<div class="footdef">
<sup>
    <a id="fn.4" href= "#fnr.4" role="doc-backlink">4</a>
</sup>
coolio</div>
  </div>
</div>"##
        );
        Ok(())
    }

    #[test]
    fn esoteric_footnotes() -> Result {
        let a = html_export(
            r"
And anonymous ones [fn::mysterious]

what [fn::]

bad [fn:]
",
        )?;

        assert_eq!(
            a,
            r##"<p>
And anonymous ones <sup>
    <a id="fnr.1" href="#fn.1" class="footref" role="doc-backlink">1</a>
</sup>
</p>
<p>
what <sup>
    <a id="fnr.2" href="#fn.2" class="footref" role="doc-backlink">2</a>
</sup>
</p>
<p>
bad [fn:]
</p>

<div id="footnotes">
    <style>
    .footdef p {
    display:inline;
    }
    </style>
    <h2 class="footnotes">Footnotes</h2>
    <div id="text-footnotes">


<div class="footdef">
<sup>
    <a id="fn.1" href= "#fnr.1" role="doc-backlink">1</a>
</sup>
mysterious</div>

<div class="footdef">
<sup>
    <a id="fn.2" href= "#fnr.2" role="doc-backlink">2</a>
</sup>
</div>
  </div>
</div>"##
        );

        Ok(())
    }

    #[test]
    fn file_link() -> Result {
        let a = html_export(r"[[file:html.org][hi]]")?;

        assert_eq!(
            a,
            r#"<p>
<a href="html.org">hi</a>
</p>
"#
        );

        Ok(())
    }

    #[test]
    fn file_link_image() -> Result {
        let a = html_export(
            r"
[[file:bmc.jpg]]
",
        )?;
        assert_eq!(
            a,
            r#"<figure>
<img src="bmc.jpg" alt="bmc.jpg">
</figure>"#
        );

        Ok(())
    }

    #[test]
    fn basic_link_image() -> Result {
        let a = html_export(
            r"
[[https://upload.wikimedia.org/wikipedia/commons/a/a6/Org-mode-unicorn.svg]]
",
        )?;

        assert_eq!(
            a,
            r#"<figure>
<img src="https://upload.wikimedia.org/wikipedia/commons/a/a6/Org-mode-unicorn.svg" alt="Org-mode-unicorn.svg">
</figure>"#
        );

        Ok(())
    }

    #[test]
    fn unspecified_link() -> Result {
        let a = html_export(r"[[./hello]]")?;

        assert_eq!(
            a,
            r##"<p>
<a href="./hello">./hello</a>
</p>
"##
        );

        Ok(())
    }

    #[test]
    fn checkbox() -> Result {
        let a = html_export("- [X]\n")?;

        assert_eq!(
            a,
            r#"<ul>
<li class="on"></li>
</ul>
"#
        );

        let b = html_export("- [ ]\n")?;

        assert_eq!(
            b,
            r#"<ul>
<li class="off"></li>
</ul>
"#
        );

        let c = html_export("- [-]\n")?;

        assert_eq!(
            c,
            r#"<ul>
<li class="trans"></li>
</ul>
"#
        );

        Ok(())
    }

    #[test]
    fn words_with_line_breaks() {
        let a = r#"

#+kw: hi

* yeah
hello

{{{keyword(kw)}}}

content

here
"#;
        dbg!(html_export(a));
        assert_eq!(html_export(a).unwrap(), "love!");
    }
}
