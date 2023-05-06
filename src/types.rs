use derive_more::From;
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use crate::element::{Block, Heading, Keyword, Paragraph};
use crate::object::{Bold, Code, InlineSrc, Italic, Link, StrikeThrough, Underline, Verbatim};
use bitflags::bitflags;
use std::ops::Deref;

// #[derive(Debug)]
// pub struct MatchError;
// impl Error for MatchError {}

// #[derive(Debug)]
// pub struct EofError;
// impl Error for EofError {}

#[derive(Debug)]
pub enum MatchError {
    InvalidLogic,
    EofError,
}

pub type Result<T> = std::result::Result<T, MatchError>;

// #[derive(Debug)]
// pub struct ParseNode<'a>(pub Rc<RefCell<Match<Node<'a>>>>);

pub type Cache<'a> = RefCell<std::collections::HashMap<usize, Node<'a>>>;

// impl<'a> Node<'a> {
//     pub fn new(node: Match<Node<'a>>) -> Self {
//         Node(Rc::new(RefCell::new(node)))
//     }

//     // pub fn get_obj(&self) -> &Node {
//     //     &self.borrow().obj
//     // }

//     // pub fn clone(&self) -> ParseNode {
//     //     ParseNode(Rc::clone(&self))
//     // }
// }

// impl<'a> Deref for Node<'a> {
//     type Target = Rc<RefCell<Match<Node<'a>>>>;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

impl std::fmt::Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "unsuccesful match")
    }
}

// impl std::fmt::Display for EofError {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "end of file")
//     }
// }

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

#[derive(Debug)]
pub enum Node<'a> {
    Leaf(Match<Leaf<'a>>),
    Branch(Rc<RefCell<Match<Branch<'a>>>>),
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

    pub fn clone(self) -> Node<'a> {
        match self {
            Node::Branch(val) => Node::Branch(val.clone()),
            Node::Leaf(val) => Node::Leaf(val),
        }
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

// impl<'a> From<Link<'a>> for Branch<'a> {
//     fn from(value: Link<'a>) -> Self {
//         Branch::Link(value)
//     }
// }

// impl<'a> From<Keyword<'a>> for Branch<'a> {
//     fn from(value: Keyword<'a>) -> Self {
//         Branch::Keyword(value)
//     }
// }
// impl<'a> From<Paragraph<'a>> for Branch<'a> {
//     fn from(value: Paragraph<'a>) -> Self {
//         Branch::Paragraph(value)
//     }
// }

// impl<'a> From<Heading<'a>> for Branch<'a> {
//     fn from(value: Heading<'a>) -> Self {
//         Branch::Heading(value)
//     }
// }

#[derive(Debug, From)]
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


#[derive(Debug, Clone, Copy, From)]
pub enum Leaf<'a> {
    Plain(&'a str),
    SoftBreak,
    Eof,
    MarkupEnd(MarkupKind),
    InlineSrc(InlineSrc<'a>),
    Verbatim(Verbatim<'a>),
    Code(Code<'a>),
}

// #[derive(Debug)]
// pub enum ObjectMinimal<'a> {
//     Plain(&'a str),
//     Markup(MarkupKind, Vec<ObjectMinimal<'a>>),
//     // Entity(StrType)
// }

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

// TODO: maybe make all fields bitflags for space optimization
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ParseOpts {
    pub allow_newline: bool,
    pub from_paragraph: bool,
    pub prev_newline: bool,
    pub from_object: bool,
    pub in_link: bool,
    pub markup: MarkupKind,
}
