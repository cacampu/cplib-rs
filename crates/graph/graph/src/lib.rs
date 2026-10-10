//! グラフアルゴリズムを頂点番号 `0..n` に対して統一的に書くための `Graph` トレイト。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::ops::Add;

use algebra::Zero;

/// `0..len()` の頂点を持つグラフ。重みなしグラフは `Weight = ()` にする。
pub trait Graph {
    type Weight: Copy;
    fn len(&self) -> usize;
    fn edges(&self, v: usize) -> impl Iterator<Item = (usize, Self::Weight)> + '_;

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 隣接リストによる `Graph` の実装。
///
/// 有向・無向、重みの有無は型ではなく辺の追加方法で決める。
///
/// ```
/// use graph::{AdjList, Graph};
/// let mut g = AdjList::<i64>::new(3);
/// g.add_undirected_edge(0, 1, 5);
/// g.add_edge(1, 2, 3); // 有向辺
/// assert_eq!(g.edges(1).collect::<Vec<_>>(), vec![(0, 5), (2, 3)]);
/// ```
#[derive(Clone, Debug)]
pub struct AdjList<W = ()> {
    adj: Vec<Vec<(usize, W)>>,
}

/// 重みなしグラフの別名。
pub type UnweightedGraph = AdjList<()>;

impl<W: Copy> AdjList<W> {
    pub fn new(n: usize) -> Self {
        Self {
            adj: vec![Vec::new(); n],
        }
    }

    pub fn add_edge(&mut self, u: usize, v: usize, w: W) {
        self.adj[u].push((v, w));
    }

    pub fn add_undirected_edge(&mut self, u: usize, v: usize, w: W) {
        self.add_edge(u, v, w);
        self.add_edge(v, u, w);
    }
}

impl<W: Copy> Graph for AdjList<W> {
    type Weight = W;

    #[inline]
    fn len(&self) -> usize {
        self.adj.len()
    }

    #[inline]
    fn edges(&self, v: usize) -> impl Iterator<Item = (usize, W)> + '_ {
        self.adj[v].iter().copied()
    }
}

/// 単一始点からの最短距離 (辺数)。到達不能な頂点は `None`。`O(V+E)`。
///
/// ```
/// use graph::{bfs, AdjList};
/// let mut g = AdjList::<()>::new(3);
/// g.add_undirected_edge(0, 1, ());
/// g.add_undirected_edge(1, 2, ());
/// assert_eq!(bfs(&g, 0), vec![Some(0), Some(1), Some(2)]);
/// ```
pub fn bfs<G: Graph>(g: &G, src: usize) -> Vec<Option<u64>> {
    let mut dist = vec![None; g.len()];
    dist[src] = Some(0);
    let mut queue = VecDeque::from([src]);
    while let Some(u) = queue.pop_front() {
        let d = dist[u].unwrap();
        for (v, _) in g.edges(u) {
            if dist[v].is_none() {
                dist[v] = Some(d + 1);
                queue.push_back(v);
            }
        }
    }
    dist
}

/// 単一始点からの最短距離。到達不能な頂点は `None`。負の重みがあると不正。`O(E log V)`。
///
/// ```
/// use graph::{dijkstra, AdjList};
/// let mut g = AdjList::<i64>::new(3);
/// g.add_edge(0, 1, 5);
/// g.add_edge(0, 2, 1);
/// g.add_edge(2, 1, 1);
/// assert_eq!(dijkstra(&g, 0), vec![Some(0), Some(2), Some(1)]);
/// ```
pub fn dijkstra<G>(g: &G, src: usize) -> Vec<Option<G::Weight>>
where
    G: Graph,
    G::Weight: Ord + Add<Output = G::Weight> + Zero,
{
    let mut dist: Vec<Option<G::Weight>> = vec![None; g.len()];
    dist[src] = Some(G::Weight::zero());
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((G::Weight::zero(), src)));
    while let Some(Reverse((d, u))) = heap.pop() {
        if dist[u] != Some(d) {
            continue;
        }
        for (v, w) in g.edges(u) {
            let nd = d + w;
            if dist[v].is_none_or(|cur| nd < cur) {
                dist[v] = Some(nd);
                heap.push(Reverse((nd, v)));
            }
        }
    }
    dist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bfs_unreachable_is_none() {
        let mut g = AdjList::<()>::new(3);
        g.add_undirected_edge(0, 1, ());
        assert_eq!(bfs(&g, 0), vec![Some(0), Some(1), None]);
    }

    #[test]
    fn dijkstra_matches_bfs_on_unit_weights() {
        let mut g = AdjList::<i64>::new(5);
        for (u, v) in [(0, 1), (1, 2), (2, 3), (3, 4), (0, 4)] {
            g.add_undirected_edge(u, v, 1);
        }
        let bfs_dist: Vec<_> = bfs(&g, 0)
            .into_iter()
            .map(|d| d.map(|d| d as i64))
            .collect();
        assert_eq!(dijkstra(&g, 0), bfs_dist);
    }

    #[test]
    fn dijkstra_picks_shorter_weighted_path() {
        let mut g = AdjList::<i64>::new(4);
        g.add_edge(0, 1, 10);
        g.add_edge(0, 2, 1);
        g.add_edge(2, 3, 1);
        g.add_edge(3, 1, 1);
        assert_eq!(dijkstra(&g, 0), vec![Some(0), Some(3), Some(1), Some(2)]);
    }
}
