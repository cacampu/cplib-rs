pub trait RerootingAlgebra {
    type V: Clone;
    type E: Clone;
    type N: Clone;
    fn identity(&self) -> Self::V;
    fn merge(&self, a: &Self::V, b: &Self::V) -> Self::V;
    fn apply_edge(&self, val: &Self::V, edge: &Self::E) -> Self::V;
    fn apply_node(&self, val: &Self::V, node: &Self::N) -> Self::V;
}

pub struct RerootingDP<RA: RerootingAlgebra> {
    _n: usize,
    adj: Vec<Vec<(usize, RA::E)>>,
    nodes: Vec<RA::N>,
    dp: Vec<Vec<RA::V>>,
    ans: Vec<RA::V>,
    logic: RA,
}
impl<RA: RerootingAlgebra> RerootingDP<RA> {
    pub fn new(nodes_data: Vec<RA::N>, logic: RA) -> Self {
        let n = nodes_data.len();
        RerootingDP {
            _n: n,
            adj: vec![vec![]; n],
            nodes: nodes_data,
            dp: vec![vec![]; n],
            ans: vec![logic.identity(); n],
            logic,
        }
    }
    pub fn add_edge(&mut self, u: usize, v: usize, edge_data: RA::E) {
        self.adj[u].push((v, edge_data.clone()));
        self.adj[v].push((u, edge_data));
    }
    pub fn solve(&mut self) -> Vec<RA::V> {
        self.dfs1(0, usize::MAX);
        self.dfs2(0, usize::MAX, &self.logic.identity());
        self.ans.clone()
    }
    fn dfs1(&mut self, u: usize, p: usize) -> RA::V {
        let c_len = self.adj[u].len();
        let mut prod = self.logic.identity();
        self.dp[u] = vec![self.logic.identity(); c_len];

        for i in 0..c_len {
            let (v, edge) = self.adj[u][i].clone();
            if v == p {
                continue;
            }
            let child_val = self.dfs1(v, u);
            let lifted_val = self.logic.apply_edge(&child_val, &edge);
            prod = self.logic.merge(&prod, &lifted_val);
            self.dp[u][i] = lifted_val;
        }
        self.logic.apply_node(&prod, &self.nodes[u])
    }
    fn dfs2(&mut self, u: usize, p: usize, val_from_parent: &RA::V) {
        if let Some(i) = self.adj[u].iter().position(|&(v, _)| v == p) {
            self.dp[u][i] = val_from_parent.clone();
        }
        let c_len = self.adj[u].len();
        let mut l_prod = vec![self.logic.identity(); c_len + 1]; //[0,i)
        let mut r_prod = vec![self.logic.identity(); c_len + 1]; //[i,n)

        for i in 0..c_len {
            l_prod[i + 1] = self.logic.merge(&l_prod[i], &self.dp[u][i]);
        }
        for i in (0..c_len).rev() {
            r_prod[i] = self.logic.merge(&self.dp[u][i], &r_prod[i + 1]);
        }

        let all_prod = l_prod[c_len].clone();
        self.ans[u] = self.logic.apply_node(&all_prod, &self.nodes[u]);

        for i in 0..c_len {
            let (v, edge) = self.adj[u][i].clone();
            if v == p {
                continue;
            }
            let prod_without_v = self.logic.merge(&l_prod[i], &r_prod[i + 1]);
            let apply_u = self.logic.apply_node(&prod_without_v, &self.nodes[u]);
            let val_to_v = self.logic.apply_edge(&apply_u, &edge);
            self.dfs2(v, u, &val_to_v);
        }
    }
}
