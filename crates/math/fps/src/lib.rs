//! 形式的冪級数 (formal power series)。
//!
//! 係数は `ModInt<M>` で、`M` は NTT-friendly な素数であること (998244353 など)。
//!
//! `inv` / `exp` / `log` / `pow` / `sqrt` はいずれも Newton 法で、
//! 精度を倍々にしながら畳み込みを繰り返すため `O(n log n)`。
//! 打ち切り項数は呼び出し側が明示的に渡す。

use std::ops::{Add, AddAssign, Index, Mul, MulAssign, Neg, Sub, SubAssign};

use convolution::convolution;
use modint::ModInt;

/// 係数列 `a[0] + a[1] x + a[2] x^2 + ...`。
///
/// 末尾の 0 は詰めない。項数は `len()` がそのまま表す。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Fps<const M: u32> {
    coef: Vec<ModInt<M>>,
}

impl<const M: u32> Fps<M> {
    pub fn new(coef: Vec<ModInt<M>>) -> Self {
        Self { coef }
    }

    /// 項数 `n` の零級数。
    pub fn zeros(n: usize) -> Self {
        Self {
            coef: vec![ModInt::raw(0); n],
        }
    }

    /// 定数 `c` のみの級数。
    pub fn constant(c: ModInt<M>) -> Self {
        Self { coef: vec![c] }
    }

    /// 整数列から作る。
    pub fn from_ints<T: Copy + Into<i64>>(values: &[T]) -> Self {
        Self {
            coef: values.iter().map(|&x| ModInt::new(x.into())).collect(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.coef.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.coef.is_empty()
    }

    #[inline]
    pub fn coef(&self) -> &[ModInt<M>] {
        &self.coef
    }

    #[inline]
    pub fn coef_mut(&mut self) -> &mut Vec<ModInt<M>> {
        &mut self.coef
    }

    pub fn into_vec(self) -> Vec<ModInt<M>> {
        self.coef
    }

    /// `i` 次の係数。範囲外なら 0。
    #[inline]
    pub fn at(&self, i: usize) -> ModInt<M> {
        self.coef.get(i).copied().unwrap_or(ModInt::raw(0))
    }

    /// ちょうど `n` 項に揃える。足りなければ 0 で埋め、多ければ切る。
    pub fn pre(&self, n: usize) -> Self {
        let mut coef = self.coef.clone();
        coef.resize(n, ModInt::raw(0));
        Self { coef }
    }

    /// 末尾の 0 を落とす。
    pub fn shrink(mut self) -> Self {
        while self.coef.last().is_some_and(|x| x.val() == 0) {
            self.coef.pop();
        }
        self
    }

    /// 最低次の非零項の次数。全て 0 なら `None`。
    pub fn lowest_degree(&self) -> Option<usize> {
        self.coef.iter().position(|x| x.val() != 0)
    }

    /// `x^k` 倍する。
    pub fn shift_up(&self, k: usize) -> Self {
        let mut coef = vec![ModInt::raw(0); k];
        coef.extend_from_slice(&self.coef);
        Self { coef }
    }

    /// `x^k` で割る (下位 `k` 項を捨てる)。
    pub fn shift_down(&self, k: usize) -> Self {
        Self {
            coef: self.coef.get(k..).unwrap_or(&[]).to_vec(),
        }
    }

    /// 微分。
    pub fn derivative(&self) -> Self {
        if self.coef.len() <= 1 {
            return Self { coef: vec![] };
        }
        let coef = self
            .coef
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, &c)| c * ModInt::new(i as u64))
            .collect();
        Self { coef }
    }

    /// 積分。定数項は 0 とする。
    pub fn integral(&self) -> Self {
        let n = self.coef.len();
        let mut coef = vec![ModInt::raw(0); n + 1];
        // 1..=n の逆元をまとめて求める
        let inv = inverse_table::<M>(n + 1);
        for (i, &c) in self.coef.iter().enumerate() {
            coef[i + 1] = c * inv[i + 1];
        }
        Self { coef }
    }

    /// 逆元を `n` 項求める。定数項が 0 だとパニックする。
    pub fn inv(&self, n: usize) -> Self {
        assert!(
            self.at(0).val() != 0,
            "inv requires a non-zero constant term"
        );
        if n == 0 {
            return Self { coef: vec![] };
        }
        // g_{2k} = g_k (2 - f g_k) mod x^{2k}
        let mut g = vec![self.at(0).inv()];
        let mut k = 1;
        while k < n {
            k *= 2;
            let f = &self.coef[..self.coef.len().min(k)];
            let mut fg = convolution(f, &g);
            fg.truncate(k);
            for x in fg.iter_mut() {
                *x = -*x;
            }
            fg[0] += ModInt::new(2u32);
            let mut next = convolution(&g, &fg);
            next.truncate(k);
            g = next;
        }
        g.resize(n, ModInt::raw(0));
        Self { coef: g }
    }

    /// `log` を `n` 項求める。定数項が 1 でないとパニックする。
    pub fn log(&self, n: usize) -> Self {
        assert!(
            self.at(0) == ModInt::new(1u32),
            "log requires the constant term to be 1"
        );
        if n == 0 {
            return Self { coef: vec![] };
        }
        // log f = ∫ f' / f
        let d = self.pre(n).derivative();
        let inv = self.inv(n);
        let mut prod = convolution(d.coef(), inv.coef());
        prod.truncate(n.saturating_sub(1));
        Self { coef: prod }.integral().pre(n)
    }

    /// `exp` を `n` 項求める。定数項が 0 でないとパニックする。
    pub fn exp(&self, n: usize) -> Self {
        assert!(
            self.at(0).val() == 0,
            "exp requires the constant term to be 0"
        );
        if n == 0 {
            return Self { coef: vec![] };
        }
        // g_{2k} = g_k (1 + f - log g_k) mod x^{2k}
        let mut g = Self {
            coef: vec![ModInt::new(1u32)],
        };
        let mut k = 1;
        while k < n {
            k *= 2;
            let mut t = self.pre(k) - g.log(k);
            t.coef[0] += ModInt::new(1u32);
            let mut next = convolution(g.coef(), t.coef());
            next.truncate(k);
            g = Self { coef: next };
        }
        g.pre(n)
    }

    /// `self^e` を `n` 項求める。
    ///
    /// 定数項が 0 でもよい。最低次数を `d` とすると `x^{d e}` の分だけずれる。
    pub fn pow(&self, e: u64, n: usize) -> Self {
        if n == 0 {
            return Self { coef: vec![] };
        }
        if e == 0 {
            // f^0 = 1 (f が零級数でも 1 とする慣習に従う)
            let mut coef = vec![ModInt::raw(0); n];
            coef[0] = ModInt::new(1u32);
            return Self { coef };
        }
        let Some(low) = self.lowest_degree() else {
            // 零級数の正冪は零
            return Self::zeros(n);
        };
        // x^{low * e} が範囲外なら全て 0
        let shift = (low as u128) * (e as u128);
        if shift >= n as u128 {
            return Self::zeros(n);
        }
        let shift = shift as usize;
        let rest = n - shift;

        let c = self.at(low);
        // f = c x^low (1 + ...) と分解し、括弧の中身を exp(e log(...)) で計算する
        let normalized = self.shift_down(low) * c.inv();
        let base = (normalized.log(rest) * ModInt::new(e)).exp(rest);
        (base * c.pow(e)).shift_up(shift).pre(n)
    }

    /// 平方根を `n` 項求める。存在しなければ `None`。
    pub fn sqrt(&self, n: usize) -> Option<Self> {
        if n == 0 {
            return Some(Self { coef: vec![] });
        }
        let Some(low) = self.lowest_degree() else {
            return Some(Self::zeros(n));
        };
        // x^low の low が奇数だと平方根が形式的冪級数にならない
        if low % 2 != 0 {
            return None;
        }
        let shift = low / 2;
        if shift >= n {
            return Some(Self::zeros(n));
        }
        let rest = n - shift;

        let c = self.at(low);
        let root = c.sqrt()?;
        let normalized = self.shift_down(low) * c.inv();
        // 定数項 1 の級数の平方根は exp(log/2)
        let half = ModInt::new(2u32).inv();
        let base = (normalized.log(rest) * half).exp(rest);
        Some((base * root).shift_up(shift).pre(n))
    }

    /// 多項式としての商と余り。`self = q * rhs + r`、`deg r < deg rhs`。
    pub fn div_rem(&self, rhs: &Self) -> (Self, Self) {
        let a = self.clone().shrink();
        let b = rhs.clone().shrink();
        assert!(!b.is_empty(), "division by the zero polynomial");
        if a.len() < b.len() {
            return (Self { coef: vec![] }, a);
        }
        let q_len = a.len() - b.len() + 1;
        // 係数を反転すると商は逆元との畳み込みで求まる
        let ra = a.reversed();
        let rb = b.reversed();
        let q = (ra * rb.inv(q_len)).pre(q_len).reversed();
        let r = (a - q.clone() * b)
            .pre(rhs.clone().shrink().len() - 1)
            .shrink();
        (q.shrink(), r)
    }

    fn reversed(&self) -> Self {
        let mut coef = self.coef.clone();
        coef.reverse();
        Self { coef }
    }

    /// `x = c` での値。
    pub fn eval(&self, x: ModInt<M>) -> ModInt<M> {
        // ホーナー法
        self.coef
            .iter()
            .rev()
            .fold(ModInt::raw(0), |acc, &c| acc * x + c)
    }
}

/// `1..n` の逆元をまとめて `O(n)` で求める。
pub fn inverse_table<const M: u32>(n: usize) -> Vec<ModInt<M>> {
    let mut inv = vec![ModInt::<M>::raw(0); n.max(1)];
    if n > 1 {
        inv[1] = ModInt::new(1u32);
    }
    for i in 2..n {
        // M = q*i + r とすると i^{-1} = -q * r^{-1}
        let q = (M / i as u32) as u64;
        let r = (M % i as u32) as usize;
        inv[i] = -inv[r] * ModInt::new(q);
    }
    inv
}

impl<const M: u32> Index<usize> for Fps<M> {
    type Output = ModInt<M>;
    fn index(&self, i: usize) -> &ModInt<M> {
        &self.coef[i]
    }
}

impl<const M: u32> From<Vec<ModInt<M>>> for Fps<M> {
    fn from(coef: Vec<ModInt<M>>) -> Self {
        Self { coef }
    }
}

impl<const M: u32> Add for Fps<M> {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl<const M: u32> AddAssign for Fps<M> {
    fn add_assign(&mut self, rhs: Self) {
        if self.coef.len() < rhs.coef.len() {
            self.coef.resize(rhs.coef.len(), ModInt::raw(0));
        }
        for (x, y) in self.coef.iter_mut().zip(rhs.coef.iter()) {
            *x += *y;
        }
    }
}

impl<const M: u32> Sub for Fps<M> {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

impl<const M: u32> SubAssign for Fps<M> {
    fn sub_assign(&mut self, rhs: Self) {
        if self.coef.len() < rhs.coef.len() {
            self.coef.resize(rhs.coef.len(), ModInt::raw(0));
        }
        for (x, y) in self.coef.iter_mut().zip(rhs.coef.iter()) {
            *x -= *y;
        }
    }
}

impl<const M: u32> Neg for Fps<M> {
    type Output = Self;
    fn neg(mut self) -> Self {
        for x in self.coef.iter_mut() {
            *x = -*x;
        }
        self
    }
}

/// 畳み込み。打ち切らずに `deg a + deg b` 次まで返す。
impl<const M: u32> Mul for Fps<M> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            coef: convolution(&self.coef, &rhs.coef),
        }
    }
}

impl<const M: u32> Mul<ModInt<M>> for Fps<M> {
    type Output = Self;
    fn mul(mut self, rhs: ModInt<M>) -> Self {
        self *= rhs;
        self
    }
}

impl<const M: u32> MulAssign<ModInt<M>> for Fps<M> {
    fn mul_assign(&mut self, rhs: ModInt<M>) {
        for x in self.coef.iter_mut() {
            *x *= rhs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: u32 = 998_244_353;
    type Mi = ModInt<P>;
    type F = Fps<P>;

    fn xorshift() -> impl FnMut() -> u64 {
        let mut state = 88172645463325252u64;
        move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        }
    }

    /// 定数項が 1 のランダムな級数。
    fn random_unit(n: usize, next: &mut impl FnMut() -> u64) -> F {
        let mut coef: Vec<Mi> = (0..n).map(|_| Mi::new(next())).collect();
        coef[0] = Mi::new(1u32);
        F::new(coef)
    }

    // --- 素朴な参照実装 ---

    fn naive_inv(f: &F, n: usize) -> F {
        let mut g = vec![Mi::raw(0); n];
        let f0_inv = f.at(0).inv();
        g[0] = f0_inv;
        for i in 1..n {
            let mut s = Mi::raw(0);
            for k in 1..=i {
                s += f.at(k) * g[i - k];
            }
            g[i] = -s * f0_inv;
        }
        F::new(g)
    }

    /// g = exp(f) は g' = f' g、つまり n g[n] = sum_{k=1..n} k f[k] g[n-k]。
    fn naive_exp(f: &F, n: usize) -> F {
        let mut g = vec![Mi::raw(0); n];
        if n == 0 {
            return F::new(g);
        }
        g[0] = Mi::new(1u32);
        let inv = inverse_table::<P>(n + 1);
        for i in 1..n {
            let mut s = Mi::raw(0);
            for k in 1..=i {
                s += Mi::new(k as u64) * f.at(k) * g[i - k];
            }
            g[i] = s * inv[i];
        }
        F::new(g)
    }

    /// log f = ∫ f'/f を素朴な逆元経由で計算する。
    fn naive_log(f: &F, n: usize) -> F {
        let d = f.pre(n).derivative();
        let inv = naive_inv(f, n);
        let mut prod = vec![Mi::raw(0); n.saturating_sub(1)];
        for (i, x) in prod.iter_mut().enumerate() {
            let mut s = Mi::raw(0);
            for k in 0..=i {
                s += d.at(k) * inv.at(i - k);
            }
            *x = s;
        }
        F::new(prod).integral().pre(n)
    }

    // --- テスト ---

    #[test]
    fn add_sub_mul_basic() {
        let a = F::from_ints(&[1i64, 2, 3]);
        let b = F::from_ints(&[4i64, 5]);
        assert_eq!(
            (a.clone() + b.clone()).coef(),
            F::from_ints(&[5i64, 7, 3]).coef()
        );
        assert_eq!(
            (a.clone() - b.clone()).coef(),
            F::from_ints(&[-3i64, -3, 3]).coef()
        );
        // (1 + 2x + 3x^2)(4 + 5x) = 4 + 13x + 22x^2 + 15x^3
        assert_eq!((a * b).coef(), F::from_ints(&[4i64, 13, 22, 15]).coef());
    }

    #[test]
    fn derivative_and_integral() {
        let f = F::from_ints(&[3i64, 1, 4, 1, 5]);
        // d/dx: 1 + 8x + 3x^2 + 20x^3
        assert_eq!(
            f.derivative().coef(),
            F::from_ints(&[1i64, 8, 3, 20]).coef()
        );
        // ∫ してから微分すると戻る
        assert_eq!(f.integral().derivative().coef(), f.coef());
        // 微分してから積分すると定数項が落ちる
        let back = f.derivative().integral();
        assert_eq!(back.at(0).val(), 0);
        for i in 1..f.len() {
            assert_eq!(back.at(i), f.at(i));
        }
    }

    #[test]
    fn inverse_table_is_correct() {
        let inv = inverse_table::<P>(1000);
        for (i, &x) in inv.iter().enumerate().skip(1) {
            assert_eq!(Mi::new(i as u64) * x, Mi::new(1u32), "i = {i}");
        }
    }

    #[test]
    fn inv_matches_naive_and_identity() {
        let mut next = xorshift();
        for n in [1usize, 2, 5, 16, 17, 100] {
            let mut coef: Vec<Mi> = (0..n.max(1)).map(|_| Mi::new(next())).collect();
            coef[0] = Mi::new(next() % 1000 + 1);
            let f = F::new(coef);
            let g = f.inv(n);
            assert_eq!(g.coef(), naive_inv(&f, n).coef(), "n = {n}");
            // f * g == 1 mod x^n
            let prod = (f.clone() * g).pre(n);
            assert_eq!(prod.at(0), Mi::new(1u32));
            for i in 1..n {
                assert_eq!(prod.at(i).val(), 0, "n = {n}, i = {i}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "non-zero constant term")]
    fn inv_of_zero_constant_panics() {
        F::from_ints(&[0i64, 1]).inv(4);
    }

    #[test]
    fn log_matches_naive() {
        let mut next = xorshift();
        for n in [1usize, 2, 5, 16, 33, 100] {
            let f = random_unit(n.max(1), &mut next);
            assert_eq!(f.log(n).coef(), naive_log(&f, n).coef(), "n = {n}");
        }
    }

    #[test]
    fn exp_matches_naive() {
        let mut next = xorshift();
        for n in [1usize, 2, 5, 16, 33, 100] {
            let mut coef: Vec<Mi> = (0..n.max(1)).map(|_| Mi::new(next())).collect();
            coef[0] = Mi::raw(0);
            let f = F::new(coef);
            assert_eq!(f.exp(n).coef(), naive_exp(&f, n).coef(), "n = {n}");
        }
    }

    #[test]
    fn exp_of_x_is_the_exponential_series() {
        // exp(x) = sum x^k / k!
        let n = 20;
        let f = F::from_ints(&[0i64, 1]);
        let got = f.exp(n);
        let mut fact = Mi::new(1u32);
        for k in 0..n {
            if k > 0 {
                fact *= Mi::new(k as u64);
            }
            assert_eq!(got.at(k), fact.inv(), "k = {k}");
        }
    }

    #[test]
    fn exp_and_log_are_inverse() {
        let mut next = xorshift();
        let n = 64;
        let f = random_unit(n, &mut next);
        assert_eq!(f.log(n).exp(n).coef(), f.pre(n).coef());

        let mut coef: Vec<Mi> = (0..n).map(|_| Mi::new(next())).collect();
        coef[0] = Mi::raw(0);
        let g = F::new(coef);
        assert_eq!(g.exp(n).log(n).coef(), g.pre(n).coef());
    }

    #[test]
    fn pow_matches_repeated_multiplication() {
        let mut next = xorshift();
        let n = 50;
        for e in 0..6u64 {
            let f = F::new((0..8).map(|_| Mi::new(next())).collect());
            let mut expected = F::from_ints(&[1i64]);
            for _ in 0..e {
                expected = (expected * f.clone()).pre(n);
            }
            assert_eq!(f.pow(e, n).coef(), expected.pre(n).coef(), "e = {e}");
        }
    }

    /// 定数項が 0 の場合もずらして計算できる。
    #[test]
    fn pow_with_zero_constant_term() {
        // (x^2 + x^3)^3 = x^6 (1 + x)^3 = x^6 + 3x^7 + 3x^8 + x^9
        let f = F::from_ints(&[0i64, 0, 1, 1]);
        let got = f.pow(3, 12);
        let expected = F::from_ints(&[0i64, 0, 0, 0, 0, 0, 1, 3, 3, 1, 0, 0]);
        assert_eq!(got.coef(), expected.coef());

        // ずらしが範囲外なら全て 0
        assert!(f.pow(3, 5).coef().iter().all(|x| x.val() == 0));
        // 零級数
        assert!(F::zeros(4).pow(2, 4).coef().iter().all(|x| x.val() == 0));
        // e = 0 は 1
        assert_eq!(
            F::zeros(4).pow(0, 3).coef(),
            F::from_ints(&[1i64, 0, 0]).coef()
        );
    }

    /// 定数項が 1 でない場合も c^e 倍で処理できる。
    #[test]
    fn pow_with_non_unit_constant() {
        let f = F::from_ints(&[3i64, 1]);
        // (3 + x)^2 = 9 + 6x + x^2
        assert_eq!(f.pow(2, 3).coef(), F::from_ints(&[9i64, 6, 1]).coef());
        // 大きな指数でも動く
        let n = 10;
        let got = f.pow(1_000_000_000, n);
        let expected = {
            let mut acc = F::from_ints(&[1i64]);
            let mut base = f.clone();
            let mut e = 1_000_000_000u64;
            while e > 0 {
                if e & 1 == 1 {
                    acc = (acc * base.clone()).pre(n);
                }
                base = (base.clone() * base).pre(n);
                e >>= 1;
            }
            acc
        };
        assert_eq!(got.coef(), expected.coef());
    }

    #[test]
    fn sqrt_squares_back() {
        let mut next = xorshift();
        let n = 32;
        // 定数項が平方剰余になるまで引き直す
        let mut tries = 0;
        let mut checked = 0;
        while checked < 5 && tries < 100 {
            tries += 1;
            let f = F::new((0..n).map(|_| Mi::new(next())).collect());
            if let Some(s) = f.sqrt(n) {
                assert_eq!((s.clone() * s).pre(n).coef(), f.pre(n).coef());
                checked += 1;
            }
        }
        assert!(checked >= 5, "平方根が取れる例が少なすぎる");
    }

    #[test]
    fn sqrt_edge_cases() {
        // 1 + x の平方根は二項級数
        let f = F::from_ints(&[1i64, 1]);
        let s = f.sqrt(8).unwrap();
        assert_eq!((s.clone() * s).pre(8).coef(), f.pre(8).coef());

        // x^2 (1 + x) は low が偶数なので取れる
        let g = F::from_ints(&[0i64, 0, 1, 1]);
        let s = g.sqrt(8).unwrap();
        assert_eq!((s.clone() * s).pre(8).coef(), g.pre(8).coef());

        // low が奇数なら None
        assert!(F::from_ints(&[0i64, 1]).sqrt(4).is_none());
        // 定数項が平方非剰余なら None
        assert!(Mi::new(3u32).sqrt().is_none());
        assert!(F::from_ints(&[3i64, 1]).sqrt(4).is_none());
        // 零級数
        assert!(
            F::zeros(4)
                .sqrt(4)
                .unwrap()
                .coef()
                .iter()
                .all(|x| x.val() == 0)
        );
    }

    #[test]
    fn div_rem_reconstructs() {
        let mut next = xorshift();
        for (n, m) in [(10usize, 3usize), (5, 5), (3, 7), (100, 30)] {
            let a = F::new((0..n).map(|_| Mi::new(next() % 100)).collect()).shrink();
            let mut b: Vec<Mi> = (0..m).map(|_| Mi::new(next() % 100)).collect();
            // 最高次を非零にする
            b[m - 1] = Mi::new(next() % 99 + 1);
            let b = F::new(b);
            let (q, r) = a.div_rem(&b);
            let recon = (q * b.clone() + r.clone()).shrink();
            assert_eq!(recon.coef(), a.clone().shrink().coef(), "n={n} m={m}");
            assert!(r.shrink().len() < b.shrink().len(), "余りの次数が大きい");
        }
    }

    #[test]
    fn eval_matches_direct() {
        let f = F::from_ints(&[1i64, 2, 3]);
        let x = Mi::new(5u32);
        assert_eq!(f.eval(x), Mi::new(1 + 2 * 5 + 3 * 25u32));
    }

    /// NTT 経路を通る大きさで恒等式を確認する。
    #[test]
    fn large_identities() {
        let mut next = xorshift();
        let n = 1000;
        let f = random_unit(n, &mut next);
        // exp(log f) == f
        assert_eq!(f.log(n).exp(n).coef(), f.pre(n).coef());
        // f * f^{-1} == 1
        let prod = (f.clone() * f.inv(n)).pre(n);
        assert_eq!(prod.at(0), Mi::new(1u32));
        assert!(prod.coef()[1..].iter().all(|x| x.val() == 0));
        // (f^2)^{1/2} == f (定数項 1 なので符号は f 側)
        let sq = (f.clone() * f.clone()).pre(n);
        assert_eq!(sq.sqrt(n).unwrap().coef(), f.pre(n).coef());
    }
}
