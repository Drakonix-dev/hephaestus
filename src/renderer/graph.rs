use std::collections::{HashMap, HashSet};

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
            RenderDomain::World3D => 0,
            RenderDomain::World2D => 1,
            RenderDomain::UI => 2,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderDomain {
    UI,
    World2D,
    World3D,
    Other(u32),
}

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
        assert!(
            self.nodes.contains(&before) && self.nodes.contains(&after),
            "Phases must exist"
        );
        self.edges.entry(after).or_default().push(before);
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
            if visited.contains(node) {
                return Ok(());
            }
            if !visiting.insert(*node) {
                return Err("Cycle detected in render graph");
            }

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
            .add_phase(RenderPhase::new(
                RenderDomain::World3D,
                Some(SubPhase::Opaque),
            ))
            .add_phase(RenderPhase::new(
                RenderDomain::World3D,
                Some(SubPhase::Transparent),
            ))
            .add_phase(RenderPhase::new(
                RenderDomain::World2D,
                Some(SubPhase::Opaque),
            ))
            .add_phase(RenderPhase::new(
                RenderDomain::World2D,
                Some(SubPhase::Transparent),
            ))
            .add_phase(RenderPhase::new(RenderDomain::UI, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORLD3D_OPAQUE: RenderPhase =
        RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Opaque));
    const WORLD3D_TRANSPARENT: RenderPhase =
        RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Transparent));
    const WORLD2D_OPAQUE: RenderPhase =
        RenderPhase::new(RenderDomain::World2D, Some(SubPhase::Opaque));
    const WORLD2D_TRANSPARENT: RenderPhase =
        RenderPhase::new(RenderDomain::World2D, Some(SubPhase::Transparent));
    const UI: RenderPhase = RenderPhase::new(RenderDomain::UI, None);

    #[test]
    fn default_graph_draws_world_before_ui_and_opaque_before_transparent() {
        let order = RenderGraph::default().linearize().unwrap();

        assert_eq!(
            order,
            vec![
                WORLD3D_OPAQUE,
                WORLD3D_TRANSPARENT,
                WORLD2D_OPAQUE,
                WORLD2D_TRANSPARENT,
                UI,
            ]
        );
    }

    #[test]
    fn add_after_places_phase_after_reference() {
        let order = RenderGraph::new()
            .add_phase(WORLD3D_OPAQUE)
            .add_after(UI, WORLD3D_OPAQUE)
            .linearize()
            .unwrap();

        let ui_idx = order.iter().position(|p| p == &UI).unwrap();
        let world_idx = order.iter().position(|p| p == &WORLD3D_OPAQUE).unwrap();
        assert!(
            world_idx < ui_idx,
            "expected {WORLD3D_OPAQUE:?} before {UI:?}, got {order:?}"
        );
    }

    #[test]
    fn add_before_places_phase_before_reference() {
        let order = RenderGraph::new()
            .add_phase(UI)
            .add_before(WORLD3D_OPAQUE, UI)
            .linearize()
            .unwrap();

        let ui_idx = order.iter().position(|p| p == &UI).unwrap();
        let world_idx = order.iter().position(|p| p == &WORLD3D_OPAQUE).unwrap();
        assert!(
            world_idx < ui_idx,
            "expected {WORLD3D_OPAQUE:?} before {UI:?}, got {order:?}"
        );
    }

    #[test]
    fn add_dependency_orders_before_ahead_of_after() {
        let order = RenderGraph::new()
            .add_phase(WORLD3D_OPAQUE)
            .add_phase(UI)
            .add_dependency(WORLD3D_OPAQUE, UI)
            .linearize()
            .unwrap();

        assert_eq!(order, vec![WORLD3D_OPAQUE, UI]);
    }

    #[test]
    fn linearize_detects_cycles() {
        let result = RenderGraph::new()
            .add_phase(WORLD3D_OPAQUE)
            .add_phase(UI)
            .add_dependency(WORLD3D_OPAQUE, UI)
            .add_dependency(UI, WORLD3D_OPAQUE)
            .linearize();

        assert!(result.is_err());
    }

    #[test]
    fn remove_phase_drops_it_and_its_dependency_edges() {
        let order = RenderGraph::new()
            .add_phase(WORLD3D_OPAQUE)
            .add_phase(UI)
            .add_dependency(WORLD3D_OPAQUE, UI)
            .remove_phase(WORLD3D_OPAQUE)
            .linearize()
            .unwrap();

        assert_eq!(order, vec![UI]);
    }
}
