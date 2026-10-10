//! 木の重軽分解 (Heavy-Light Decomposition)。LCA、パス/部分木のセグ木クエリ用の区間化。

use graph::Graph;

/// `Hld::new` で `O(n)` 前計算すると、LCA は `O(log n)`、パス・部分木は
/// `idx` 空間での連続区間として `O(log n)` 個の区間に分解できる。
///
/// ```
/// use graph::AdjList;
/// use hld::Hld;
/// let mut g = AdjList::<()>::new(5);
/// for &(u, v) in &[(0, 1), (0, 2), (1, 3), (1, 4)] {
///     g.add_undirected_edge(u, v, ());
/// }
/// let hld = Hld::new(&g, 0);
/// assert_eq!(hld.lca(3, 4), 1);
/// assert_eq!(hld.lca(3, 2), 0);
/// ```
pub struct Hld {
    parent: Vec<usize>,
    depth: Vec<usize>,
    size: Vec<usize>,
    head: Vec<usize>,
    idx: Vec<usize>,
    vertex: Vec<usize>,
}

impl Hld {
    pub fn new<G: Graph>(g: &G, root: usize) -> Self {
        let n = g.len();
        let mut parent = vec![usize::MAX; n];
        let mut size = vec![1; n];
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
        dfs_size(root, usize::MAX, g, &mut parent, &mut size, &mut children);
        for c in children.iter_mut() {
            c.sort_unstable_by_key(|&v| usize::MAX - size[v]); // 重い子を先頭に
        }

        let mut depth = vec![0; n];
        let mut head = vec![0; n];
        let mut idx = vec![0; n];
        let mut vertex = vec![0; n];
        let mut timer = 0;
        dfs_order(
            root,
            root,
            0,
            &children,
            &mut depth,
            &mut head,
            &mut idx,
            &mut vertex,
            &mut timer,
        );

        Self {
            parent,
            depth,
            size,
            head,
            idx,
            vertex,
        }
    }

    /// 頂点のオイラー順 (DFS 前順) での位置。
    #[inline]
    pub fn idx(&self, v: usize) -> usize {
        self.idx[v]
    }

    /// `idx` の位置にある頂点。
    #[inline]
    pub fn vertex_at(&self, i: usize) -> usize {
        self.vertex[i]
    }

    pub fn parent(&self, v: usize) -> Option<usize> {
        (self.parent[v] != usize::MAX).then(|| self.parent[v])
    }

    #[inline]
    pub fn depth(&self, v: usize) -> usize {
        self.depth[v]
    }

    /// `v` の部分木が占める `idx` 空間での半開区間。
    #[inline]
    pub fn subtree_range(&self, v: usize) -> (usize, usize) {
        (self.idx[v], self.idx[v] + self.size[v])
    }

    pub fn lca(&self, mut u: usize, mut v: usize) -> usize {
        while self.head[u] != self.head[v] {
            if self.depth[self.head[u]] < self.depth[self.head[v]] {
                std::mem::swap(&mut u, &mut v);
            }
            u = self.parent[self.head[u]];
        }
        if self.depth[u] < self.depth[v] { u } else { v }
    }

    /// `u`-`v` 間のパスを、`idx` 空間での半開区間の列に分解する。
    pub fn path_ranges(&self, mut u: usize, mut v: usize) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        while self.head[u] != self.head[v] {
            if self.depth[self.head[u]] < self.depth[self.head[v]] {
                std::mem::swap(&mut u, &mut v);
            }
            ranges.push((self.idx[self.head[u]], self.idx[u] + 1));
            u = self.parent[self.head[u]];
        }
        let (lo, hi) = if self.idx[u] <= self.idx[v] {
            (self.idx[u], self.idx[v])
        } else {
            (self.idx[v], self.idx[u])
        };
        ranges.push((lo, hi + 1));
        ranges
    }
}

fn dfs_size<G: Graph>(
    u: usize,
    p: usize,
    g: &G,
    parent: &mut [usize],
    size: &mut [usize],
    children: &mut [Vec<usize>],
) {
    parent[u] = p;
    for (v, _) in g.edges(u) {
        if v == p {
            continue;
        }
        dfs_size(v, u, g, parent, size, children);
        size[u] += size[v];
        children[u].push(v);
    }
}

#[allow(clippy::too_many_arguments)]
fn dfs_order(
    u: usize,
    h: usize,
    d: usize,
    children: &[Vec<usize>],
    depth: &mut [usize],
    head: &mut [usize],
    idx: &mut [usize],
    vertex: &mut [usize],
    timer: &mut usize,
) {
    depth[u] = d;
    head[u] = h;
    idx[u] = *timer;
    vertex[*timer] = u;
    *timer += 1;
    for (i, &v) in children[u].iter().enumerate() {
        let child_head = if i == 0 { h } else { v };
        dfs_order(
            v,
            child_head,
            d + 1,
            children,
            depth,
            head,
            idx,
            vertex,
            timer,
        );
    }
}

#[cfg(test)]
mod tests {
    use graph::AdjList;

    use super::*;

    fn random_tree(n: usize, next: &mut impl FnMut() -> u64) -> AdjList<()> {
        let mut g = AdjList::<()>::new(n);
        for v in 1..n {
            let p = (next() % v as u64) as usize;
            g.add_undirected_edge(p, v, ());
        }
        g
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

    fn naive_parent(g: &AdjList<()>, root: usize) -> Vec<usize> {
        let mut parent = vec![usize::MAX; g.len()];
        let mut stack = vec![root];
        let mut seen = vec![false; g.len()];
        seen[root] = true;
        while let Some(u) = stack.pop() {
            for (v, _) in g.edges(u) {
                if !seen[v] {
                    seen[v] = true;
                    parent[v] = u;
                    stack.push(v);
                }
            }
        }
        parent
    }

    fn naive_path(
        parent: &[usize],
        mut u: usize,
        mut v: usize,
    ) -> std::collections::BTreeSet<usize> {
        fn depth_of(parent: &[usize], mut v: usize) -> usize {
            let mut d = 0;
            while parent[v] != usize::MAX {
                v = parent[v];
                d += 1;
            }
            d
        }
        let mut set = std::collections::BTreeSet::new();
        let (mut du, mut dv) = (depth_of(parent, u), depth_of(parent, v));
        while du > dv {
            set.insert(u);
            u = parent[u];
            du -= 1;
        }
        while dv > du {
            set.insert(v);
            v = parent[v];
            dv -= 1;
        }
        while u != v {
            set.insert(u);
            set.insert(v);
            u = parent[u];
            v = parent[v];
        }
        set.insert(u);
        set
    }

    #[test]
    fn lca_matches_naive() {
        let mut next = xorshift();
        for _ in 0..20 {
            let n = 1 + (next() % 50) as usize;
            let g = random_tree(n, &mut next);
            let parent = naive_parent(&g, 0);
            let hld = Hld::new(&g, 0);
            for _ in 0..30 {
                let u = (next() % n as u64) as usize;
                let v = (next() % n as u64) as usize;
                let path = naive_path(&parent, u, v);
                let naive_lca = *path
                    .iter()
                    .min_by_key(|&&x| {
                        let mut d = 0;
                        let mut y = x;
                        while parent[y] != usize::MAX {
                            y = parent[y];
                            d += 1;
                        }
                        d
                    })
                    .unwrap();
                assert_eq!(hld.lca(u, v), naive_lca, "n={n}, u={u}, v={v}");
            }
        }
    }

    #[test]
    fn path_ranges_cover_exactly_the_path() {
        let mut next = xorshift();
        for _ in 0..20 {
            let n = 1 + (next() % 50) as usize;
            let g = random_tree(n, &mut next);
            let parent = naive_parent(&g, 0);
            let hld = Hld::new(&g, 0);
            for _ in 0..30 {
                let u = (next() % n as u64) as usize;
                let v = (next() % n as u64) as usize;
                let expected = naive_path(&parent, u, v);
                let mut got = std::collections::BTreeSet::new();
                for (l, r) in hld.path_ranges(u, v) {
                    for i in l..r {
                        got.insert(hld.vertex_at(i));
                    }
                }
                assert_eq!(got, expected, "n={n}, u={u}, v={v}");
            }
        }
    }

    #[test]
    fn subtree_range_covers_exactly_the_subtree() {
        let mut next = xorshift();
        let n = 40;
        let g = random_tree(n, &mut next);
        let parent = naive_parent(&g, 0);
        let hld = Hld::new(&g, 0);
        for v in 0..n {
            let mut expected = std::collections::BTreeSet::new();
            for x in 0..n {
                let mut y = x;
                loop {
                    if y == v {
                        expected.insert(x);
                        break;
                    }
                    if parent[y] == usize::MAX {
                        break;
                    }
                    y = parent[y];
                }
            }
            let (l, r) = hld.subtree_range(v);
            let got: std::collections::BTreeSet<usize> = (l..r).map(|i| hld.vertex_at(i)).collect();
            assert_eq!(got, expected, "v={v}");
        }
    }
}
