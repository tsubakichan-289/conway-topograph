use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<[f32; 2]>,
    pub edges: Vec<[usize; 2]>,
    #[serde(default)]
    pub double_edges: BTreeSet<usize>,
}

impl Graph {
    pub fn add_node(&mut self, position: [f32; 2]) -> usize {
        let index = self.nodes.len();
        self.nodes.push(position);
        index
    }

    pub fn degree(&self, node: usize) -> usize {
        self.edges
            .iter()
            .filter(|edge| edge.contains(&node))
            .count()
    }

    pub fn remove_node(&mut self, node: usize) -> bool {
        if node >= self.nodes.len() {
            return false;
        }
        self.nodes.remove(node);
        let old_edges = std::mem::take(&mut self.edges);
        let old_double_edges = std::mem::take(&mut self.double_edges);
        for (index, [a, b]) in old_edges.into_iter().enumerate() {
            if a == node || b == node {
                continue;
            }
            if old_double_edges.contains(&index) {
                self.double_edges.insert(self.edges.len());
            }
            self.edges
                .push([a - usize::from(a > node), b - usize::from(b > node)]);
        }
        true
    }

    pub fn connect(&mut self, a: usize, b: usize) -> Result<(), &'static str> {
        if a >= self.nodes.len() || b >= self.nodes.len() {
            return Err("Node does not exist.");
        }
        if a == b {
            return Err("Choose a different node.");
        }
        if self.edges.iter().any(|e| *e == [a, b] || *e == [b, a]) {
            return Err("These nodes are already connected.");
        }
        if self.degree(a) >= 3 || self.degree(b) >= 3 {
            return Err("A node can have at most 3 edges.");
        }
        self.edges.push([a, b]);
        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        let mut checked = Self {
            nodes: self.nodes.clone(),
            edges: Vec::new(),
            ..Default::default()
        };
        self.nodes.iter().flatten().all(|v| v.is_finite())
            && self
                .double_edges
                .iter()
                .all(|&edge| edge < self.edges.len())
            && self
                .edges
                .iter()
                .all(|&[a, b]| checked.connect(a, b).is_ok())
    }

    /// Edges forming an angle of at most 90 degrees at a shared node.
    pub fn sharp_angle_edges(&self) -> BTreeSet<usize> {
        let mut adjacent = vec![Vec::new(); self.nodes.len()];
        for (edge, &[a, b]) in self.edges.iter().enumerate() {
            adjacent[a].push((edge, b));
            adjacent[b].push((edge, a));
        }
        let mut marked = BTreeSet::new();
        for (node, incident) in adjacent.iter().enumerate() {
            let direction = |other: usize| {
                [
                    self.nodes[other][0] as f64 - self.nodes[node][0] as f64,
                    self.nodes[other][1] as f64 - self.nodes[node][1] as f64,
                ]
            };
            for (i, &(a, neighbor_a)) in incident.iter().enumerate() {
                let u = direction(neighbor_a);
                for &(b, neighbor_b) in &incident[i + 1..] {
                    let v = direction(neighbor_b);
                    // Coincident endpoints have no defined angle.
                    if u != [0.0, 0.0] && v != [0.0, 0.0] && u[0] * v[0] + u[1] * v[1] >= 0.0 {
                        marked.extend([a, b]);
                    }
                }
            }
        }
        marked
    }

    /// Shortest path by edge count, with insertion order breaking ties.
    pub fn path_edges(&self, start: usize, end: usize) -> Option<Vec<usize>> {
        if start >= self.nodes.len() || end >= self.nodes.len() {
            return None;
        }
        let mut adjacent = vec![Vec::new(); self.nodes.len()];
        for (index, &[a, b]) in self.edges.iter().enumerate() {
            adjacent[a].push((b, index));
            adjacent[b].push((a, index));
        }
        let mut visited = vec![false; self.nodes.len()];
        let mut previous = vec![None; self.nodes.len()];
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        while let Some(node) = queue.pop_front() {
            if node == end {
                let mut path = Vec::new();
                let mut current = end;
                while current != start {
                    let (parent, edge) = previous[current]?;
                    path.push(edge);
                    current = parent;
                }
                path.reverse();
                return Some(path);
            }
            for &(next, edge) in &adjacent[node] {
                if !visited[next] {
                    visited[next] = true;
                    previous[next] = Some((node, edge));
                    queue.push_back(next);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_highlighting_tracks_right_acute_and_obtuse_angles() {
        let mut g = Graph::default();
        for p in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [-1.0, -1.0]] {
            g.add_node(p);
        }
        // Mix edge orientations: the shared node need not be the first endpoint.
        for [a, b] in [[1, 0], [0, 2], [3, 0]] {
            g.connect(a, b).unwrap();
        }
        assert_eq!(g.sharp_angle_edges(), BTreeSet::from([0, 1]));
        g.nodes[2] = [0.1, 1.0];
        assert_eq!(g.sharp_angle_edges(), BTreeSet::from([0, 1]));
        g.nodes[2] = [-0.1, 1.0];
        assert!(g.sharp_angle_edges().is_empty());
        g.nodes[2] = [0.0, 0.0];
        assert!(g.sharp_angle_edges().is_empty());
        g.nodes[2] = [2.0, 0.0];
        assert_eq!(g.sharp_angle_edges(), BTreeSet::from([0, 1]));
    }

    #[test]
    fn paths_handle_branches_cycles_and_disconnected_nodes() {
        let mut g = graph();
        for [a, b] in [[0, 1], [1, 2], [2, 3], [1, 4]] {
            g.connect(a, b).unwrap();
        }
        assert_eq!(g.path_edges(0, 3), Some(vec![0, 1, 2]));
        assert_eq!(g.path_edges(4, 0), Some(vec![3, 0]));
        assert_eq!(g.path_edges(0, 5), None);
        assert_eq!(g.path_edges(0, 0), Some(vec![]));
        assert_eq!(g.path_edges(0, 6), None);
        g.connect(0, 3).unwrap();
        assert_eq!(g.path_edges(0, 3), Some(vec![4]));
    }

    fn graph() -> Graph {
        let mut graph = Graph::default();
        for i in 0..6 {
            graph.add_node([i as f32, 0.0]);
        }
        graph
    }

    #[test]
    fn fourth_edge_is_rejected_at_either_endpoint() {
        let mut g = graph();
        for node in 1..=3 {
            assert!(g.connect(0, node).is_ok());
        }
        assert!(g.connect(0, 4).is_err());
        assert!(g.connect(5, 0).is_err());
        assert_eq!(g.edges.len(), 3);
        assert_eq!(g.degree(0), 3);
    }

    #[test]
    fn invalid_connections_do_not_change_graph() {
        let mut g = graph();
        g.connect(0, 1).unwrap();
        for (a, b) in [(0, 0), (0, 1), (1, 0), (0, 6), (6, 1)] {
            assert!(g.connect(a, b).is_err());
        }
        assert_eq!(g.edges, vec![[0, 1]]);
    }

    #[test]
    fn validates_restored_graph() {
        let mut g = graph();
        assert!(g.is_valid());
        g.edges.push([0, 9]);
        assert!(!g.is_valid());
        g.edges = vec![[0, 1], [1, 0]];
        assert!(!g.is_valid());
        g.edges.clear();
        g.nodes[0][0] = f32::NAN;
        assert!(!g.is_valid());
    }

    #[test]
    fn deletion_preserves_remaining_connections_and_double_lines() {
        let mut g = graph();
        for [a, b] in [[0, 1], [1, 2], [2, 3], [0, 4], [4, 5]] {
            g.connect(a, b).unwrap();
        }
        g.double_edges.extend([0, 2, 4]);
        assert!(g.remove_node(1));
        assert_eq!(g.nodes.len(), 5);
        assert_eq!(g.edges, vec![[1, 2], [0, 3], [3, 4]]);
        assert_eq!(g.double_edges, BTreeSet::from([0, 2]));
        assert!(g.is_valid());
        assert!(!g.remove_node(5));
        while !g.nodes.is_empty() {
            assert!(g.remove_node(0));
            assert!(g.is_valid());
        }
        assert!(g.edges.is_empty());
        assert!(g.double_edges.is_empty());
    }
}
