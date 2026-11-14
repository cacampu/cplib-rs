use modint::ModInt;

use crate::Fps;

impl<const M: u32> Fps<M> {
    /// 疎な多項式との積をちょうど `n` 項求める。`O(n * terms.len())`。
    /// `terms` は `(次数, 係数)`。順序は自由で、同じ次数が複数あってもよい。
    pub fn mul_sparse(&self, terms: &[(usize, ModInt<M>)], n: usize) -> Self {
        let mut result = Self::zeros(n);
        for &(degree, c) in terms {
            if degree >= n || c.val() == 0 {
                continue;
            }
            for (i, &a) in self.coef.iter().take(n - degree).enumerate() {
                result.coef[i + degree] += a * c;
            }
        }
        result
    }

    /// 疎な多項式で割った級数をちょうど `n` 項求める。`O(n * terms.len())`。
    /// `terms` の形式は `mul_sparse` と同じ。定数項の合計が非零であること。
    pub fn div_sparse(&self, terms: &[(usize, ModInt<M>)], n: usize) -> Self {
        let constant: ModInt<M> = terms
            .iter()
            .filter(|&&(degree, _)| degree == 0)
            .map(|&(_, c)| c)
            .sum();
        assert!(
            constant.val() != 0,
            "division requires a non-zero constant term"
        );
        let inv = constant.inv();
        let mut result = Self::zeros(n);
        for i in 0..n {
            let mut c = self.at(i);
            for &(degree, a) in terms {
                if degree > 0 && degree <= i {
                    c -= a * result.coef[i - degree];
                }
            }
            result.coef[i] = c * inv;
        }
        result
    }

    /// `(1 + c x^degree)` をその場で掛ける。項数は変えず、超過分を打ち切る。`O(len)`。
    ///
    /// ```
    /// use fps::Fps;
    /// use modint::ModInt998244353 as Mi;
    /// let mut f = Fps::from_ints(&[1, 2, 3, 4]);
    /// f.mul_binomial_assign(2, Mi::new(-1));
    /// assert_eq!(f, Fps::from_ints(&[1, 2, 2, 2]));
    /// f.div_binomial_assign(2, Mi::new(-1));
    /// assert_eq!(f, Fps::from_ints(&[1, 2, 3, 4]));
    /// ```
    pub fn mul_binomial_assign(&mut self, degree: usize, c: ModInt<M>) {
        if degree == 0 {
            *self *= ModInt::new(1u32) + c;
            return;
        }
        for i in (degree..self.len()).rev() {
            let lower = self.coef[i - degree];
            self.coef[i] += c * lower;
        }
    }

    /// `(1 + c x^degree)` でその場で割る。項数は変えない。`O(len)`。
    /// `degree = 0` のときは `1 + c != 0` が必要。
    pub fn div_binomial_assign(&mut self, degree: usize, c: ModInt<M>) {
        if degree == 0 {
            let constant = ModInt::new(1u32) + c;
            assert!(
                constant.val() != 0,
                "division requires a non-zero constant term"
            );
            *self *= constant.inv();
            return;
        }
        for i in degree..self.len() {
            let lower = self.coef[i - degree];
            self.coef[i] -= c * lower;
        }
    }
}
