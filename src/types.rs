use derive_more::From;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

use crate::element::{Block, Comment, Heading, Keyword, Paragraph, PlainList};
use crate::object::{Bold, Code, InlineSrc, Italic, Link, StrikeThrough, Underline, Verbatim};
use bitflags::bitflags;

pub type BranchNode<T> = Rc<RefCell<Match<T>>>;
pub type BoxLeafNode<T> = Box<Match<T>>;
pub type LeafNode<T> = Match<T>;

pub struct BlankLine;
pub struct SoftBreak;

#[derive(From)]
pub enum Node<'a> {
    // Branch
    Root(BranchNode<Vec<Node<'a>>>),
    Heading(BranchNode<Heading<'a>>),
    Block(BranchNode<Block<'a>>),
    Link(BranchNode<Link<'a>>),
    Paragraph(BranchNode<Paragraph<'a>>),
    Italic(BranchNode<Italic<'a>>),
    Bold(BranchNode<Bold<'a>>),
    StrikeThrough(BranchNode<StrikeThrough<'a>>),
    Underline(BranchNode<Underline<'a>>),
    PlainList(BranchNode<PlainList<'a>>),

    // Leaf
    // ZST
    BlankLine(LeafNode<BlankLine>),
    SoftBreak(LeafNode<SoftBreak>),
    // Normal
    Plain(LeafNode<&'a str>),
    MarkupEnd(LeafNode<MarkupKind>),
    Verbatim(LeafNode<Verbatim<'a>>),
    Code(LeafNode<Code<'a>>),
    Comment(LeafNode<Comment<'a>>),
    // Boxed
    InlineSrc(BoxLeafNode<InlineSrc<'a>>),
    Keyword(BoxLeafNode<Keyword<'a>>),
}


pub type Result<T> = std::result::Result<T, MatchError>;
pub type Cache<'a> = RefCell<std::collections::HashMap<usize, Node<'a>>>;

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

impl<'a> Node<'a> {
    pub fn make_branch<T>(branch: T, start: usize, end: usize) -> Node<'a>
    where
        Node<'a>: From<BranchNode<T>>,
    {
        Node::from(Rc::new(RefCell::new(Match {
            obj: branch,
            start,
            end,
        })))
    }

    pub fn make_leaf<T>(leaf: T, start: usize, end: usize) -> Node<'a>
    where
        Node<'a>: From<LeafNode<T>>,
    {
        Node::from(Match {
            obj: leaf,
            start,
            end,
        })
    }

    pub fn make_boxed_leaf<T>(leaf: T, start: usize, end: usize) -> Node<'a>
    where
        Node<'a>: From<BoxLeafNode<T>>,
    {
        Node::from(Box::new(Match {
            obj: leaf,
            start,
            end,
        }))
    }

    pub fn get_start(&self) -> usize {
        match self {
            Node::Root(inner) => inner.borrow().start,
            Node::Heading(inner) => inner.borrow().start,
            Node::Block(inner) => inner.borrow().start,
            Node::Link(inner) => inner.borrow().start,
            Node::Paragraph(inner) => inner.borrow().start,
            Node::Italic(inner) => inner.borrow().start,
            Node::Bold(inner) => inner.borrow().start,
            Node::StrikeThrough(inner) => inner.borrow().start,
            Node::Underline(inner) => inner.borrow().start,
            Node::PlainList(inner) => inner.borrow().start,

            Node::BlankLine(inner) => inner.start,
            Node::SoftBreak(inner) => inner.start,
            Node::Plain(inner) => inner.start,
            Node::MarkupEnd(inner) => inner.start,
            Node::Verbatim(inner) => inner.start,
            Node::Code(inner) => inner.start,
            Node::Comment(inner) => inner.start,
            Node::InlineSrc(inner) => inner.start,
            Node::Keyword(inner) => inner.start,
        }
    }
    pub fn get_end(&self) -> usize {
        match self {
            Node::Root(inner) => inner.borrow().end,
            Node::Heading(inner) => inner.borrow().end,
            Node::Block(inner) => inner.borrow().end,
            Node::Link(inner) => inner.borrow().end,
            Node::Paragraph(inner) => inner.borrow().end,
            Node::Italic(inner) => inner.borrow().end,
            Node::Bold(inner) => inner.borrow().end,
            Node::StrikeThrough(inner) => inner.borrow().end,
            Node::Underline(inner) => inner.borrow().end,
            Node::PlainList(inner) => inner.borrow().end,

            Node::BlankLine(inner) => inner.end,
            Node::SoftBreak(inner) => inner.end,
            Node::Plain(inner) => inner.end,
            Node::MarkupEnd(inner) => inner.end,
            Node::Verbatim(inner) => inner.end,
            Node::Code(inner) => inner.end,
            Node::Comment(inner) => inner.end,
            Node::InlineSrc(inner) => inner.end,
            Node::Keyword(inner) => inner.end,
        }
    }

}

#[derive(Debug, Clone, Copy)]
pub struct Match<T> {
    pub obj: T,
    pub start: usize,
    /// One past the last index in the match, such that
    /// arr[start..end] returns the matched region
    // makes starting the next match more convenient too
    pub end: usize,
}

pub(crate) trait Parseable<'a> {
    fn parse(byte_arr: &'a [u8], index: usize, parse_opts: ParseOpts) -> Result<Node>;
}

// Custom Debug Impls
//
// We don't use the default debug impls becaus the
// Rc<RefCell<Match<Node::Branch(Branch::Paragraph(...))>>>
//
// ... levels of indirection make it impossible to digest the output.

impl<'a> std::fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Whether something is a leaf or a branch is pretty internal, don't bother
        // with exposing this in debugging output
        //
        // These enum variants have types which have the same name as themselves
        // Branch::Paragraph(Paragraph(...)) is a lot of extra noise vs just Paragraph(...)
        // Skip over the Match struct since the start/end values really clutter the output
        if f.alternate() {
            match self {
                Node::Root(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
                Node::Heading(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
                Node::Block(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
                Node::Link(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
                Node::Paragraph(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
                Node::Italic(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
                Node::Bold(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
                Node::StrikeThrough(inner) => {
                    f.write_fmt(format_args!("{:#?}", inner.borrow().obj))
                }
                Node::Underline(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
                Node::PlainList(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),

                Node::BlankLine(_) => f.write_str("BlankLine"),
                Node::SoftBreak(_) => f.write_str("SoftBreak"),
                Node::Plain(inner) => f.write_fmt(format_args!("{:#?}", inner.obj)),
                Node::MarkupEnd(inner) => f.write_fmt(format_args!("{:#?}", inner.obj)),
                Node::Verbatim(inner) => f.write_fmt(format_args!("{:#?}", inner.obj)),
                Node::Code(inner) => f.write_fmt(format_args!("{:#?}", inner.obj)),
                Node::Comment(inner) => f.write_fmt(format_args!("{:#?}", inner.obj)),
                Node::InlineSrc(inner) => f.write_fmt(format_args!("{:#?}", inner.obj)),
                Node::Keyword(inner) => f.write_fmt(format_args!("{:#?}", inner.obj)),
            }
        } else {
            match self {
                Node::Root(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::Heading(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::Block(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::Link(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::Paragraph(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::Italic(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::Bold(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::StrikeThrough(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::Underline(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
                Node::PlainList(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),

                Node::BlankLine(_) => f.write_str("BlankLine"),
                Node::SoftBreak(_) => f.write_str("SoftBreak"),
                Node::Plain(inner) => f.write_fmt(format_args!("{:?}", inner.obj)),
                Node::MarkupEnd(inner) => f.write_fmt(format_args!("{:?}", inner.obj)),
                Node::Verbatim(inner) => f.write_fmt(format_args!("{:?}", inner.obj)),
                Node::Code(inner) => f.write_fmt(format_args!("{:?}", inner.obj)),
                Node::Comment(inner) => f.write_fmt(format_args!("{:?}", inner.obj)),
                Node::InlineSrc(inner) => f.write_fmt(format_args!("{:?}", inner.obj)),
                Node::Keyword(inner) => f.write_fmt(format_args!("{:?}", inner.obj)),
            }
        }
    }
}


        }
    }
}
