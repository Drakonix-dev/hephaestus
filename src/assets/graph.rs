use std::{any::TypeId, collections::HashMap};

use crate::assets::{BuiltAs, DependencyKind, Handle};

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct Node {
    generation: u64,
    id: usize,
    t: TypeId,
}

impl<B: BuiltAs> From<Handle<B>> for Node {
    fn from(handle: Handle<B>) -> Node {
        Node {
            generation: handle.generation,
            id: handle.id,
            t: TypeId::of::<B>(),
        }
    }
}

pub(crate) struct Edge {
    kind: DependencyKind,
    target: Node,
}

impl Edge {
    fn new(target: Node, kind: DependencyKind) -> Self {
        Self { kind, target }
    }
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
