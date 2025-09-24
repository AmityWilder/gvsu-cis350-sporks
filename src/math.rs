//! Module for discrete mathematical structures and algorithms.

use std::collections::{HashSet, VecDeque};

/// Types that can be used as an ID.
pub trait Id: Copy + Eq + std::hash::Hash {}

impl<T: Copy + Eq + std::hash::Hash> Id for T {}

/// Breadth-first search iterator.
#[derive(Debug, Clone)]
pub struct BfsIter<'a, T> {
    graph: &'a Graph<T>,
    visited: HashSet<T>,
    queue: VecDeque<T>,
}

impl<'a, T: Id> BfsIter<'a, T> {
    fn new<I>(graph: &'a Graph<T>, roots: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let queue = VecDeque::from_iter(roots);
        Self {
            graph,
            visited: HashSet::from_iter(queue.iter().copied()),
            queue,
        }
    }
}

impl<'a, T: Id> Iterator for BfsIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(v) = self.queue.pop_front() {
            let Some(adj) = self.graph.adjacent(&v) else {
                if cfg!(any(test, debug_assertions)) {
                    panic!("all queued should be in the graph");
                } else {
                    return None;
                }
            };
            self.queue
                .extend(adj.iter().filter(|&&a| self.visited.insert(a)));
            return Some(v);
        }
        None
    }
}

impl<T> std::iter::FusedIterator for BfsIter<'_, T> where Self: Iterator {}

/// Depth-first search iterator.
#[derive(Debug, Clone)]
pub struct DfsIter<'a, T> {
    graph: &'a Graph<T>,
    visited: HashSet<T>,
    stack: Vec<T>,
}

impl<'a, T: Id> DfsIter<'a, T> {
    fn new(graph: &'a Graph<T>, root: T) -> Self {
        let stack = vec![root];
        Self {
            graph,
            visited: HashSet::new(),
            stack,
        }
    }
}

impl<'a, T: Id> Iterator for DfsIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(v) = self.stack.pop() {
            if self.visited.insert(v) {
                let Some(adj) = self.graph.adjacent(&v) else {
                    if cfg!(any(test, debug_assertions)) {
                        panic!("all queued should be in the graph");
                    } else {
                        return None;
                    }
                };
                self.stack.extend(adj.iter().rev().copied());
            }
            return Some(v);
        }
        None
    }
}

impl<T> std::iter::FusedIterator for DfsIter<'_, T> where Self: Iterator {}

// pub struct DinicIter<'a, T: 'a> {}

/// Directed graph.
#[derive(Debug)]
pub struct Graph<V> {
    verts: Vec<V>,
    adj: Vec<V>,
    vert_adjs: Vec<(bool, usize)>,
}

impl<V: Id> Graph<V> {
    /// Construct a graph from an iterator over vertices and an iterator over edges.
    /// `(a, b) => a -> b`
    pub fn from_verts_and_edges<I, J, K>(verts: I, edges: J) -> Option<Self>
    where
        I: IntoIterator<Item = V>,
        J: IntoIterator<Item = (V, V), IntoIter = K>,
        K: Iterator<Item = (V, V)> + Clone,
    {
        let verts = Vec::from_iter(verts);
        let edges: K = edges.into_iter();
        let mut adj = Vec::with_capacity(edges.size_hint().0);
        let mut vert_adjs = vec![(false, 0); verts.len()];
        for (a, b) in edges.clone() {
            let a_pos = verts.iter().position(|x| x == &a)?;
            let b_pos = verts.iter().position(|x| x == &b)?;
            adj.insert(
                vert_adjs[..=a_pos].iter().map(|&(_, n)| n).sum::<usize>(),
                b,
            );
            vert_adjs[a_pos].1 += 1;
            vert_adjs[b_pos].0 = true;
        }
        Some(Self {
            verts,
            adj,
            vert_adjs,
        })
    }

    /// Get the slice of all vertices in the graph.
    pub const fn verts(&self) -> &[V] {
        self.verts.as_slice()
    }

    /// Whether `vert` has any inputs or not.
    ///
    /// Returns [`None`] if `vert` is not in the graph.
    pub fn has_inputs(&self, vert: &V) -> Option<bool> {
        let pos = self.verts.iter().position(|x| x == vert)?;
        Some(self.vert_adjs[pos].0)
    }

    /// Number of outputs `vert` has.
    ///
    /// Returns [`None`] if `vert` is not in the graph.
    pub fn adjacent_len(&self, vert: &V) -> Option<usize> {
        let pos = self.verts.iter().position(|x| x == vert)?;
        Some(self.vert_adjs[pos].1)
    }

    /// Get a slice of all vertices adjacent to (following) `vert`.
    ///
    /// Returns [`None`] if `vert` is not in the graph.
    pub fn adjacent(&self, vert: &V) -> Option<&[V]> {
        let pos = self.verts.iter().position(|x| x == vert)?;
        let start = self.vert_adjs[..pos].iter().map(|&(_, n)| n).sum::<usize>();
        self.adj.get(start..start + self.vert_adjs[pos].1)
    }

    /// Construct a breadth-first search iterator over the graph.
    pub fn bfs<I>(&self, roots: I) -> BfsIter<'_, V>
    where
        I: IntoIterator<Item = V>,
    {
        BfsIter::new(self, roots)
    }

    /// Construct a depth-first search iterator over the graph.
    pub fn dfs(&self, root: V) -> DfsIter<'_, V> {
        DfsIter::new(self, root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construction() {
        // 0 -- 1 -- 2 -- 5 -- 7
        //       \         \
        //        3 -- 4    6
        let verts = [0, 1, 2, 3, 4, 5, 6, 7];
        let edges = [(0, 1), (1, 2), (1, 3), (5, 6), (2, 5), (5, 7), (3, 4)];
        let graph = Graph::from_verts_and_edges(verts, edges).unwrap();
        assert_eq!(graph.verts.as_slice(), &verts);
        assert_eq!(
            graph.adj.as_slice(),
            &[
                1, // 0 -> 1
                2, // 1 -> 2
                3, // 1 -> 3
                5, // 2 -> 5
                4, // 3 -> 4
                6, // 5 -> 6
                7, // 5 -> 7
            ]
        );
        assert_eq!(
            graph.vert_adjs.as_slice(),
            &[
                (false, 1), // _ -> 0 -> 1
                (true, 2),  // 0 -> 1 -> 2, 3
                (true, 1),  // 1 -> 2 -> 5
                (true, 1),  // 1 -> 3 -> 4
                (true, 0),  // 3 -> 4 -> _
                (true, 2),  // 2 -> 5 -> 6, 7
                (true, 0),  // 5 -> 6 -> _
                (true, 0),  // 5 -> 7 -> _
            ]
        );
    }

    #[test]
    fn test_bfs() {
        // 0 -- 1 -- 2 -- 5 -- 7
        //       \         \
        //        3 -- 4    6
        let verts = [0, 1, 2, 3, 4, 5, 6, 7];
        let edges = [(0, 1), (1, 2), (1, 3), (5, 6), (2, 5), (5, 7), (3, 4)];
        let graph = Graph::from_verts_and_edges(verts, edges).unwrap();
        let ord = graph.bfs([0]).collect::<Vec<_>>();
        assert_eq!(ord.as_slice(), &[0, 1, 2, 3, 5, 4, 6, 7]);
    }

    #[test]
    fn test_dfs() {
        // 0 -- 1 -- 2 -- 5 -- 7
        //       \         \
        //        3 -- 4    6
        let verts = [0, 1, 2, 3, 4, 5, 6, 7];
        let edges = [(0, 1), (1, 2), (1, 3), (5, 6), (2, 5), (5, 7), (3, 4)];
        let graph = Graph::from_verts_and_edges(verts, edges).unwrap();
        let ord = graph.dfs(0).collect::<Vec<_>>();
        assert_eq!(ord.as_slice(), &[0, 1, 2, 5, 6, 7, 3, 4]);
    }
}
