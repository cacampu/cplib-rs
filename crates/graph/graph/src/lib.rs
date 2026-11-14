//! グラフアルゴリズムを頂点番号 `0..n` に対して統一的に書くための `Graph` トレイト。

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
/// `AdjList::edges` はスライス、`Graph::edges` は辺をコピーするイテレータを返す。
///
/// ```
/// use graph::AdjList;
/// let mut g = AdjList::<i64>::new(3);
/// g.add_undirected_edge(0, 1, 5);
/// g.add_edge(1, 2, 3); // 有向辺
/// assert_eq!(g.edges(1), &[(0, 5), (2, 3)]);
/// assert_eq!(g.edges(1).len(), 2);
/// assert_eq!(g.edges(1)[0], (0, 5));
/// ```
#[derive(Clone, Debug)]
pub struct AdjList<W = ()> {
    /// 頂点ごとの `(行き先, 重み)`。辺の並び替え・追加・削除に直接使える。
    pub adj: Vec<Vec<(usize, W)>>,
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

    /// 頂点 `v` の辺をスライスとして返す。`len()` や添字アクセスに使える。
    #[inline]
    pub fn edges(&self, v: usize) -> &[(usize, W)] {
        &self.adj[v]
    }

    /// 頂点 `v` の辺を直接編集する。並び替えや重みの変更に使う。
    ///
    /// ```
    /// use graph::AdjList;
    /// let mut g = AdjList::new(3);
    /// g.add_edge(0, 2, 1u32);
    /// g.add_edge(0, 1, 1);
    /// let edges = g.edges_mut(0);
    /// edges.sort_unstable_by_key(|&(to, _)| to);
    /// edges[0].1 = 0;
    /// assert_eq!(g.edges(0), &[(1, 0), (2, 1)]);
    /// ```
    pub fn edges_mut(&mut self, v: usize) -> &mut [(usize, W)] {
        &mut self.adj[v]
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
        self.edges(v).iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directed_edge_is_one_way() {
        let mut g = AdjList::<i64>::new(2);
        g.add_edge(0, 1, 7);
        assert_eq!(g.edges(0), &[(1, 7)]);
        assert!(g.edges(1).is_empty());
    }

    #[test]
    fn undirected_edge_is_two_way() {
        let mut g = AdjList::<i64>::new(2);
        g.add_undirected_edge(0, 1, 7);
        assert_eq!(g.edges(0), &[(1, 7)]);
        assert_eq!(g.edges(1), &[(0, 7)]);
    }
}
