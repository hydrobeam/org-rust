use derive_more::From;
use std::collections::HashMap;
use std::fmt::{Debug, Write};
use std::ops::Index;
use std::rc::Rc;

use crate::constants::{EQUAL, PLUS, RBRACE, RBRACK, SLASH, SPACE, STAR, TILDE, UNDERSCORE, VBAR};
use crate::element::*;
use crate::node_pool::{NodeID, NodePool};
use crate::object::*;
use crate::utils::{bytes_to_str, id_escape, Match};
use bitflags::bitflags;

pub(crate) type Result<T> = std::result::Result<T, MatchError>;

pub type NodeCache = HashMap<usize, NodeID>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attr<'a> {
    pub key: &'a str,
    pub val: &'a str,
}

#[derive(Debug)]
pub struct Parser<'a> {
    pub pool: NodePool<'a>,
    pub(crate) cache: NodeCache,
    // target names to uuids
    pub targets: HashMap<&'a str, Rc<str>>,
    // uuids to number of times they occur, we increment name
    // like uuid-1 if there are duplicates
    // used to help ensure no duplicates are being inserted
    pub(crate) target_occurences: HashMap<Rc<str>, usize>,
    // name to macro def
    pub macros: HashMap<&'a str, NodeID>,

    // basic keywords, key: val
    pub keywords: HashMap<&'a str, &'a str>,

    // footnote label to footnote definition
    pub footnotes: HashMap<&'a str, NodeID>,

    pub source: &'a str,
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

    pub fn root(&self) -> &Node {
        self.pool.root()
    }

    pub fn print_tree(&self) {
        self.pool.print_tree();
    }

    /// Creates a unique id based on the raw contents of the item
    /// use an Rc<str> since the generated id will also be stored in the node
    /// and in target_occurences.
    /// and we'd like not to triple allocate
    pub(crate) fn generate_target(&mut self, raw_entry: &'a str) -> Rc<str> {
        let mut id_string = id_escape(raw_entry);
        // doesn't compile if we're not explicit about the coercion
        let rc_ret: Rc<str>;
        if let Some(counter) = self.target_occurences.get_mut(&id_string as &str) {
            *counter += 1;
            write!(id_string, "-{counter}").unwrap();
            rc_ret = id_string.into();
        } else {
            rc_ret = id_string.into();
            self.targets.entry(raw_entry).or_insert(rc_ret.clone());
        }

        self.target_occurences.insert(rc_ret.clone(), 0);
        rc_ret.clone()
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

    #[inline]
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
    pub id_target: Option<Rc<str>>,
    pub attrs: HashMap<String, Vec<Attr<'a>>>,
}

impl<'a> Default for Node<'a> {
    fn default() -> Self {
        Self {
            obj: Expr::BlankLine,
            start: Default::default(),
            end: Default::default(),
            parent: Option::default(),
            id_target: None,
            attrs: HashMap::new(),
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
            id_target: None,
            attrs: HashMap::new(),
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
    PlainLink(PlainLink<'a>),
    Superscript(Superscript<'a>),
    Subscript(Subscript<'a>),
    Drawer(Drawer<'a>),
    Affiliated(Affiliated<'a>),
    FootnoteDef(FootnoteDef<'a>),
    FootnoteRef(FootnoteRef<'a>),

    // Leaf
    BlankLine,
    SoftBreak,
    LineBreak,
    HorizontalRule,
    // Normal
    Plain(&'a str),
    Verbatim(Verbatim<'a>),
    Code(Code<'a>),
    Comment(Comment<'a>),
    InlineSrc(InlineSrc<'a>),
    Keyword(Keyword<'a>),
    LatexEnv(LatexEnv<'a>),
    LatexFragment(LatexFragment<'a>),
    Entity(Entity<'a>),
    Emoji(Emoji<'a>),
    Target(Target<'a>),
    Macro(MacroCall<'a>),
    ExportSnippet(ExportSnippet<'a>),
    MacroDef(MacroDef<'a>),
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
    MarkupEnd(MarkupKind),
}

impl std::fmt::Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "unsuccesful match")
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub(crate) struct MarkupKind: u32 {
        const Italic        = 1 << 0;
        const Bold          = 1 << 1;
        const Underline     = 1 << 2;
        const StrikeThrough = 1 << 3;
        const Verbatim      = 1 << 4;
        const Code          = 1 << 5;
        const Link          = 1 << 6;
        const Table         = 1 << 7;
        const SupSub        = 1 << 8;
        const FootnoteRef   = 1 << 9;
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
            RBRACK => self.contains(MarkupKind::Link) || self.contains(MarkupKind::Link),
            TILDE => self.contains(MarkupKind::Code),
            EQUAL => self.contains(MarkupKind::Verbatim),
            VBAR => self.contains(MarkupKind::Table),
            RBRACE => self.contains(MarkupKind::SupSub),
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
    pub fn children_mut(&mut self) -> Option<&mut Vec<NodeID>> {
        match self {
            Expr::Root(root) => Some(root),
            Expr::Heading(heading) => heading.children.as_mut(),
            Expr::Block(block) => match block {
                Block::Center { contents, .. }
                | Block::Quote { contents, .. }
                | Block::Special { contents, .. } => Some(contents),
                _ => None,
            },

            Expr::RegularLink(link) => link.description.as_mut(),
            Expr::Paragraph(par) => Some(&mut par.0),
            Expr::Italic(it) => Some(&mut it.0),
            Expr::Bold(bo) => Some(&mut bo.0),
            Expr::StrikeThrough(st) => Some(&mut st.0),
            Expr::Underline(un) => Some(&mut un.0),
            Expr::PlainList(pl) => Some(&mut pl.children),
            Expr::Item(item) => Some(&mut item.children),
            Expr::Table(inner) => Some(&mut inner.children),
            Expr::TableRow(ref mut inner) => match inner {
                TableRow::Rule => None,
                TableRow::Standard(stan) => Some(stan),
            },
            Expr::TableCell(inner) => Some(&mut inner.0),
            Expr::Superscript(inner) => match &mut inner.0 {
                PlainOrRec::Plain(_) => None,
                PlainOrRec::Rec(rec) => Some(rec),
            },
            Expr::Subscript(inner) => match &mut inner.0 {
                PlainOrRec::Plain(_) => None,
                PlainOrRec::Rec(rec) => Some(rec),
            },
            Expr::Drawer(inner) => Some(&mut inner.children),
            Expr::FootnoteDef(inner) => Some(&mut inner.children),
            Expr::FootnoteRef(inner) => inner.children.as_mut(),
            _ => None,
        }
    }

    pub fn children(&self) -> Option<&Vec<NodeID>> {
        match &self {
            Expr::Root(root) => Some(root),
            Expr::Heading(heading) => heading.children.as_ref(),
            Expr::Block(block) => match block {
                Block::Center { contents, .. }
                | Block::Quote { contents, .. }
                | Block::Special { contents, .. } => Some(contents),
                _ => None,
            },

            Expr::RegularLink(link) => link.description.as_ref(),
            Expr::Paragraph(par) => Some(&par.0),
            Expr::Italic(it) => Some(&it.0),
            Expr::Bold(bo) => Some(&bo.0),
            Expr::StrikeThrough(st) => Some(&st.0),
            Expr::Underline(un) => Some(&un.0),
            Expr::PlainList(pl) => Some(&pl.children),
            Expr::Item(item) => Some(&item.children),
            Expr::Table(inner) => Some(&inner.children),
            Expr::TableRow(inner) => match &inner {
                TableRow::Rule => None,
                TableRow::Standard(stan) => Some(stan),
            },
            Expr::TableCell(inner) => Some(&inner.0),
            Expr::Superscript(inner) => match &inner.0 {
                PlainOrRec::Plain(_) => None,
                PlainOrRec::Rec(rec) => Some(rec),
            },
            Expr::Subscript(inner) => match &inner.0 {
                PlainOrRec::Plain(_) => None,
                PlainOrRec::Rec(rec) => Some(rec),
            },
            Expr::Drawer(inner) => Some(&inner.children),
            Expr::FootnoteDef(inner) => Some(&inner.children),
            Expr::FootnoteRef(inner) => inner.children.as_ref(),
            _ => None,
        }
    }

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
                println!("properties: {:#?}", inner.properties);
                print!("title: ");
                if let Some(title) = &inner.title {
                    for id in &title.1 {
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
            Expr::Block(inner) => {
                println!("Block{{");
                match inner {
                    Block::Center { contents, .. }
                    | Block::Quote { contents, .. }
                    | Block::Special { contents, .. } => {
                        for id in contents {
                            pool[*id].obj.print_tree(pool);
                            print!(",");
                        }
                    }
                    Block::Comment { contents, .. }
                    | Block::Example { contents, .. }
                    | Block::Export { contents, .. }
                    | Block::Src { contents, .. }
                    | Block::Verse { contents, .. } => {
                        println!("{contents:#?}");
                    }
                }
                print!("\nEndBlock}}");
            }
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
            Expr::LineBreak => print!("LineBreak"),
            Expr::HorizontalRule => print!("HorizontalRule"),
            Expr::Plain(inner) => print!("{inner:#?}"),
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
            Expr::Emoji(inner) => print!("{inner:#?}"),
            Expr::Superscript(inner) => print!("{inner:#?}"),
            Expr::Subscript(inner) => print!("{inner:#?}"),
            Expr::Target(inner) => print!("{inner:#?}"),
            Expr::Macro(inner) => print!("{inner:#?}"),
            Expr::Drawer(inner) => print!("{inner:#?}"),
            Expr::ExportSnippet(inner) => print!("{inner:#?}"),
            Expr::Affiliated(inner) => print!("{inner:#?}"),
            Expr::MacroDef(inner) => print!("{inner:#?}"),
            Expr::FootnoteDef(inner) => print!("{inner:#?}"),
            Expr::FootnoteRef(inner) => print!("{inner:#?}"),
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
                Expr::LineBreak => f.write_str("LineBreak"),
                Expr::HorizontalRule => f.write_str("HorizontalRule"),
                Expr::Plain(inner) => f.write_fmt(format_args!("{inner:#?}")),
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
                Expr::Emoji(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Superscript(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Subscript(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Target(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Macro(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Drawer(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::ExportSnippet(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::Affiliated(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::MacroDef(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::FootnoteDef(inner) => f.write_fmt(format_args!("{inner:#?}")),
                Expr::FootnoteRef(inner) => f.write_fmt(format_args!("{inner:#?}")),
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
                Expr::FootnoteDef(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::FootnoteRef(inner) => f.write_fmt(format_args!("{inner:?}")),

                Expr::BlankLine => f.write_str("BlankLine"),
                Expr::SoftBreak => f.write_str("SoftBreak"),
                Expr::LineBreak => f.write_str("LineBreak"),
                Expr::HorizontalRule => f.write_str("HorizontalRule"),
                Expr::Plain(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Verbatim(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Code(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Comment(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::InlineSrc(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Keyword(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Entity(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Table(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::TableRow(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::TableCell(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Emoji(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Superscript(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Subscript(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Target(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Macro(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Drawer(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::ExportSnippet(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::Affiliated(inner) => f.write_fmt(format_args!("{inner:?}")),
                Expr::MacroDef(inner) => f.write_fmt(format_args!("{inner:?}")),
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
