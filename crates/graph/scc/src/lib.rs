//! 有向グラフの強連結成分分解 (Tarjan)。

use graph::Graph;

/// 各頂点の属する強連結成分の番号。番号は縮約後の DAG の
/// トポロジカル順 (辺は小さい番号から大きい番号へ) になる。`O(V+E)`。
///
/// ```
/// use graph::AdjList;
/// use scc::scc;
/// let mut g = AdjList::<()>::new(4);
/// for &(u, v) in &[(0, 1), (1, 2), (2, 0), (2, 3)] {
///     g.add_edge(u, v, ());
/// }
/// let id = scc(&g);
/// assert_eq!(id[0], id[1]);
/// assert_eq!(id[1], id[2]);
/// assert!(id[2] < id[3]);
/// ```
pub fn scc<G: Graph>(g: &G) -> Vec<usize> {
    let n = g.len();
    let mut ord = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut on_stack = vec![false; n];
    let mut stack = Vec::new();
    let mut id = vec![usize::MAX; n];
    let mut timer = 0;
    let mut comp = 0;

    for s in 0..n {
        if ord[s] == usize::MAX {
            dfs(
                s,
                g,
                &mut timer,
                &mut comp,
                &mut ord,
                &mut low,
                &mut on_stack,
                &mut stack,
                &mut id,
            );
        }
    }

    // comp は閉じた順 (トポロジカル順の逆) に振られているので反転する。
    for v in id.iter_mut() {
        *v = comp - 1 - *v;
    }
    id
}

#[allow(clippy::too_many_arguments)]
fn dfs<G: Graph>(
    u: usize,
    g: &G,
    timer: &mut usize,
    comp: &mut usize,
    ord: &mut [usize],
    low: &mut [usize],
    on_stack: &mut [bool],
    stack: &mut Vec<usize>,
    id: &mut [usize],
) {
    ord[u] = *timer;
    low[u] = *timer;
    *timer += 1;
    stack.push(u);
    on_stack[u] = true;

    for (v, _) in g.edges(u) {
        if ord[v] == usize::MAX {
            dfs(v, g, timer, comp, ord, low, on_stack, stack, id);
            low[u] = low[u].min(low[v]);
        } else if on_stack[v] {
            low[u] = low[u].min(ord[v]);
        }
    }

    if low[u] == ord[u] {
        loop {
            let v = stack.pop().unwrap();
            on_stack[v] = false;
            id[v] = *comp;
            if v == u {
                break;
            }
        }
        *comp += 1;
    }
}

#[cfg(test)]
mod tests {
    use graph::AdjList;

    use super::*;

    #[test]
    fn two_separate_cycles_are_different_components() {
        let mut g = AdjList::<()>::new(4);
        for &(u, v) in &[(0, 1), (1, 0), (2, 3), (3, 2)] {
            g.add_edge(u, v, ());
        }
        let id = scc(&g);
        assert_eq!(id[0], id[1]);
        assert_eq!(id[2], id[3]);
        assert_ne!(id[0], id[2]);
    }

    #[test]
    fn dag_each_vertex_is_its_own_component_in_topological_order() {
        let mut g = AdjList::<()>::new(3);
        g.add_edge(0, 1, ());
        g.add_edge(1, 2, ());
        let id = scc(&g);
        assert_eq!(id, vec![0, 1, 2]);
    }

    #[test]
    fn single_cycle_through_all_vertices() {
        let n = 6;
        let mut g = AdjList::<()>::new(n);
        for i in 0..n {
            g.add_edge(i, (i + 1) % n, ());
        }
        let id = scc(&g);
        assert!(id.iter().all(|&c| c == id[0]));
    }
}
