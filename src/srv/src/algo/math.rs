//! Module for discrete mathematical structures and algorithms.

use std::collections::{HashMap, HashSet, VecDeque, hash_map};

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
    verts: HashMap<V, std::ops::Range<usize>>,
    adj: Vec<V>,
}

impl<V: Id> Graph<V> {
    /// Construct a graph from an iterator over vertices and an iterator over connections.
    ///
    /// Expects connections to be the "next" vertex in the edge.
    pub fn from_forward<I, J>(iter: I) -> Option<Self>
    where
        I: IntoIterator<Item = (V, J)>,
        J: IntoIterator<Item = V>,
    {
        let it = iter.into_iter();
        let mut adj = Vec::with_capacity(it.size_hint().1.unwrap_or(it.size_hint().0));
        let verts = it
            .map(|(v, e)| {
                let start = adj.len();
                adj.extend(e);
                let end = adj.len();
                (v, start..end)
            })
            .collect();
        Some(Self { verts, adj })
    }

    /// Get iterator over the vertices in the graph in an arbitrary order.
    pub fn verts(&self) -> hash_map::Keys<'_, V, std::ops::Range<usize>> {
        self.verts.keys()
    }

    /// Get a slice of all vertices adjacent to (following) `vert`.
    ///
    /// Returns [`None`] if `vert` is not in the graph.
    pub fn adjacent(&self, vert: &V) -> Option<&[V]> {
        self.verts.get(vert).cloned().and_then(|x| self.adj.get(x))
    }

    /// Returns an arbitrarily ordered iterator, possibly containing duplicates, of all vertices in the graph that have inputs.
    pub fn receivers(&self) -> &[V] {
        self.adj.as_slice()
    }

    /// Construct a breadth-first search iterator over the graph.
    pub fn bfs<I: IntoIterator<Item = V>>(&self, roots: I) -> BfsIter<'_, V> {
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
        let verts = [
            (0, [1].iter().copied()),
            (1, [2, 3].iter().copied()),
            (2, [5].iter().copied()),
            (3, [4].iter().copied()),
            (4, [].iter().copied()),
            (5, [6, 7].iter().copied()),
            (6, [].iter().copied()),
            (7, [].iter().copied()),
        ];
        let graph = Graph::from_forward(verts).unwrap();
        assert_eq!(
            graph.verts().copied().collect::<HashSet<_>>(),
            HashSet::from([0, 1, 2, 3, 4, 5, 6, 7])
        );
        assert_eq!(graph.adjacent(&0), Some([1].as_slice()));
        assert_eq!(graph.adjacent(&1), Some([2, 3].as_slice()));
        assert_eq!(graph.adjacent(&2), Some([5].as_slice()));
        assert_eq!(graph.adjacent(&3), Some([4].as_slice()));
        assert_eq!(graph.adjacent(&4), Some([].as_slice()));
        assert_eq!(graph.adjacent(&5), Some([6, 7].as_slice()));
        assert_eq!(graph.adjacent(&6), Some([].as_slice()));
        assert_eq!(graph.adjacent(&7), Some([].as_slice()));
    }

    #[test]
    fn test_bfs() {
        // 0 -- 1 -- 2 -- 5 -- 7
        //       \         \
        //        3 -- 4    6
        let verts = [
            (0, [1].iter().copied()),
            (1, [2, 3].iter().copied()),
            (2, [5].iter().copied()),
            (3, [4].iter().copied()),
            (4, [].iter().copied()),
            (5, [6, 7].iter().copied()),
            (6, [].iter().copied()),
            (7, [].iter().copied()),
        ];
        let graph = Graph::from_forward(verts).unwrap();
        let ord = graph.bfs([0]).collect::<Vec<_>>();
        assert_eq!(ord.as_slice(), &[0, 1, 2, 3, 5, 4, 6, 7]);
    }

    #[test]
    fn test_dfs() {
        // 0 -- 1 -- 2 -- 5 -- 7
        //       \         \
        //        3 -- 4    6
        let verts = [
            (0, [1].iter().copied()),
            (1, [2, 3].iter().copied()),
            (2, [5].iter().copied()),
            (3, [4].iter().copied()),
            (4, [].iter().copied()),
            (5, [6, 7].iter().copied()),
            (6, [].iter().copied()),
            (7, [].iter().copied()),
        ];
        let graph = Graph::from_forward(verts).unwrap();
        let ord = graph.dfs(0).collect::<Vec<_>>();
        assert_eq!(ord.as_slice(), &[0, 1, 2, 5, 6, 7, 3, 4]);
    }
}
