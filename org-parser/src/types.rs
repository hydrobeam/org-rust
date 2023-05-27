use derive_more::From;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Index;

use crate::constants::{EQUAL, PLUS, RBRACK, SLASH, SPACE, STAR, TILDE, UNDERSCORE, VBAR};
use crate::element::{
    Block, BlockContents, Comment, Heading, Item, Keyword, LatexEnv, Paragraph, PlainList, Table,
    TableCell, TableRow,
};
use crate::node_pool::{NodeID, NodePool};
use crate::object::{
    Bold, Code, Entity, InlineSrc, Italic, LatexFragment, PlainLink, RegularLink, StrikeThrough,
    Underline, Verbatim,
};
use crate::utils::{bytes_to_str, Match};
use bitflags::bitflags;

pub(crate) type Result<T> = std::result::Result<T, MatchError>;

pub type NodeCache = HashMap<usize, NodeID>;

pub(crate) struct Parser<'a> {
    pub pool: NodePool<'a>,
    pub cache: NodeCache,
}

impl<'a> Parser<'a> {
    pub(crate) fn alloc<T>(
        &mut self,
        obj: T,
        start: usize,
        end: usize,
        parent: Option<NodeID>,
    ) -> NodeID
    where
        Expr<'a>: From<T>,
    {
        let ret = self.pool.alloc(obj, start, end, parent);
        self.cache.insert(start, ret);
        ret
    }

    /// Allocates a node in the pool at a given location.
    ///
    /// Returns the index that was allocated.
    ///
    /// Works well with [`NodePool::reserve_id`].
    ///
    /// # Safety:
    ///
    /// Must refer to an ID that already exists in the pool.
    /// Will panic at runtime otherwise.
    ///
    pub(crate) fn alloc_with_id<T>(
        &mut self,
        obj: T,
        start: usize,
        end: usize,
        parent: Option<NodeID>,
        target_id: NodeID,
    ) -> NodeID
    where
        Expr<'a>: From<T>,
    {
        self.pool.alloc_with_id(obj, start, end, parent, target_id);
        self.cache.insert(start, target_id);
        target_id
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Cursor<'a> {
    pub byte_arr: &'a [u8],
    pub index: usize,
}

impl<'a> std::ops::Deref for Cursor<'a> {
    type Target = &'a [u8];

    fn deref(&self) -> &Self::Target {
        &self.byte_arr
    }
}

impl<'a> Cursor<'a> {
    pub fn new(byte_arr: &'a [u8]) -> Self {
        Self { byte_arr, index: 0 }
    }

    pub fn peek(&self, diff: usize) -> Result<u8> {
        assert!(diff > 0);
        self.byte_arr
            .get(self.index + diff)
            .copied()
            .ok_or(MatchError::EofError)
    }

    pub fn peek_rev(&self, diff: usize) -> Result<u8> {
        assert!(diff > 0);

        // handle access this way in case of underflow
        self.index
            .checked_sub(diff)
            .and_then(|num| self.byte_arr.get(num))
            .copied()
            .ok_or(MatchError::EofError)
    }

    pub fn advance(&mut self, diff: usize) {
        self.index += diff;
    }

    pub fn move_to(&mut self, loc: usize) {
        self.index = loc;
    }

    pub fn adv_copy(mut self, diff: usize) -> Self {
        self.index += diff;
        self
    }

    pub fn move_to_copy(mut self, loc: usize) -> Self {
        self.index = loc;
        self
    }

    pub fn next(&mut self) {
        self.index += 1;
    }

    pub fn prev(&mut self) {
        self.index -= 1;
    }

    pub fn curr(&self) -> u8 {
        self.byte_arr[self.index]
    }

    pub fn try_curr(&self) -> Result<u8> {
        self.byte_arr
            .get(self.index)
            .copied()
            .ok_or(MatchError::EofError)
    }

    pub fn is_index_valid(&self) -> Result<()> {
        if self.index < self.byte_arr.len() {
            Ok(())
        } else {
            Err(MatchError::EofError)
        }
    }

    pub fn word(&mut self, word: &str) -> Result<()> {
        if self.byte_arr[self.index..].starts_with(word.as_bytes()) {
            self.index += word.len();
            Ok(())
        } else {
            Err(MatchError::InvalidLogic)
        }
    }

    pub fn skip_ws(&mut self) {
        while self.curr() == SPACE {
            self.next();
        }
    }

    pub fn clamp_backwards(self, start: usize) -> &'a str {
        bytes_to_str(&self.byte_arr[start..self.index])
    }

    pub fn clamp_forwards(self, end: usize) -> &'a str {
        bytes_to_str(&self.byte_arr[self.index..end])
    }

    pub fn clamp(self, start: usize, end: usize) -> &'a str {
        bytes_to_str(&self.byte_arr[start..end])
    }

    pub fn adv_till_byte(&mut self, byte: u8) {
        self.index = self.byte_arr[self.index..]
            .iter()
            .position(|&x| x == byte)
            .unwrap_or(self.byte_arr[self.index..].len()) // EOF case, just go to the end
            + self.index;
    }

    pub fn fn_until(self, func: impl Fn(u8) -> bool) -> Result<Match<&'a str>> {
        let ret = self.byte_arr[self.index..]
            .iter()
            .position(|x| func(*x))
            .ok_or(MatchError::EofError)?
            + self.index;

        Ok(Match {
            start: self.index,
            end: ret,
            obj: self.clamp_forwards(ret),
        })
    }

    pub fn fn_while(self, func: impl Fn(u8) -> bool) -> Result<Match<&'a str>> {
        let ret = self.byte_arr[self.index..]
            .iter()
            .position(|x| !func(*x))
            .ok_or(MatchError::EofError)?
            + self.index;

        Ok(Match {
            start: self.index,
            end: ret,
            obj: self.clamp_forwards(ret),
        })
    }

    pub fn rest(&self) -> &[u8] {
        &self.byte_arr[self.index..]
    }

    pub fn cut_off(mut self, loc: usize) -> Self {
        self.byte_arr = &self.byte_arr[..loc];
        self
    }

    pub fn clamp_off(mut self, begin: usize, end: usize) -> Self {
        self.byte_arr = &self.byte_arr[begin..end];
        self.index = 0;
        self
    }
}

impl<'a> Index<usize> for Cursor<'a> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.byte_arr[index]
    }
}

#[derive(Clone, Debug)]
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
            parent: Option::default(),
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
    RegularLink(RegularLink<'a>),
    Paragraph(Paragraph),
    Italic(Italic),
    Bold(Bold),
    StrikeThrough(StrikeThrough),
    Underline(Underline),
    PlainList(PlainList),
    Item(Item<'a>),
    Table(Table),
    TableRow(TableRow),
    TableCell(TableCell),

    // Leaf
    BlankLine,
    SoftBreak,
    // Normal
    Plain(&'a str),
    MarkupEnd(MarkupKind),
    Verbatim(Verbatim<'a>),
    Code(Code<'a>),
    Comment(Comment<'a>),
    InlineSrc(InlineSrc<'a>),
    Keyword(Keyword<'a>),
    LatexEnv(LatexEnv<'a>),
    LatexFragment(LatexFragment<'a>),
    PlainLink(PlainLink<'a>),
    Entity(Entity<'a>),
}

// TODO: maybe make all fields bitflags for space optimization
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ParseOpts {
    pub from_paragraph: bool,
    pub from_object: bool,
    pub from_list: bool,
    pub list_line: bool,
    pub markup: MarkupKind,
    pub indentation_level: u8,
}

#[derive(Debug)]
pub(crate) enum MatchError {
    InvalidLogic,
    EofError,
    InvalidIndentation,
}

impl std::fmt::Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "unsuccesful match")
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    // TODO: make Expr::MarkupEnd an Error thing so that this can be private
    pub struct MarkupKind: u32 {
        const Italic        = 1 << 0;
        const Bold          = 1 << 1;
        const Underline     = 1 << 2;
        const StrikeThrough = 1 << 3;
        const Verbatim      = 1 << 4;
        const Code          = 1 << 5;
        const Link          = 1 << 6;
        const Table         = 1 << 7;
    }
}

impl MarkupKind {
    /// For use in plain markup types (code & verbatim)
    /// to determine if they have hit an end marker in a nested
    /// markup situation. Checks if an incoming byte would close
    /// the markup that is held.
    ///
    ///
    /// /abc ~one tw/ o~
    /// should be:
    ///     Italic{abc ~one tw} o~
    ///
    /// not:
    ///    /abc Code{one tw/ o}
    ///
    pub(crate) fn byte_match(self, byte: u8) -> bool {
        match byte {
            STAR => self.contains(MarkupKind::Bold),
            SLASH => self.contains(MarkupKind::Italic),
            UNDERSCORE => self.contains(MarkupKind::Underline),
            PLUS => self.contains(MarkupKind::StrikeThrough),
            RBRACK => self.contains(MarkupKind::Link),
            TILDE => self.contains(MarkupKind::Code),
            EQUAL => self.contains(MarkupKind::Verbatim),
            VBAR => self.contains(MarkupKind::Table),
            _ => false,
        }
    }
}

pub(crate) trait Parseable<'a> {
    fn parse(
        parser: &mut Parser<'a>,
        cursor: Cursor<'a>,
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
            Expr::LatexFragment(inner) => print!("{inner:#?}"),
            Expr::Root(inner) => {
                print!("Root(");
                for id in inner {
                    // print!("{:#?}: ", id);
                    pool[*id].obj.print_tree(pool);
                    println!();
                }
                print!(")");
            }
            Expr::Heading(inner) => {
                println!("Heading {{");
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
                println!();
                print!("children: [");
                if let Some(children) = &inner.children {
                    for id in children {
                        // print!("{:#?}: ", id);
                        pool[*id].obj.print_tree(pool);
                        print!(", ");
                    }
                }
                print!("]");
                print!("}}");
            }
            Expr::Block(inner) => match &inner.contents {
                BlockContents::Greater(children) => {
                    println!("Block{{");
                    for id in children {
                        pool[*id].obj.print_tree(pool);
                        print!(",");
                    }
                    print!("\nEndBlock}}");
                }
                BlockContents::Lesser(cont) => {
                    println!("{inner:#?}");
                }
            },
            Expr::RegularLink(inner) => {
                println!("RegularLink{{");
                print!("{:#?}", inner.path);
                if let Some(children) = &inner.description {
                    for id in children {
                        pool[*id].obj.print_tree(pool);
                        print!(",");
                    }
                }
                println!("}}");
            }

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
            Expr::PlainList(inner) => {
                print!("PlainList{{");
                for id in &inner.children {
                    pool[*id].obj.print_tree(pool);
                }
                print!("}}");
            }
            Expr::BlankLine => print!("BlankLine"),
            Expr::SoftBreak => print!("SoftBreak"),
            Expr::Plain(inner) => print!("{inner:#?}"),
            Expr::MarkupEnd(inner) => print!("{inner:#?}"),
            Expr::Verbatim(inner) => print!("{inner:#?}"),
            Expr::Code(inner) => print!("{inner:#?}"),
            Expr::Comment(inner) => print!("{inner:#?}"),
            Expr::InlineSrc(inner) => print!("{inner:#?}"),
            Expr::Keyword(inner) => print!("{inner:#?}"),
            Expr::LatexEnv(inner) => print!("{inner:#?}"),
            Expr::Item(inner) => {
                print!("Item{{");
                for id in &inner.children {
                    pool[*id].obj.print_tree(pool);
                }
                print!("}}");
            }
            Expr::PlainLink(inner) => print!("{inner:#?}"),
            Expr::Entity(inner) => print!("{inner:#?}"),
            Expr::Table(inner) => {
                println!("Table{{");
                for id in &inner.children {
                    pool[*id].obj.print_tree(pool);
                }
                print!("\n}}");
                dbg!(inner);
            }

            Expr::TableRow(inner) => {
                if let TableRow::Standard(stans) = inner {
                    print!("|");
                    for id in stans {
                        pool[*id].obj.print_tree(pool);
                    }
                } else {
                    print!("h-rule");
                }

                println!();
            }
            Expr::TableCell(inner) => {
                for id in &inner.0 {
                    pool[*id].obj.print_tree(pool);
                }
                print!("|");
            }
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
                Expr::PlainLink(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Item(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::LatexFragment(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Root(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Heading(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Block(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::RegularLink(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Paragraph(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Italic(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Bold(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::StrikeThrough(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Underline(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::PlainList(inner) => f.write_fmt(format_args!("{inner:#?}")),

                Expr::BlankLine => f.write_str("BlankLine"),
                Expr::SoftBreak => f.write_str("SoftBreak"),
                Expr::Plain(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::MarkupEnd(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Verbatim(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Code(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Comment(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::InlineSrc(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Keyword(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::LatexEnv(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Entity(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Table(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::TableRow(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::TableCell(inner) => f.write_fmt(format_args!("{inner:#?}")),
            }
        } else {
            match self {
                Expr::PlainLink(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Item(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::LatexFragment(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::LatexEnv(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Root(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Heading(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Block(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::RegularLink(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Paragraph(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Italic(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Bold(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::StrikeThrough(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Underline(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::PlainList(inner) => f.write_fmt(format_args!("{inner:?}")),

                Expr::BlankLine => f.write_str("BlankLine"),
                Expr::SoftBreak => f.write_str("SoftBreak"),
                Expr::Plain(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::MarkupEnd(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Verbatim(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Code(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Comment(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::InlineSrc(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Keyword(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Entity(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Table(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::TableRow(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::TableCell(inner) => f.write_fmt(format_args!("{inner:?}")),
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
