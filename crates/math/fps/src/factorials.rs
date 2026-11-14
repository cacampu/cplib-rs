use modint::ModInt;

/// `0..=n` の階乗・階乗逆元。指数型母関数や二項係数に使う。
/// 法 `M` は素数で、`n < M` が必要。前計算 `O(n + log M)`。
///
/// ```
/// use fps::{Factorials, Fps};
/// use modint::ModInt998244353 as Mi;
/// let table = Factorials::<998244353>::new(5);
/// assert_eq!(table.binom(5, 2), Mi::new(10));
/// // n 個のラベル付き要素に対する個数 a[n] を EGF の係数 a[n]/n! にする。
/// let counts = [1, 2, 3];
/// let egf = Fps::new(counts.iter().enumerate()
///     .map(|(i, &a)| Mi::new(a) * table.inv_fact[i]).collect());
/// assert_eq!(egf[2] * table.fact[2], Mi::new(3));
/// ```
#[derive(Clone, Debug)]
pub struct Factorials<const M: u32> {
    pub fact: Vec<ModInt<M>>,
    pub inv_fact: Vec<ModInt<M>>,
}

impl<const M: u32> Factorials<M> {
    pub fn new(n: usize) -> Self {
        assert!(
            n < M as usize,
            "factorial degree must be less than the modulus"
        );
        let mut fact = vec![ModInt::new(1u32); n + 1];
        for i in 1..=n {
            fact[i] = fact[i - 1] * ModInt::new(i);
        }
        let mut inv_fact = vec![ModInt::new(1u32); n + 1];
        inv_fact[n] = fact[n].inv();
        for i in (1..=n).rev() {
            inv_fact[i - 1] = inv_fact[i] * ModInt::new(i);
        }
        Self { fact, inv_fact }
    }

    /// `n choose k`。`k > n` なら 0。`n` は前計算範囲内であること。
    pub fn binom(&self, n: usize, k: usize) -> ModInt<M> {
        if k > n {
            return ModInt::raw(0);
        }
        self.fact[n] * self.inv_fact[k] * self.inv_fact[n - k]
    }
}
