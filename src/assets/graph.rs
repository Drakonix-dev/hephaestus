use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    sync::Arc,
};

use crate::assets::{
    AssetError, AssetStatus, BuiltAs, Dependency, DependencyKind, Handle, INVARIANT, Resolved,
    SourceFor, registry::ErasedRegistry,
};

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
    status: AssetStatus,
    target: Node,
}

impl Edge {
    fn new(target: Node, kind: DependencyKind) -> Self {
        Self {
            kind,
            status: AssetStatus::Pending,
            target,
        }
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

trait ErasedDependency: Any + Send {}
trait ErasedHandle: Any + Send {}

struct DependencyCell<B: BuiltAs, S: SourceFor<B>> {
    kind: DependencyKind,
    src: S,
    _marker: PhantomData<fn() -> B>,
}

impl<B: BuiltAs, S: SourceFor<B>> DependencyCell<B, S> {
    fn new(src: S, kind: DependencyKind) -> Self {
        Self {
            kind,
            src,
            _marker: PhantomData,
        }
    }
}

impl<B: BuiltAs, S: SourceFor<B>> ErasedDependency for DependencyCell<B, S> {}

struct HandleCell<B: BuiltAs> {
    handle: Handle<B>,
    kind: DependencyKind,
}

impl<B: BuiltAs> HandleCell<B> {
    fn new(handle: Handle<B>, kind: DependencyKind) -> Self {
        Self { handle, kind }
    }
}

impl<B: BuiltAs> ErasedHandle for HandleCell<B> {}

pub struct Deps {
    deps: Vec<Box<dyn ErasedDependency>>,
    handles: Vec<Box<dyn ErasedHandle>>,
}

impl Deps {
    pub(crate) fn new() -> Self {
        Self {
            deps: Vec::new(),
            handles: Vec::new(),
        }
    }

    pub(crate) fn add_to_graph(&self, graph: &mut Graph) {
        todo!("add to graph")
    }

    pub fn require<B: BuiltAs, S: SourceFor<B>>(
        &mut self,
        src: S,
        kind: DependencyKind,
    ) -> Dependency<B> {
        self.deps.push(Box::new(DependencyCell::new(src, kind)));
        Dependency::new(self.deps.len())
    }

    pub fn require_handle<B: BuiltAs>(&mut self, handle: Handle<B>, kind: DependencyKind) {
        self.handles.push(Box::new(HandleCell::new(handle, kind)));
    }
}

pub struct Fetch {
    built: Vec<(Box<dyn Any>, Option<Arc<dyn Any>>)>,
    handles: HashMap<(TypeId, usize, u64), Option<Arc<dyn Any>>>,
}

impl Fetch {
    pub(crate) fn new() -> Self {
        Self {
            built: Vec::new(),
            handles: HashMap::new(),
        }
    }

    pub(crate) fn get_dependencies(&mut self, deps: &Deps, reg: &mut dyn ErasedRegistry) {
        todo!("get dependencies")
    }

    pub fn get<B: BuiltAs>(
        &self,
        dependency: Dependency<B>,
    ) -> Result<(Handle<B>, Resolved<B::Built>), AssetError> {
        let (handle, built) = self
            .built
            .get(dependency.idx)
            .ok_or(dependency.not_found())?;

        let built = if let Some(b) = built {
            Some(b.downcast_ref::<B::Built>().expect(INVARIANT))
        } else {
            None
        };
        let handle = *handle.downcast_ref::<Handle<B>>().expect(INVARIANT);

        Ok((handle, Resolved::new(handle.id, built)))
    }

    pub fn get_handle<B: BuiltAs>(
        &self,
        handle: Handle<B>,
    ) -> Result<Resolved<B::Built>, AssetError> {
        let built = self
            .handles
            .get(&(TypeId::of::<B>(), handle.id, handle.generation))
            .ok_or(handle.not_found())?;

        let built = if let Some(b) = built {
            Some(b.downcast_ref::<B::Built>().expect(INVARIANT))
        } else {
            None
        };

        Ok(Resolved::new(handle.id, built))
    }
}
