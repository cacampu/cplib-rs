use convolution::convolution;
use modint::ModInt;

use crate::Fps;

/// 有理型母関数 `p(x) / q(x)` の `x^n` の係数 (Bostan–Mori)。
/// `q[0] != 0` が必要。分子の次数が分母以上でもよい。
/// `d = max(p.len(), q.len())` として `O(d log(d+1) log(n+1))`。
///
/// ```
/// use fps::{bostan_mori, Fps};
/// use modint::ModInt998244353 as Mi;
/// // x / (1-x-x^2) は Fibonacci 数列の母関数。
/// let p = Fps::from_ints(&[0, 1]);
/// let q = Fps::from_ints(&[1, -1, -1]);
/// assert_eq!(bostan_mori(&p, &q, 10), Mi::new(55));
/// ```
pub fn bostan_mori<const M: u32>(p: &Fps<M>, q: &Fps<M>, mut n: u64) -> ModInt<M> {
    assert!(
        q.at(0).val() != 0,
        "denominator requires a non-zero constant term"
    );
    let mut p = p.clone().shrink();
    let mut q = q.clone().shrink();
    while n > 0 && !p.is_empty() {
        if q.len() == 1 {
            return if n < p.len() as u64 {
                p[n as usize] / q[0]
            } else {
                ModInt::raw(0)
            };
        }
        // q(x)q(-x) は偶数次のみ。分子は n と同じ偶奇の係数を残す。
        let mut negative = q.clone();
        for i in (1..negative.len()).step_by(2) {
            negative[i] = -negative[i];
        }
        p = Fps::new(
            convolution(p.coef(), negative.coef())
                .into_iter()
                .skip((n & 1) as usize)
                .step_by(2)
                .collect(),
        )
        .shrink();
        q = Fps::new(
            convolution(q.coef(), negative.coef())
                .into_iter()
                .step_by(2)
                .collect(),
        )
        .shrink();
        n >>= 1;
    }
    p.at(0) / q[0]
}

/// 線形漸化式 `a[i] = sum_{j=0..d} coefficients[j] * a[i-1-j]` の第 `n` 項。
/// `initial` は `a[0..d]` で、係数列と同じ長さであること。空の漸化式は零数列。
/// `O(d log(d+1) log(n+1))`。
///
/// ```
/// use fps::linear_recurrence;
/// use modint::ModInt998244353 as Mi;
/// assert_eq!(linear_recurrence(&[0, 1].map(Mi::new), &[1, 1].map(Mi::new), 10), Mi::new(55));
/// ```
pub fn linear_recurrence<const M: u32>(
    initial: &[ModInt<M>],
    coefficients: &[ModInt<M>],
    n: u64,
) -> ModInt<M> {
    assert_eq!(
        initial.len(),
        coefficients.len(),
        "initial and coefficients must have equal lengths"
    );
    if n < initial.len() as u64 {
        return initial[n as usize];
    }
    let mut denominator = vec![ModInt::new(1u32)];
    denominator.extend(coefficients.iter().map(|&c| -c));
    let q = Fps::new(denominator);
    let p = Fps::new(initial.to_vec()).mul_truncated(&q, initial.len());
    bostan_mori(&p, &q, n)
}

/// 与えられた数列の接頭辞に合う最小次数の線形漸化式を `O(s.len()^2)` で復元する。
/// 返り値の係数の順序は `linear_recurrence` と同じ。空列・零数列なら空の係数列。
///
/// 未知の `d` 次漸化式の後続項を復元する用途では、通常 `2d` 項以上が必要。
/// 入力の接頭辞だけから、その後も同じ漸化式に従うことを保証するものではない。
pub fn berlekamp_massey<const M: u32>(s: &[ModInt<M>]) -> Vec<ModInt<M>> {
    let one = ModInt::new(1u32);
    let zero = ModInt::raw(0);
    let mut current = vec![one];
    let mut previous = vec![one];
    let (mut degree, mut shift) = (0, 1);
    let mut last_discrepancy = one;
    for n in 0..s.len() {
        let mut discrepancy = s[n];
        for i in 1..=degree {
            discrepancy += current[i] * s[n - i];
        }
        if discrepancy.val() == 0 {
            shift += 1;
            continue;
        }
        let old = current.clone();
        let scale = discrepancy / last_discrepancy;
        current.resize(current.len().max(previous.len() + shift), zero);
        for (i, &c) in previous.iter().enumerate() {
            current[i + shift] -= scale * c;
        }
        if degree * 2 <= n {
            degree = n + 1 - degree;
            previous = old;
            last_discrepancy = discrepancy;
            shift = 1;
        } else {
            shift += 1;
        }
    }
    current.resize(degree + 1, zero);
    current.into_iter().skip(1).map(|c| -c).collect()
}
