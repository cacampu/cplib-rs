//! `Graph` トレイトに対する単一始点最短路アルゴリズム。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::ops::Add;

use algebra::{One, Zero};
use graph::Graph;

/// 単一始点からの最短距離 (辺数)。到達不能な頂点は `None`。`O(V+E)`。
///
/// ```
/// use graph::AdjList;
/// use shortest_path::bfs;
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

/// 辺の重みが `0` か `1` だけのグラフでの単一始点距離。到達不能なら `None`。`O(V+E)`。
///
/// ```
/// use graph::AdjList;
/// use shortest_path::zero_one_bfs;
/// let mut g = AdjList::<u32>::new(3);
/// g.add_edge(0, 1, 1);
/// g.add_edge(0, 2, 0);
/// g.add_edge(2, 1, 0);
/// assert_eq!(zero_one_bfs(&g, 0), vec![Some(0), Some(0), Some(0)]);
/// ```
pub fn zero_one_bfs<G>(g: &G, src: usize) -> Vec<Option<u64>>
where
    G: Graph,
    G::Weight: PartialEq + Zero + One,
{
    let mut dist = vec![None; g.len()];
    let mut visited = vec![false; g.len()];
    dist[src] = Some(0);
    let mut deque = VecDeque::from([src]);
    while let Some(u) = deque.pop_front() {
        if visited[u] {
            continue;
        }
        visited[u] = true;
        let d = dist[u].unwrap();
        for (v, w) in g.edges(u) {
            let step = if w == G::Weight::zero() {
                0
            } else if w == G::Weight::one() {
                1
            } else {
                panic!("zero_one_bfs requires edge weights to be 0 or 1");
            };
            let nd = d + step;
            if dist[v].is_none_or(|cur| nd < cur) {
                dist[v] = Some(nd);
                if step == 0 {
                    deque.push_front(v);
                } else {
                    deque.push_back(v);
                }
            }
        }
    }
    dist
}

/// 単一始点からの最短距離。到達不能な頂点は `None`。負の重みがあると不正。`O(E log V)`。
///
/// ```
/// use graph::AdjList;
/// use shortest_path::dijkstra;
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

/// 単一始点からの最短距離。負の重みを許し、`src` から到達可能な負サイクルが
/// あれば `None`。`O(VE)`。
///
/// ```
/// use graph::AdjList;
/// use shortest_path::bellman_ford;
/// let mut g = AdjList::<i64>::new(3);
/// g.add_edge(0, 1, 5);
/// g.add_edge(0, 2, 10);
/// g.add_edge(1, 2, -8);
/// assert_eq!(bellman_ford(&g, 0), Some(vec![Some(0), Some(5), Some(-3)]));
/// ```
pub fn bellman_ford<G>(g: &G, src: usize) -> Option<Vec<Option<G::Weight>>>
where
    G: Graph,
    G::Weight: Ord + Add<Output = G::Weight> + Zero,
{
    let n = g.len();
    let mut dist: Vec<Option<G::Weight>> = vec![None; n];
    dist[src] = Some(G::Weight::zero());
    for i in 0..n {
        let mut updated = false;
        for u in 0..n {
            let Some(du) = dist[u] else { continue };
            for (v, w) in g.edges(u) {
                let nd = du + w;
                if dist[v].is_none_or(|cur| nd < cur) {
                    if i == n - 1 {
                        return None;
                    }
                    dist[v] = Some(nd);
                    updated = true;
                }
            }
        }
        if !updated {
            break;
        }
    }
    Some(dist)
}

#[cfg(test)]
mod tests {
    use graph::AdjList;

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

    fn xorshift() -> impl FnMut() -> u64 {
        let mut state = 88172645463325252u64;
        move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        }
    }

    #[test]
    fn zero_one_bfs_matches_dijkstra() {
        let mut next = xorshift();
        for _ in 0..20 {
            let n = 1 + (next() % 20) as usize;
            let mut g01 = AdjList::<u32>::new(n);
            let mut gw = AdjList::<i64>::new(n);
            for u in 0..n {
                for v in 0..n {
                    if u != v && next() % 3 == 0 {
                        let w = (next() % 2) as u32;
                        g01.add_edge(u, v, w);
                        gw.add_edge(u, v, w as i64);
                    }
                }
            }
            let expected: Vec<_> = dijkstra(&gw, 0)
                .into_iter()
                .map(|d| d.map(|d| d as u64))
                .collect();
            assert_eq!(zero_one_bfs(&g01, 0), expected, "n={n}");
        }
    }

    #[test]
    fn bellman_ford_matches_dijkstra_on_nonnegative_weights() {
        let mut next = xorshift();
        for _ in 0..20 {
            let n = 1 + (next() % 20) as usize;
            let mut g = AdjList::<i64>::new(n);
            for u in 0..n {
                for v in 0..n {
                    if u != v && next() % 3 == 0 {
                        g.add_edge(u, v, (next() % 20) as i64);
                    }
                }
            }
            assert_eq!(bellman_ford(&g, 0), Some(dijkstra(&g, 0)), "n={n}");
        }
    }

    #[test]
    fn bellman_ford_handles_negative_edges() {
        let mut g = AdjList::<i64>::new(3);
        g.add_edge(0, 1, 5);
        g.add_edge(0, 2, 10);
        g.add_edge(1, 2, -8);
        assert_eq!(bellman_ford(&g, 0), Some(vec![Some(0), Some(5), Some(-3)]));
    }

    #[test]
    fn bellman_ford_detects_reachable_negative_cycle() {
        let mut g = AdjList::<i64>::new(3);
        g.add_edge(0, 1, 1);
        g.add_edge(1, 2, -1);
        g.add_edge(2, 1, -1);
        assert_eq!(bellman_ford(&g, 0), None);
    }

    #[test]
    fn bellman_ford_ignores_negative_cycle_unreachable_from_src() {
        let mut g = AdjList::<i64>::new(4);
        g.add_edge(0, 1, 3);
        g.add_edge(2, 3, -1);
        g.add_edge(3, 2, -1);
        assert_eq!(
            bellman_ford(&g, 0),
            Some(vec![Some(0), Some(3), None, None])
        );
    }
}
