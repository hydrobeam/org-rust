use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

use crate::types::{Expr, Node};

#[derive(Clone, Copy)]
pub struct NodeID(u32);

impl std::fmt::Debug for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug)]
pub struct NodePool<'a> {
    pub inner_vec: Vec<Node<'a>>,
    pub counter: u32,
}

impl<'a> NodePool<'a> {
    pub(crate) fn new() -> Self {
        Self {
            inner_vec: Vec::new(),
            /// The next free index in the pool.
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
        self.inner_vec[target_id.0 as usize] = Node::new(obj, start, end, parent);

        target_id
    }

    pub fn get(&self, id: NodeID) -> Option<&'a Node> {
        self.inner_vec.get(id.0 as usize)
    }

    pub fn get_mut(&mut self, id: NodeID) -> Option<&'a mut Node> {
        self.inner_vec.get_mut(id.0 as usize)
    }

    /// Allocates a defualt Node at in index and returns its index.
    ///
    /// To be used when intending to replace the Node at the index
    /// in conjunction with `alloc_from_id`.
    ///
    pub(crate) fn reserve_id(&mut self) -> NodeID {
        self.inner_vec.push(Node::default());
        let old_counter = self.counter;
        self.counter += 1;
        NodeID(old_counter)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<'a>> {
        IntoIterator::into_iter(&self.inner_vec)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Node<'a>> {
        IntoIterator::into_iter(&mut self.inner_vec)
    }

    pub fn root(&self) -> &Node {
        &self.inner_vec[0]
    }

    pub fn root_id(&self) -> NodeID {
        NodeID(0)
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
