use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::iter::Product;

use modint::ModInt;

use crate::Fps;

// 次数が偏った入力でも、大きな式を小さな式の個数分だけ掛け直さない。
fn merge_smallest<T>(
    items: impl IntoIterator<Item = T>,
    size: impl Fn(&T) -> usize,
    merge: impl Fn(T, T) -> T,
) -> Option<T> {
    let mut items: Vec<_> = items.into_iter().map(Some).collect();
    let mut heap: BinaryHeap<_> = items
        .iter()
        .enumerate()
        .map(|(i, item)| Reverse((size(item.as_ref().unwrap()), i)))
        .collect();
    while heap.len() > 1 {
        let Reverse((_, i)) = heap.pop().unwrap();
        let Reverse((_, j)) = heap.pop().unwrap();
        let merged = merge(items[i].take().unwrap(), items[j].take().unwrap());
        heap.push(Reverse((size(&merged), items.len())));
        items.push(Some(merged));
    }
    heap.pop().and_then(|Reverse((_, i))| items[i].take())
}

/// 多項式の総積。小さい式から畳み込む。空積は 1。
/// 総項数を `s`、多項式の個数を `k` として `O(s log(s+1) log(k+1))`。
///
/// ```
/// use fps::Fps;
/// type F = Fps<998244353>;
/// let f: F = [2, 3, 5].into_iter()
///     .map(|a| F::from_ints(&[1, a])).product();
/// assert_eq!(f, F::from_ints(&[1, 10, 31, 30]));
/// ```
impl<const M: u32> Product for Fps<M> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        merge_smallest(iter, Self::len, |a, b| a * b)
            .unwrap_or_else(|| Self::constant(ModInt::new(1u32)))
    }
}

/// 有理式 `p/q` の和を分子・分母の組で返す。約分はしない。
/// 分母は零多項式でないこと。空和は `(0, 1)`。
/// 総項数を `s`、式の個数を `k` として `O(s log(s+1) log(k+1))`。
pub fn sum_rationals<const M: u32>(
    terms: impl IntoIterator<Item = (Fps<M>, Fps<M>)>,
) -> (Fps<M>, Fps<M>) {
    let terms = terms.into_iter().inspect(|(_, q)| {
        assert!(q.lowest_degree().is_some(), "zero polynomial denominator");
    });
    merge_smallest(
        terms,
        |(p, q)| p.len().max(q.len()),
        |(p, q), (r, s)| (p * s.clone() + r * q.clone(), q * s),
    )
    .unwrap_or_else(|| (Fps::zeros(0), Fps::constant(ModInt::new(1u32))))
}

/// `p[k] = sum(values[i]^k)` を `k = 0..n` について返す (`0^0 = 1`)。
/// `O(m log^2(m+1) + n log(n+1))`、`m = values.len()`。
pub fn power_sums<const M: u32>(values: &[ModInt<M>], n: usize) -> Fps<M> {
    if n == 0 {
        return Fps::zeros(0);
    }
    let mut result = Fps::zeros(n);
    result[0] = ModInt::new(values.len());
    if n == 1 || values.is_empty() {
        return result;
    }
    // q = prod(1 - a_i x) とすると -x q'/q = sum_{k>=1} p[k] x^k。
    let q: Fps<M> = values
        .iter()
        .map(|&a| Fps::new(vec![ModInt::new(1u32), -a]))
        .product();
    let q = q.pre(n);
    let logarithmic_derivative = q.derivative().div_series(&q, n - 1);
    for k in 1..n {
        result[k] = -logarithmic_derivative[k - 1];
    }
    result
}
