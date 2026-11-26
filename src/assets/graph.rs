use std::{any::TypeId, collections::HashMap};

use crate::assets::GraphError;

#[derive(Default)]
struct Graph {
    at: Vec<usize>,
    idxs: HashMap<TypeId, usize>,
    nodes: Vec<Node>,
    search: SearchState,
}

impl Graph {
    fn add(&mut self, u: TypeId, v: TypeId) -> Result<(), GraphError> {
        if u == v {
            return Err(GraphError::SelfReference);
        }

        let uidx = self.get(u);
        let uord = self.nodes[uidx].ord;

        let vidx = self.get(v);
        let vord = self.nodes[vidx].ord;

        if uord < vord {
            if !self.nodes[uidx].succ.contains(&vidx) {
                self.nodes[uidx].succ.push(vidx);
                self.nodes[vidx].pred.push(uidx);
            }

            return Ok(());
        }

        self.search.run(&self.nodes, uidx, uord, vidx, vord)?;
        self.reorder();

        self.nodes[uidx].succ.push(vidx);
        self.nodes[vidx].pred.push(uidx);

        Ok(())
    }

    fn get(&mut self, n: TypeId) -> usize {
        if let Some(idx) = self.idxs.get(&n) {
            *idx
        } else {
            self.insert(n)
        }
    }

    fn insert(&mut self, n: TypeId) -> usize {
        let idx = self.nodes.len();
        let pos = self.at.len();
        let node = Node::new(n, pos);

        self.nodes.push(node);
        self.at.push(idx);
        self.idxs.insert(n, idx);
        self.search.visited.push(false);

        idx
    }

    fn reorder(&mut self) {
        let iter = self
            .search
            .bwd
            .iter()
            .chain(self.search.fwd.iter())
            .zip(&self.search.ords);

        for (&idx, &ord) in iter {
            self.nodes[idx].ord = ord;
            self.at[ord] = idx;
        }
    }
}

struct Node {
    id: TypeId,
    ord: usize,
    pred: Vec<usize>,
    succ: Vec<usize>,
}

impl Node {
    fn new(id: TypeId, ord: usize) -> Self {
        Self {
            id,
            ord,
            pred: Vec::new(),
            succ: Vec::new(),
        }
    }
}

#[derive(Default)]
struct SearchState {
    bwd: Vec<usize>,
    fwd: Vec<usize>,
    ords: Vec<usize>,
    stack: Vec<usize>,
    visited: Vec<bool>,
}

impl SearchState {
    fn determine_order(&mut self, nodes: &Vec<Node>) {
        self.ords.clear();

        let mut bp: usize = 0;
        let mut fp: usize = 0;

        while bp < self.bwd.len() && fp < self.fwd.len() {
            let bx = self.bwd[bp];
            let fx = self.fwd[fp];

            if nodes[bx].ord < nodes[fx].ord {
                self.ords.push(nodes[bx].ord);
                bp += 1;
            } else {
                self.ords.push(nodes[fx].ord);
                fp += 1;
            }
        }

        for &idx in &self.bwd[bp..] {
            self.ords.push(nodes[idx].ord);
        }

        for &idx in &self.fwd[fp..] {
            self.ords.push(nodes[idx].ord);
        }
    }

    fn mark_bwd(&mut self, idx: usize) {
        self.bwd.push(idx);
        self.visited[idx] = true;
        self.stack.push(idx);
    }

    fn mark_fwd(&mut self, idx: usize) {
        self.fwd.push(idx);
        self.visited[idx] = true;
        self.stack.push(idx);
    }

    fn reset(&mut self) {
        self.stack.clear();

        for idx in self.bwd.drain(..) {
            self.visited[idx] = false;
        }

        for idx in self.fwd.drain(..) {
            self.visited[idx] = false;
        }
    }

    fn run(
        &mut self,
        nodes: &Vec<Node>,
        uidx: usize,
        uord: usize,
        vidx: usize,
        vord: usize,
    ) -> Result<(), GraphError> {
        self.reset();

        self.mark_fwd(vidx);
        while let Some(idx) = self.stack.pop() {
            for &next in &nodes[idx].succ {
                if next == uidx {
                    return Err(GraphError::Cycle);
                }

                if self.visited[next] || nodes[next].ord > uord {
                    continue;
                }

                self.mark_fwd(next);
            }
        }

        self.mark_bwd(uidx);
        while let Some(idx) = self.stack.pop() {
            for &next in &nodes[idx].pred {
                if self.visited[next] || nodes[next].ord < vord {
                    continue;
                }

                self.mark_bwd(next);
            }
        }

        self.bwd.sort_by_key(|&idx| nodes[idx].ord);
        self.fwd.sort_by_key(|&idx| nodes[idx].ord);

        self.determine_order(&nodes);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Distinct zero-sized marker types so each `N<K>` has its own `TypeId`.
    struct N<const K: usize>;

    macro_rules! ids {
        ($($k:literal)*) => { [$(TypeId::of::<N<$k>>()),*] };
    }

    const COUNT: usize = 16;

    fn ids() -> [TypeId; COUNT] {
        ids!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15)
    }

    fn ord(g: &Graph, id: TypeId) -> usize {
        g.nodes[g.idxs[&id]].ord
    }

    fn edge_count(g: &Graph) -> usize {
        g.nodes.iter().map(|n| n.succ.len()).sum()
    }

    /// Checks every structural invariant the graph is supposed to maintain.
    fn assert_consistent(g: &Graph) {
        let n = g.nodes.len();
        assert_eq!(g.at.len(), n, "`at` length mismatch");
        assert_eq!(g.idxs.len(), n, "`idxs` length mismatch");
        assert_eq!(g.search.visited.len(), n, "`visited` length mismatch");

        // `visited` must be clean between searches, or a later search can
        // wrongly skip nodes.
        assert!(
            g.search.visited.iter().all(|v| !v)
                || !g.search.bwd.is_empty()
                || !g.search.fwd.is_empty(),
            "stale visited flags with empty search lists"
        );

        // `ord` is a permutation of 0..n and `at` is its inverse.
        for (idx, node) in g.nodes.iter().enumerate() {
            assert!(node.ord < n, "ord out of range");
            assert_eq!(g.at[node.ord], idx, "`at` is not the inverse of `ord`");
            assert_eq!(
                g.idxs[&node.id], idx,
                "`idxs` points at the wrong node"
            );
        }

        // Topological invariant + pred/succ symmetry + no duplicate edges.
        for (u, node) in g.nodes.iter().enumerate() {
            for (i, &v) in node.succ.iter().enumerate() {
                assert!(
                    node.ord < g.nodes[v].ord,
                    "edge {u}->{v} violates order ({} >= {})",
                    node.ord,
                    g.nodes[v].ord
                );
                assert!(
                    !node.succ[..i].contains(&v),
                    "duplicate edge {u}->{v}"
                );
                assert_eq!(
                    g.nodes[v].pred.iter().filter(|&&p| p == u).count(),
                    1,
                    "pred list out of sync for {u}->{v}"
                );
            }

            for &p in &node.pred {
                assert!(g.nodes[p].succ.contains(&u), "dangling pred {p}->{u}");
            }
        }
    }

    #[test]
    fn self_reference_is_rejected() {
        let [a, ..] = ids();
        let mut g = Graph::default();

        assert!(matches!(g.add(a, a), Err(GraphError::SelfReference)));
        assert!(
            g.nodes.is_empty(),
            "rejected self edge should not insert nodes"
        );
    }

    #[test]
    fn forward_edge_needs_no_reorder() {
        let [a, b, ..] = ids();
        let mut g = Graph::default();

        g.add(a, b).unwrap();
        assert_consistent(&g);
        assert_eq!(ord(&g, a), 0);
        assert_eq!(ord(&g, b), 1);
    }

    #[test]
    fn backward_edge_reorders() {
        let [a, b, ..] = ids();
        let mut g = Graph::default();
        g.get(a);
        g.get(b);

        // b currently sorts after a, so b->a forces a reorder.
        g.add(b, a).unwrap();
        assert_consistent(&g);
        assert!(ord(&g, b) < ord(&g, a));
    }

    #[test]
    fn duplicate_edge_is_ignored() {
        let [a, b, ..] = ids();
        let mut g = Graph::default();

        g.add(a, b).unwrap();
        g.add(a, b).unwrap();
        assert_consistent(&g);
        assert_eq!(edge_count(&g), 1);
    }

    #[test]
    fn two_cycle_is_rejected() {
        let [a, b, ..] = ids();
        let mut g = Graph::default();

        g.add(a, b).unwrap();
        assert!(matches!(g.add(b, a), Err(GraphError::Cycle)));
        assert_consistent(&g);
        assert_eq!(edge_count(&g), 1);
    }

    #[test]
    fn long_cycle_is_rejected() {
        let [a, b, c, d, ..] = ids();
        let mut g = Graph::default();

        g.add(a, b).unwrap();
        g.add(b, c).unwrap();
        g.add(c, d).unwrap();
        assert!(matches!(g.add(d, a), Err(GraphError::Cycle)));
        assert_consistent(&g);
        assert_eq!(edge_count(&g), 3);
    }

    #[test]
    fn cycle_error_leaves_graph_usable() {
        let [a, b, c, ..] = ids();
        let mut g = Graph::default();

        g.add(a, b).unwrap();
        g.add(b, c).unwrap();
        assert!(g.add(c, a).is_err());

        // The failed search returns early with `visited` set; the next search
        // must still see every node.
        g.add(c, TypeId::of::<N<99>>()).unwrap();
        g.add(TypeId::of::<N<98>>(), a).unwrap();
        assert_consistent(&g);
    }

    #[test]
    fn reorder_moves_whole_affected_region() {
        // Insert in order a..e, then build e->d->c->b->a one edge at a time so
        // every insertion is a backward edge that drags a growing chain.
        let [a, b, c, d, e, ..] = ids();
        let mut g = Graph::default();
        for id in [a, b, c, d, e] {
            g.get(id);
        }

        g.add(b, a).unwrap();
        g.add(c, b).unwrap();
        g.add(d, c).unwrap();
        g.add(e, d).unwrap();
        assert_consistent(&g);

        let order: Vec<_> = g.at.iter().map(|&i| g.nodes[i].id).collect();
        assert_eq!(order, vec![e, d, c, b, a]);
    }

    #[test]
    fn unrelated_nodes_keep_their_order() {
        // Nodes outside the affected region [ord(v), ord(u)] must not move.
        let [a, b, c, d, e, ..] = ids();
        let mut g = Graph::default();
        for id in [a, b, c, d, e] {
            g.get(id);
        }

        g.add(d, b).unwrap();
        assert_consistent(&g);
        assert_eq!(ord(&g, a), 0);
        assert_eq!(ord(&g, e), 4);
    }

    #[test]
    fn diamond() {
        let [a, b, c, d, ..] = ids();
        let mut g = Graph::default();
        for id in [d, c, b, a] {
            g.get(id);
        }

        g.add(a, b).unwrap();
        g.add(a, c).unwrap();
        g.add(b, d).unwrap();
        g.add(c, d).unwrap();
        assert_consistent(&g);
        assert!(matches!(g.add(d, a), Err(GraphError::Cycle)));
    }

    /// Randomised check against a brute-force reachability oracle.
    #[test]
    fn matches_reference_on_random_edges() {
        let ids = ids();

        // xorshift64: deterministic, no dev-dependency.
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut rand = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % COUNT as u64) as usize
        };

        for _round in 0..20 {
            let mut g = Graph::default();
            let mut adj = [[false; COUNT]; COUNT];

            let reaches =
                |adj: &[[bool; COUNT]; COUNT], from: usize, to: usize| {
                    let mut seen = [false; COUNT];
                    let mut stack = vec![from];

                    while let Some(x) = stack.pop() {
                        if x == to {
                            return true;
                        }

                        if std::mem::replace(&mut seen[x], true) {
                            continue;
                        }

                        stack.extend((0..COUNT).filter(|&y| adj[x][y]));
                    }
                    false
                };

            for _ in 0..120 {
                let (u, v) = (rand(), rand());
                let result = g.add(ids[u], ids[v]);

                if u == v {
                    assert!(matches!(result, Err(GraphError::SelfReference)));
                } else if reaches(&adj, v, u) {
                    assert!(
                        matches!(result, Err(GraphError::Cycle)),
                        "{u}->{v} should cycle"
                    );
                } else {
                    assert!(result.is_ok(), "{u}->{v} rejected: {result:?}");
                    adj[u][v] = true;
                }

                assert_consistent(&g);
            }

            let expected = adj.iter().flatten().filter(|&&e| e).count();
            assert_eq!(edge_count(&g), expected);
        }
    }
}
