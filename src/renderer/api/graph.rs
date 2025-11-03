use std::collections::{HashMap, HashSet};

// RenderPhase defines a phase of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderPhase {
    pub domain: RenderDomain,
    pub subphase: Option<SubPhase>,
}

impl RenderPhase {
    pub const fn new(domain: RenderDomain, subphase: Option<SubPhase>) -> Self {
        Self { domain, subphase }
    }

    fn sort_key(&self) -> (u32, u32) {
        let dkey = match self.domain {
            RenderDomain::UI => 0,
            RenderDomain::World2D => 1,
            RenderDomain::World3D => 2,
            RenderDomain::Other(x) => 3 + x,
        };

        let skey = match self.subphase {
            None => 0,
            Some(SubPhase::Opaque) => 1,
            Some(SubPhase::Transparent) => 2,
            Some(SubPhase::Other(x)) => 3 + x,
        };

        (dkey, skey)
    }
}

impl Ord for RenderPhase {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for RenderPhase {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// RenderDomain defines the different domains of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderDomain {
    UI,
    World2D,
    World3D,
    Other(u32),
}

// SubPhase defines the different sub-phases of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubPhase {
    Opaque,
    Transparent,
    Other(u32),
}

pub struct RenderGraph {
    nodes: HashSet<RenderPhase>,
    edges: HashMap<RenderPhase, Vec<RenderPhase>>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
        }
    }

    // --- manipulate graph ---

    pub fn add_after(mut self, phase: RenderPhase, after: RenderPhase) -> Self {
        self.nodes.insert(phase);
        self.edges.entry(phase).or_default();
        self.add_dependency(after, phase)
    }
    
    pub fn add_before(mut self, phase: RenderPhase, before: RenderPhase) -> Self {
        self.nodes.insert(phase);
        self.edges.entry(phase).or_default();
        self.add_dependency(phase, before)
    }

    pub fn add_dependency(mut self, before: RenderPhase, after: RenderPhase) -> Self {
        assert!(self.nodes.contains(&before) && self.nodes.contains(&after), "Phases must exist");
        self.edges.entry(before).or_default().push(after);
        self
    }

    pub fn add_phase(mut self, phase: RenderPhase) -> Self {
        self.nodes.insert(phase);
        self.edges.entry(phase).or_default();
        self
    }

    pub fn remove_phase(mut self, phase: RenderPhase) -> Self {
        self.nodes.remove(&phase);
        self.edges.remove(&phase);
        
        for deps in self.edges.values_mut() {
            deps.retain(|p| p != &phase);
        }

        self
    }

    // --- generate graph ---
    
    pub fn linearize(&self) -> Result<Vec<RenderPhase>, &'static str> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn visit(
            node: &RenderPhase,
            graph: &RenderGraph,
            visited: &mut HashSet<RenderPhase>,
            visiting: &mut HashSet<RenderPhase>,
            sorted: &mut Vec<RenderPhase>,
        ) -> Result<(), &'static str> {
            if visited.contains(node) { return Ok(()); }
            if !visiting.insert(*node) { return Err("Cycle detected in render graph"); }

            if let Some(deps) = graph.edges.get(node) {
                let mut deps_sorted = deps.clone();
                deps_sorted.sort();
                
                for dep in deps_sorted {
                    visit(&dep, graph, visited, visiting, sorted)?;
                }
            }

            visiting.remove(node);
            visited.insert(*node);
            sorted.push(*node);
            Ok(())
        }

        let mut nodes_sorted: Vec<_> = self.nodes.iter().collect();
        nodes_sorted.sort();

        for node in nodes_sorted {
            visit(node, self, &mut visited, &mut visiting, &mut sorted)?;
        }

        Ok(sorted)
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        RenderGraph::new()
            .add_phase(RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Opaque)))
            .add_phase(RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Transparent)))
            .add_phase(RenderPhase::new(RenderDomain::World2D, Some(SubPhase::Opaque)))
            .add_phase(RenderPhase::new(RenderDomain::World2D, Some(SubPhase::Transparent)))
            .add_phase(RenderPhase::new(RenderDomain::UI, None))
    }
}
