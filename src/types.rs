use derive_more::From;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

use crate::element::{Block, Comment, Heading, Keyword, Paragraph};
use crate::object::{Bold, Code, InlineSrc, Italic, Link, StrikeThrough, Underline, Verbatim};
use bitflags::bitflags;

pub enum Node<'a> {
    Leaf(Match<Leaf<'a>>),
    Branch(Rc<RefCell<Match<Branch<'a>>>>),
}

#[derive(From)]
pub enum Branch<'a> {
    Root(Vec<Node<'a>>),
    Keyword(Keyword<'a>),
    Heading(Heading<'a>),
    Block(Block<'a>),
    Link(Link<'a>),
    Paragraph(Paragraph<'a>),
    Italic(Italic<'a>),
    Bold(Bold<'a>),
    StrikeThrough(StrikeThrough<'a>),
    Underline(Underline<'a>),
}

#[derive(Clone, Copy, From)]
pub enum Leaf<'a> {
    Plain(&'a str),
    SoftBreak,
    Eof,
    MarkupEnd(MarkupKind),
    InlineSrc(InlineSrc<'a>),
    Verbatim(Verbatim<'a>),
    Code(Code<'a>),
    Keyword(Keyword<'a>),
    Comment(Comment<'a>),
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
        Branch<'a>: From<T>,
    {
        Node::Branch(Rc::new(RefCell::new(Match {
            obj: Branch::from(branch),
            start,
            end,
        })))
    }

    pub fn make_leaf<T>(leaf: T, start: usize, end: usize) -> Node<'a>
    where
        Leaf<'a>: From<T>,
    {
        Node::Leaf(Match {
            obj: Leaf::from(leaf),
            start,
            end,
        })
    }

    pub fn get_start(&self) -> usize {
        match self {
            Node::Leaf(val) => val.start,
            Node::Branch(val) => val.borrow().start,
        }
    }
    pub fn get_end(&self) -> usize {
        match self {
            Node::Leaf(val) => val.end,
            Node::Branch(val) => val.borrow().end,
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
        // Skip over the Match struct since the start/end values really clutter the output
        if f.alternate() {
            match self {
                Node::Leaf(inner) => f.write_fmt(format_args!("{:#?}", inner.obj)),
                Node::Branch(inner) => f.write_fmt(format_args!("{:#?}", inner.borrow().obj)),
            }
        } else {
            match self {
                Node::Leaf(inner) => f.write_fmt(format_args!("{:?}", inner.obj)),
                Node::Branch(inner) => f.write_fmt(format_args!("{:?}", inner.borrow().obj)),
            }
        }
    }
}

#[rustfmt::skip]
impl<'a> std::fmt::Debug for Branch<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // These enum variants have types which have the same name as themselves
        // Branch::Paragraph(Paragraph(...)) is a lot of extra noise vs just Paragraph(...)

        if f.alternate() {
            match self {
                Branch::Root          (inner) => f.write_fmt(format_args!("Root: {:#?}", inner)),
                Branch::Keyword       (inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Branch::Heading       (inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Branch::Block         (inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Branch::Link          (inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Branch::Paragraph     (inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Branch::Italic        (inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Branch::Bold          (inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Branch::StrikeThrough (inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Branch::Underline     (inner) => f.write_fmt(format_args!("{:#?}", inner)),
            }
        } else {
            match self {
                Branch::Root          (inner) => f.write_fmt(format_args!("Root: {:?}", inner)),
                Branch::Keyword       (inner) => f.write_fmt(format_args!("{:?}", inner)),
                Branch::Heading       (inner) => f.write_fmt(format_args!("{:?}", inner)),
                Branch::Block         (inner) => f.write_fmt(format_args!("{:?}", inner)),
                Branch::Link          (inner) => f.write_fmt(format_args!("{:?}", inner)),
                Branch::Paragraph     (inner) => f.write_fmt(format_args!("{:?}", inner)),
                Branch::Italic        (inner) => f.write_fmt(format_args!("{:?}", inner)),
                Branch::Bold          (inner) => f.write_fmt(format_args!("{:?}", inner)),
                Branch::StrikeThrough (inner) => f.write_fmt(format_args!("{:?}", inner)),
                Branch::Underline     (inner) => f.write_fmt(format_args!("{:?}", inner)),
            }
        }
    }
}

#[rustfmt::skip]
impl<'a> std::fmt::Debug for Leaf<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // These enum variants have types which have the same name as themselves

        if f.alternate() {
            match self {
                Leaf::Plain(inner)     => f.write_fmt(format_args!("{:#?}", inner)),
                Leaf::SoftBreak        => f.write_str("SoftBreak"),
                Leaf::Eof              => f.write_str("EOF"),
                Leaf::MarkupEnd(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Leaf::InlineSrc(inner) => f.write_fmt(format_args!("{:#?}", inner)),
                Leaf::Verbatim(inner)  => f.write_fmt(format_args!("{:#?}", inner)),
                Leaf::Code(inner)      => f.write_fmt(format_args!("{:#?}", inner)),
                Leaf::Keyword(inner)   => f.write_fmt(format_args!("{:#?}", inner)),
                Leaf::Comment(inner)   => f.write_fmt(format_args!("{:?}", inner)),
            }
        } else {
            match self {
                Leaf::Plain(inner)     => f.write_fmt(format_args!("{:?}", inner)),
                Leaf::SoftBreak        => f.write_str("SoftBreak"),
                Leaf::Eof              => f.write_str("EOF"),
                Leaf::MarkupEnd(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Leaf::InlineSrc(inner) => f.write_fmt(format_args!("{:?}", inner)),
                Leaf::Verbatim(inner)  => f.write_fmt(format_args!("{:?}", inner)),
                Leaf::Code(inner)      => f.write_fmt(format_args!("{:?}", inner)),
                Leaf::Keyword(inner)   => f.write_fmt(format_args!("{:#?}", inner)),
                Leaf::Comment(inner)   => f.write_fmt(format_args!("{:#?}", inner)),
            }
        }
    }
}
