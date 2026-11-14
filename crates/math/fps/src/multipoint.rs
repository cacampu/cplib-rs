use modint::ModInt;

use crate::Fps;

struct ProductTree<const M: u32> {
    size: usize,
    count: usize,
    nodes: Vec<Fps<M>>,
}

impl<const M: u32> ProductTree<M> {
    fn new(points: &[ModInt<M>]) -> Self {
        let size = points.len().next_power_of_two();
        let mut nodes = vec![Fps::constant(ModInt::new(1u32)); size * 2];
        for (i, &x) in points.iter().enumerate() {
            nodes[size + i] = Fps::new(vec![-x, ModInt::new(1u32)]);
        }
        for i in (1..size).rev() {
            nodes[i] = nodes[i * 2].clone() * nodes[i * 2 + 1].clone();
        }
        Self {
            size,
            count: points.len(),
            nodes,
        }
    }

    fn evaluate(&self, f: &Fps<M>) -> Vec<ModInt<M>> {
        let mut result = vec![ModInt::raw(0); self.count];
        self.evaluate_at(1, f.div_rem(&self.nodes[1]).1, &mut result);
        result
    }

    fn evaluate_at(&self, node: usize, f: Fps<M>, result: &mut [ModInt<M>]) {
        if node >= self.size {
            if node - self.size < self.count {
                result[node - self.size] = f.at(0);
            }
            return;
        }
        let left = node * 2;
        let right = left + 1;
        self.evaluate_at(left, f.div_rem(&self.nodes[left]).1, result);
        self.evaluate_at(right, f.div_rem(&self.nodes[right]).1, result);
    }
}

impl<const M: u32> Fps<M> {
    /// 各点での値を入力と同じ順序で返す。重複する点も扱える。
    /// `d = self.len()`、`m = points.len()` として
    /// `O(d log(d+1) + m log^2(m+1))` 時間、`O(d + m log(m+1))` 空間。
    ///
    /// ```
    /// use fps::Fps;
    /// use modint::ModInt998244353 as Mi;
    /// let f = Fps::from_ints(&[1, 2, 3]);
    /// let points = [0, 1, 2].map(Mi::new);
    /// assert_eq!(f.multipoint_eval(&points), [1, 6, 17].map(Mi::new));
    /// ```
    pub fn multipoint_eval(&self, points: &[ModInt<M>]) -> Vec<ModInt<M>> {
        if points.is_empty() {
            return vec![];
        }
        ProductTree::new(points).evaluate(self)
    }
}

/// `f(points[i]) = values[i]` を満たす、次数が点の個数未満の多項式を返す。
/// 点は相異なること (法 `M` で比較)。項数はちょうど点の個数。空入力なら零多項式。
/// `O(n log^2(n+1))` 時間、`O(n log(n+1))` 空間。
pub fn interpolate<const M: u32>(points: &[ModInt<M>], values: &[ModInt<M>]) -> Fps<M> {
    assert_eq!(
        points.len(),
        values.len(),
        "points and values must have equal lengths"
    );
    if points.is_empty() {
        return Fps::zeros(0);
    }
    let tree = ProductTree::new(points);
    let derivatives = tree.evaluate(&tree.nodes[1].derivative());
    let mut parts = vec![Fps::zeros(0); tree.size * 2];
    for i in 0..points.len() {
        assert!(
            derivatives[i].val() != 0,
            "interpolation points must be distinct"
        );
        parts[tree.size + i] = Fps::constant(values[i] / derivatives[i]);
    }
    for i in (1..tree.size).rev() {
        let left = std::mem::take(&mut parts[i * 2]);
        let right = std::mem::take(&mut parts[i * 2 + 1]);
        parts[i] = left * tree.nodes[i * 2 + 1].clone() + right * tree.nodes[i * 2].clone();
    }
    std::mem::take(&mut parts[1]).pre(points.len())
}
