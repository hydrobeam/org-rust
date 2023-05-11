use derive_more::From;
use std::{
    collections::HashMap,
    ops::{Index, IndexMut}, fmt::{Debug, Write},
};

use crate::types::{Expr, Node};

#[derive(Clone, Copy)]
pub struct NodeID(u32);

impl std::fmt::Debug for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}



type Cache = HashMap<u32, NodeID>;

#[derive(Debug)]
pub struct NodePool<'a> {
    pub inner_vec: Vec<Node<'a>>,
    pub counter: u32,
}

impl<'a> NodePool<'a> {
    pub fn new() -> Self {
        Self {
            inner_vec: Vec::new(),
            counter: 0,
        }
    }

    pub fn alloc<T>(&mut self, obj: T, start: usize, end: usize, parent: Option<NodeID>) -> NodeID
    where
        Expr<'a>: From<T>,
    {
        let prev_id = self.counter;
        self.inner_vec.push(Node::new(obj, start, end, parent));
        self.counter += 1;
        NodeID(prev_id)
    }

    pub fn get(&self, id: NodeID) -> Option<&'a Node> {
        self.inner_vec.get(id.0 as usize)
    }

    pub fn get_mut(&mut self, id: NodeID) -> Option<&'a mut Node> {
        self.inner_vec.get_mut(id.0 as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<'a>> {
        IntoIterator::into_iter(&self.inner_vec)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Node<'a>> {
        IntoIterator::into_iter(&mut self.inner_vec)
    }

    // fn iter(&self) -> Iter<'a, Node> {
    //     self.inner_vec.iter()
    // }

    // fn iter_mut(&mut self) -> IterMut<'a, Node> {
    //     self.inner_vec.iter_mut()
    // }
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

// fn swag() {
//     // ret.alloc(obj, start, end, parent)
//     // ret.alloc(obj, start, end, parent)
//     // ret.alloc(obj, start, end, parent)
//     // ret.alloc(obj, start, end, parent)
//     // ret.alloc(obj, start, end, parent)

//     // let re = ret.iter_mut().map(|x| match x.obj {
//     //     Expr::InlineSrc(mut inl_src) => {
//     //         inl_src.lang = "rust";
//     //     }
//     //     _ => {}
//     // })

//     // let mut_br = ret.iter_mut();
//     // let mut ret: Vec<Node> = Vec::new();
//     let mut ret: NodePool = NodePool::new();
//     for item in ret.iter_mut() {
//         item.parent = Some(NodeID(1));
//     }

//     for item in ret.iter_mut() {
//         item.parent = Some(NodeID(1));
//     }
//     // drop(mut_br);
//     //

//     for item in ret.iter() {
//         match item.obj {
//             Expr::InlineSrc(mut inl) => {
//                 inl.lang = "rust";
//             }
//             _ => {}
//         }
//     }

//     // let a = &ret;
//     // ret.iter_mut().map(|x| match x.obj {
//     //     Expr::InlineSrc(mut inl_src) => {
//     //         inl_src.lang = "rust";
//     //     }
//     //     _ => {}
//     // });

//     // ret.i
//     // ret.push(value)

//     // ret.iter()
// }
