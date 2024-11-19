use std::fmt::{Debug, Display};
use std::ops::{Index, IndexMut};

use crate::types::{Expr, Node};

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq)]
/// Identifier for [`Node`]s in a [`NodePool`].
///
/// NodeIDs are guaranteed to be unique to each node since they are assigned
/// sequentially and cannot re-used.
pub struct NodeID(u32);

/// This exists ONLY for testing purposes
pub(crate) fn make_node_id(id: u32) -> NodeID {
    NodeID(id)
}

impl Display for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl std::fmt::Debug for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

/// Arena-based container storing [`Node`]s
///
/// A [`NodePool`] is essentially just a [`Vec<Node>`] that is indexed by [`NodeID`]s.
/// Each parsing construct stores [`NodeID`]s which refer to indices within the [`NodePool`].
/// It acts like a flattened version of the AST where each node can be easily iterated over
/// and tracked.
///
/// [`NodePool::iter`] and [`NodePool::iter_mut`] can be used for iterating over the contents of
/// the pool.
///
/// # Safety
///
/// Removing or inserting an element at a position other than the end might invalidate every other NodeID
/// and therefore the AST. Removing and de-allocating an individual [`Node`] is not possible due to
/// this effect. However, the result can be feigned in the tree by "deleting" the node from its parent.
/// See [`NodePool::delete_node`].
///
///
#[derive(Debug)]
pub struct NodePool<'a> {
    pub inner_vec: Vec<Node<'a>>,
    pub counter: u32,
}

impl<'a> NodePool<'a> {
    pub(crate) fn new() -> Self {
        Self {
            inner_vec: Vec::new(),
            // The next free index in the pool.
            counter: 0,
        }
    }

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
        let prev_id = self.counter;
        self.inner_vec.push(Node::new(obj, start, end, parent));
        self.counter += 1;
        NodeID(prev_id)
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
        self.inner_vec[target_id.0 as usize] = Node::new(obj, start, end, parent);

        target_id
    }

    pub fn get(&self, id: NodeID) -> Option<&'a Node> {
        self.inner_vec.get(id.0 as usize)
    }

    pub fn get_mut(&mut self, id: NodeID) -> Option<&'a mut Node> {
        self.inner_vec.get_mut(id.0 as usize)
    }

    /// Allocates a default Node at in index and returns its index.
    ///
    /// To be used when intending to replace the Node at the index
    /// in conjunction with `alloc_from_id`.
    pub(crate) fn reserve_id(&mut self) -> NodeID {
        self.inner_vec.push(Node::default());
        let old_counter = self.counter;
        self.counter += 1;
        NodeID(old_counter)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<'a>> + DoubleEndedIterator<Item = &Node<'a>> {
        self.inner_vec.iter()
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut Node<'a>> + DoubleEndedIterator<Item = &mut Node<'a>> {
        self.inner_vec.iter_mut()
    }

    /// Outputs a (somewhat) legible representation of the tree to stdout.
    pub fn print_tree(&self) {
        self.inner_vec[0].print_tree(self);
    }

    /// Returns the [`NodeID`] of the first element in the pool.
    pub fn root_id(&self) -> NodeID {
        NodeID(0)
    }

    /// Removes a [`Node`] from its parents' "children".
    ///
    /// This action mimicks the effect of a deletion, but does
    /// *not* actually deallocate or remove the node from the pool.
    pub fn delete_node(&mut self, index_id: NodeID) {
        let par_id = self[index_id].parent.unwrap();
        let par_node = &mut self[par_id];

        let children = par_node.obj.children_mut().unwrap();
        let index = children.iter().position(|&x| x == index_id).unwrap();
        children.remove(index);
    }
}

impl<'a> Index<NodeID> for NodePool<'a> {
    type Output = Node<'a>;

    fn index(&self, index: NodeID) -> &Self::Output {
        &self.inner_vec[index.0 as usize]
    }
}

impl<'a> IndexMut<NodeID> for NodePool<'a> {
    fn index_mut(&mut self, index: NodeID) -> &mut Self::Output {
        &mut self.inner_vec[index.0 as usize]
    }
}
