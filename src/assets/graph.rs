use std::{any::TypeId, collections::HashMap};

use crate::assets::{Asset, Handle};

#[derive(PartialEq, Eq, Hash)]
pub struct Node {
    generation: u64,
    id: usize,
    t: TypeId,
}

impl<A: Asset> From<Handle<A>> for Node {
    fn from(handle: Handle<A>) -> Node {
        Node {
            generation: handle.generation,
            id: handle.id,
            t: TypeId::of::<A>(),
        }
    }
}

pub(crate) struct Edge {
    kind: EdgeKind,
    target: Node,
}

impl Edge {
    fn new(target: Node, kind: EdgeKind) -> Self {
        Self { kind, target }
    }
}

pub(crate) enum EdgeKind {
    Optional,
    Required,
}

pub(crate) struct Graph {
    nodes: HashMap<Node, Vec<Edge>>,
}

impl Graph {
    pub(crate) fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub(crate) fn add_edge(&mut self, node: Node, edge: Edge) {
        self.nodes
            .entry(node)
            .or_insert_with(|| Vec::new())
            .push(edge)
    }
}
