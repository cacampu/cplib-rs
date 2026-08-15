use std::ops::{Bound, RangeBounds};

use algebra::Group;

/// 群 `G` 上の累積和。構築 `O(n)`、区間積 `O(1)`。
///
/// 差を取るため `Group` を要求する。逆元がない場合は SegTree を使う。
pub struct PrefixSum<G: Group> {
    /// `cum[i]` = 先頭 `i` 個の積。長さは `n + 1`。
    cum: Vec<G::T>,
    group: G,
}

impl<G: Group> PrefixSum<G> {
    pub fn new<I>(values: I, group: G) -> Self
    where
        I: IntoIterator<Item = G::T>,
    {
        let iter = values.into_iter();
        let mut cum = Vec::with_capacity(iter.size_hint().0 + 1);
        let mut acc = group.identity();
        cum.push(acc.clone());
        for v in iter {
            acc = group.binary_op(&acc, &v);
            cum.push(acc.clone());
        }
        Self { cum, group }
    }

    /// 元の列の長さ。
    #[inline]
    pub fn len(&self) -> usize {
        self.cum.len() - 1
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn group(&self) -> &G {
        &self.group
    }

    /// 累積和の配列そのもの。`cum()[i]` は先頭 `i` 個の積。
    #[inline]
    pub fn cum(&self) -> &[G::T] {
        &self.cum
    }

    /// 区間 `range` の積を返す。空区間(`l >= r`)なら単位元を返す。
    /// 空でない区間が `n` を超える場合のみパニックする。
    pub fn prod<R: RangeBounds<usize>>(&self, range: R) -> G::T {
        let (l, r) = self.resolve(range);
        if l >= r {
            return self.group.identity();
        }
        let n = self.len();
        assert!(r <= n, "range out of bounds: [{l}, {r}) of {n}");
        self.group.inv_binary_op(&self.cum[r], &self.cum[l])
    }

    /// 全区間の積。
    #[inline]
    pub fn all_prod(&self) -> G::T {
        self.cum[self.len()].clone()
    }

    /// `RangeBounds` を半開区間 `[l, r)` に正規化する。範囲の検査はしない。
    fn resolve<R: RangeBounds<usize>>(&self, range: R) -> (usize, usize) {
        let l = match range.start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let r = match range.end_bound() {
            Bound::Included(&i) => i.saturating_add(1),
            Bound::Excluded(&i) => i,
            Bound::Unbounded => self.len(),
        };
        (l, r)
    }
}

impl<G: Group + Default> PrefixSum<G> {
    /// 単位元・演算が型から決まる群(`Sum` など)用のショートカット。
    pub fn from_slice(values: &[G::T]) -> Self {
        Self::new(values.iter().cloned(), G::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algebra::{FnGroup, Sum, Xor};

    #[test]
    fn prod_matches_naive() {
        let a = [1i64, 2, 3, 4, 5];
        let ps = PrefixSum::<Sum<i64>>::from_slice(&a);
        assert_eq!(ps.len(), 5);
        assert_eq!(ps.cum(), [0, 1, 3, 6, 10, 15]);
        for l in 0..=a.len() {
            for r in l..=a.len() {
                assert_eq!(ps.prod(l..r), a[l..r].iter().sum::<i64>(), "{l}..{r}");
            }
        }
        assert_eq!(ps.all_prod(), 15);
        assert_eq!(ps.prod(..), 15);
        assert_eq!(ps.prod(1..=2), 5);
    }

    /// SegTree と同じ範囲の扱い。n = 100。
    #[test]
    fn range_semantics() {
        let ps = PrefixSum::new(0..100i64, Sum::<i64>::new());
        assert_eq!(ps.prod(50..), ps.prod(50..100));
        assert_eq!(ps.prod(50..), (50..100).sum::<i64>());
        let (l, r) = (50, 10);
        assert_eq!(ps.prod(l..r), 0);
        assert_eq!(ps.prod(100..100), 0);
    }

    #[test]
    #[should_panic]
    fn range_over_n_panics() {
        PrefixSum::new(0..100i64, Sum::<i64>::new()).prod(50..200);
    }

    /// `Group + Default` を満たすので from_slice が使える。累積 xor。
    #[test]
    fn prefix_xor() {
        let a = [0b1100u64, 0b1010, 0b0110, 0b0001];
        let ps = PrefixSum::<Xor<u64>>::from_slice(&a);
        for l in 0..=a.len() {
            for r in l..=a.len() {
                let expected = a[l..r].iter().fold(0, |acc, x| acc ^ x);
                assert_eq!(ps.prod(l..r), expected, "{l}..{r}");
            }
        }
    }

    #[test]
    fn unsigned_and_empty() {
        let ps = PrefixSum::<Sum<usize>>::from_slice(&[3, 1, 4, 1, 5]);
        assert_eq!(ps.prod(1..4), 6);

        let empty = PrefixSum::<Sum<i64>>::from_slice(&[]);
        assert!(empty.is_empty());
        assert_eq!(empty.prod(..), 0);
        assert_eq!(empty.all_prod(), 0);
    }

    /// 群側に法を持たせる。
    #[test]
    fn group_can_carry_constants() {
        let modulo = 1_000_000_007i64;
        let ps = PrefixSum::new(
            [modulo - 1, 5, 7],
            FnGroup::new(
                0,
                move |a: &i64, b: &i64| (a + b) % modulo,
                move |a: &i64, b: &i64| (a - b).rem_euclid(modulo),
            ),
        );
        assert_eq!(ps.prod(0..2), 4);
        assert_eq!(ps.all_prod(), 11);
    }
}
