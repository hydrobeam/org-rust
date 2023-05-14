use derive_more::From;
use std::fmt::Debug;

use crate::element::*;
use crate::node_pool::{NodeID, NodePool};
use crate::object::*;
use bitflags::bitflags;

#[derive(Clone)]
pub struct Node<'a> {
    pub obj: Expr<'a>,
    pub start: usize,
    /// One past the last index in the match, such that
    /// arr[start..end] returns the matched region
    // makes starting the next match more convenient too
    pub end: usize,
    pub parent: Option<NodeID>,
}

impl<'a> Default for Node<'a> {
    fn default() -> Self {
        Self {
            obj: Expr::BlankLine,
            start: Default::default(),
            end: Default::default(),
            parent: Default::default(),
        }
    }
}

impl std::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.write_fmt(format_args!("{:#?}", self.obj))
        } else {
            f.write_fmt(format_args!("{:?}", self.obj))
        }
    }
}

impl<'a> Node<'a> {
    pub(crate) fn new<T>(obj: T, start: usize, end: usize, parent: Option<NodeID>) -> Self
    where
        Expr<'a>: From<T>,
    {
        Self {
            obj: Expr::from(obj),
            start,
            end,
            parent,
        }
    }

    pub fn print_tree(&self, pool: &NodePool) {
        self.obj.print_tree(pool);
    }
}

#[derive(From, Clone)]
pub enum Expr<'a> {
    // Branch
    Root(Vec<NodeID>),
    Heading(Heading<'a>),
    Block(Block<'a>),
    Link(Link<'a>),
    Paragraph(Paragraph),
    Italic(Italic),
    Bold(Bold),
    StrikeThrough(StrikeThrough),
    Underline(Underline),
    PlainList(PlainList),

    // Leaf
    // ZST
    BlankLine,
    SoftBreak,
    // Normal
    Plain(&'a str),
    MarkupEnd(MarkupKind),
    Verbatim(Verbatim<'a>),
    Code(Code<'a>),
    Comment(Comment<'a>),
    // Boxed
    InlineSrc(InlineSrc<'a>),
    Keyword(Keyword<'a>),
}

pub type Result<T> = std::result::Result<T, MatchError>;

// TODO: maybe make all fields bitflags for space optimization
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ParseOpts {
    pub allow_newline: bool,
    pub from_paragraph: bool,
    pub prev_newline: bool,
    pub from_object: bool,
    pub in_link: bool,
    pub markup: MarkupKind,
    pub indentation_level: u8,
}

#[derive(Debug)]
pub enum MatchError {
    InvalidLogic,
    EofError,
}

impl std::fmt::Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "unsuccesful match")
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MarkupKind: u32 {
        const Italic        = 1 << 0;
        const Bold          = 1 << 1;
        const Underline     = 1 << 2;
        const StrikeThrough = 1 << 3;
        const Verbatim      = 1 << 4;
        const Code          = 1 << 5;
        const Link          = 1 << 6;
        const LinkDescBegin = 1 << 7;
        const LinkEnd       = 1 << 8;
    }
}

pub(crate) trait Parseable<'a> {
    fn parse(
        pool: &mut NodePool<'a>,
        byte_arr: &'a [u8],
        index: usize,
        parent: Option<NodeID>,
        parse_opts: ParseOpts,
    ) -> Result<NodeID>;
}

// Custom Debug Impls
//
// We don't use the default debug impls becaus the
// Rc<RefCell<Match<Node::Branch(Branch::Paragraph(...))>>>
//
// ... levels of indirection make it impossible to digest the output.

// TODO: this sucks because implementing Debug to pull data from elsewhere
// is either hard or not possible
impl<'a> Expr<'a> {
    fn print_tree(&self, pool: &NodePool) {
        match self {
            Expr::Root(inner) => {
                print!("Root(");
                for id in inner {
                    // print!("{:#?}: ", id);
                    pool[*id].obj.print_tree(pool);
                    print!("\n");
                }
                print!(")");
            }
            Expr::Heading(inner) => {
                print!("Heading {{\n");
                println!("heading_level: {:#?}", inner.heading_level);
                println!("keyword: {:#?}", inner.keyword);
                println!("priority: {:#?}", inner.priority);
                println!("tags: {:#?}", inner.tags);
                print!("title: ");
                if let Some(title) = &inner.title {
                    for id in title {
                        pool[*id].obj.print_tree(pool);
                    }
                }
                print!("\n");
                print!("children: [");
                if let Some(children) = &inner.children {
                    for id in children {
                        // print!("{:#?}: ", id);
                        pool[*id].obj.print_tree(pool);
                        print!(", ");
                    }
                }
                print!("]");
                print!("}}")
            }
            Expr::Block(inner) => match &inner.contents {
                BlockContents::Greater(children) => {
                    print!("Block{{\n");
                    for id in children {
                        pool[*id].obj.print_tree(pool);
                        print!(",\n ");
                    }
                    print!("\nEndBlock}}");
                }
                BlockContents::Lesser(cont) => {
                    println!("{:#?}", inner);
                }
            },
            Expr::Link(inner) => {}
            Expr::Paragraph(inner) => {
                print!("Paragraph {{");
                for id in &inner.0 {
                    // print!("{:#?}: ", id);
                    pool[*id].obj.print_tree(pool);
                    print!(", ");
                }
                print!("}}");
            }

            Expr::Italic(inner) => {
                print!("Italic{{");
                for id in &inner.0 {
                    pool[*id].obj.print_tree(pool);
                }
                print!("}}");
            }
            Expr::Bold(inner) => {
                print!("Bold{{");
                for id in &inner.0 {
                    pool[*id].obj.print_tree(pool);
                }
                print!("}}");
            }
            Expr::StrikeThrough(inner) => {
                print!("StrikeThrough{{");
                for id in &inner.0 {
                    pool[*id].obj.print_tree(pool);
                }
                print!("}}");
            }
            Expr::Underline(inner) => {
                print!("Underline{{");
                for id in &inner.0 {
                    pool[*id].obj.print_tree(pool);
                }
                print!("}}");
            }
            Expr::PlainList(inner) => todo!(),
            Expr::BlankLine => print!("BlankLine"),
            Expr::SoftBreak => print!("SoftBreak"),
            Expr::Plain(inner) => print!("{:#?}", inner),
            Expr::MarkupEnd(inner) => print!("{:#?}", inner),
            Expr::Verbatim(inner) => print!("{:#?}", inner),
            Expr::Code(inner) => print!("{:#?}", inner),
            Expr::Comment(inner) => print!("{:#?}", inner),
            Expr::InlineSrc(inner) => print!("{:#?}", inner),
            Expr::Keyword(inner) => print!("{:#?}", inner),
        }
    }
}

#[allow(clippy::format_in_format_args)]
impl<'a> std::fmt::Debug for Expr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Whether something is a leaf or a branch is pretty internal, don't bother
        // with exposing this in debugging output
        //
        // These enum variants have types which have the same name as themselves
        // Branch::Paragraph(Paragraph(...)) is a lot of extra noise vs just Paragraph(...)
        // Skip over the Match struct since the start/end values really clutter the output
        if f.alternate() {
            match self {
                Expr::Root(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Heading(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Block(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Link(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Paragraph(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Italic(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Bold(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::StrikeThrough(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Underline(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::PlainList(inner) => f.write_fmt(format_args!("{:#?}", inner)),

                Expr::BlankLine => f.write_str("BlankLine"),
                Expr::SoftBreak => f.write_str("SoftBreak"),
                Expr::Plain(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::MarkupEnd(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Verbatim(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Code(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Comment(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::InlineSrc(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Expr::Keyword(inner) => f.write_fmt(format_args!("{:#?}", inner)),
            }
        } else {
            match self {
                Expr::Root(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Heading(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Block(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Link(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Paragraph(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Italic(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Bold(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::StrikeThrough(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Underline(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::PlainList(inner) => f.write_fmt(format_args!("{:?}", inner)),

                Expr::BlankLine => f.write_str("BlankLine"),
                Expr::SoftBreak => f.write_str("SoftBreak"),
                Expr::Plain(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::MarkupEnd(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Verbatim(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Code(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Comment(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::InlineSrc(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Expr::Keyword(inner) => f.write_fmt(format_args!("{:?}", inner)),
            }
        }
    }
}

mod object {
    use bitflags::bitflags;
    bitflags! {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct Object: u32 {
            const Entity            = 1 << 0;
            const LatexFragment     = 1 << 1;
            const ExportSnippet     = 1 << 2;
            const FootnoteReference = 1 << 3;
            const Citation          = 1 << 4;
            const CitationReference = 1 << 5;
            const InlineBabel       = 1 << 6;
            const InlineSrc         = 1 << 7;
            const LineBreak         = 1 << 8;
            const Link              = 1 << 9;
            const Macro             = 1 << 10;
            const Target            = 1 << 11;
            const StatCookie        = 1 << 12;
            const SubSuperscript    = 1 << 13;
            const TableCell         = 1 << 14;
            const TimeStamp         = 1 << 15;
            const Markup            = 1 << 16;
            const Plain             = 1 << 17;
        }
    }
    const ALL: Object = Object::all();
    const STANDARD: Object = ALL.difference(Object::from_bits_truncate(
        Object::TableCell.bits() | Object::CitationReference.bits(),
    ));
    const MINIMAL: Object = Object::from_bits_truncate(
        Object::Markup.bits()
            | Object::Plain.bits()
            | Object::Entity.bits()
            | Object::SubSuperscript.bits(),
    );

    const HEADING_TEXT: Object = STANDARD.difference(Object::LineBreak);
    const TABLE_CONTENTS: Object = MINIMAL.union(Object::from_bits_truncate(
        Object::Citation.bits()
            | Object::ExportSnippet.bits()
            | Object::FootnoteReference.bits()
            | Object::Link.bits()
            | Object::Macro.bits()
            | Object::Target.bits()
            | Object::TimeStamp.bits(),
    ));
}
