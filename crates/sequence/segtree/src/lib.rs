use std::ops::{Bound, RangeBounds};

use algebra::Monoid;

/// モノイド `M` 上のセグメント木。
/// 一点更新 `O(1) + O(log n)`、区間積 `O(log n)`。
///
/// `M` はインスタンスとして保持されるので、mod の法や事前計算テーブルを
/// モノイド側に持たせて共有できる。
pub struct SegTree<M: Monoid> {
    n: usize,
    cap: usize,
    data: Vec<M::T>,
    monoid: M,
}

impl<M: Monoid> SegTree<M> {
    /// 長さ `n` の列を単位元で初期化する。
    pub fn new(n: usize, monoid: M) -> Self {
        let cap = n.next_power_of_two();
        let data = vec![monoid.identity(); cap * 2];
        Self {
            n,
            cap,
            data,
            monoid,
        }
    }

    /// 既存の列から `O(n)` で構築する。
    pub fn from_vec(values: Vec<M::T>, monoid: M) -> Self {
        let mut tree = Self::new(values.len(), monoid);
        for (i, v) in values.into_iter().enumerate() {
            tree.data[tree.cap + i] = v;
        }
        for pt in (1..tree.cap).rev() {
            tree.pull(pt);
        }
        tree
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.n
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    #[inline]
    pub fn monoid(&self) -> &M {
        &self.monoid
    }

    #[inline]
    pub fn get(&self, i: usize) -> &M::T {
        assert!(i < self.n, "index out of range: {i} >= {}", self.n);
        &self.data[self.cap + i]
    }

    pub fn set(&mut self, i: usize, value: M::T) {
        assert!(i < self.n, "index out of range: {i} >= {}", self.n);
        let mut pt = self.cap + i;
        self.data[pt] = value;
        while pt > 1 {
            pt >>= 1;
            self.pull(pt);
        }
    }

    /// `i` 番目の要素を現在の値から計算し直す。
    pub fn update(&mut self, i: usize, f: impl FnOnce(&M::T) -> M::T) {
        let value = f(self.get(i));
        self.set(i, value);
    }

    /// 区間 `range` の積を返す。空区間(`l >= r`)なら単位元を返す。
    /// 空でない区間が `n` を超える場合のみパニックする。
    pub fn prod<R: RangeBounds<usize>>(&self, range: R) -> M::T {
        let (l, r) = self.resolve(range);
        if l >= r {
            return self.monoid.identity();
        }
        assert!(r <= self.n, "range out of bounds: [{l}, {r}) of {}", self.n);
        let mut l = l + self.cap;
        let mut r = r + self.cap;
        let mut prod_l = self.monoid.identity();
        let mut prod_r = self.monoid.identity();
        while l < r {
            if l & 1 == 1 {
                prod_l = self.monoid.binary_op(&prod_l, &self.data[l]);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                prod_r = self.monoid.binary_op(&self.data[r], &prod_r);
            }
            l >>= 1;
            r >>= 1;
        }
        self.monoid.binary_op(&prod_l, &prod_r)
    }

    /// 全区間の積を `O(1)` で返す。
    #[inline]
    pub fn all_prod(&self) -> M::T {
        self.data[1].clone()
    }

    #[inline]
    fn pull(&mut self, pt: usize) {
        self.data[pt] = self
            .monoid
            .binary_op(&self.data[pt << 1], &self.data[(pt << 1) + 1]);
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
            Bound::Unbounded => self.n,
        };
        (l, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algebra::{FnMonoid, Min, Sum};

    #[test]
    fn min_prod() {
        let a = vec![5, 3, 7, 1, 9, 2];
        let tree = SegTree::from_vec(a.clone(), Min::<i64>::new());
        assert_eq!(tree.len(), a.len());
        for l in 0..=a.len() {
            for r in l..=a.len() {
                let expected = a[l..r].iter().copied().min().unwrap_or(i64::MAX);
                assert_eq!(tree.prod(l..r), expected, "range {l}..{r}");
            }
        }
        assert_eq!(tree.all_prod(), 1);
        assert_eq!(tree.prod(..), 1);
        assert_eq!(tree.prod(1..=2), 3);
    }

    #[test]
    fn set_and_update() {
        let mut tree = SegTree::from_vec(vec![1, 2, 3, 4], Sum::<i64>::new());
        assert_eq!(tree.all_prod(), 10);
        tree.set(0, 10);
        assert_eq!(*tree.get(0), 10);
        assert_eq!(tree.prod(0..2), 12);
        tree.update(3, |x| x * 2);
        assert_eq!(tree.all_prod(), 23);
    }

    #[test]
    fn new_is_filled_with_identity() {
        let tree = SegTree::new(5, Sum::<i64>::new());
        assert_eq!(tree.all_prod(), 0);
        assert_eq!(tree.prod(..), 0);
        assert_eq!(*tree.get(4), 0);
    }

    #[test]
    fn empty_tree() {
        let tree = SegTree::new(0, Sum::<i64>::new());
        assert!(tree.is_empty());
        assert_eq!(tree.prod(..), 0);
        assert_eq!(tree.all_prod(), 0);
    }

    /// モノイド側に定数(法)を持たせられることの確認。
    #[test]
    fn monoid_can_carry_constants() {
        let modulo = 1_000_000_007u64;
        let tree = SegTree::from_vec(
            vec![modulo - 1, 2, 3],
            FnMonoid::new(1, move |a: &u64, b: &u64| a * b % modulo),
        );
        assert_eq!(tree.all_prod(), (modulo - 1) * 6 % modulo);
        assert_eq!(tree.prod(1..3), 6);
    }

    /// 空区間はパニックせず単位元を返す。位置が範囲外でも同様。
    #[test]
    fn empty_range_is_identity() {
        let tree = SegTree::from_vec(vec![1, 2, 3, 4], Sum::<i64>::new());
        // 実際の使用時と同様、境界は計算結果として渡す
        let (l, r) = (3, 1);
        assert_eq!(tree.prod(2..2), 0);
        assert_eq!(tree.prod(l..r), 0);
        assert_eq!(tree.prod(l..=r), 0);
        assert_eq!(tree.prod(100..100), 0);
        assert_eq!(tree.prod(usize::MAX..), 0);
        assert_eq!(tree.prod(..0), 0);
    }

    #[test]
    #[should_panic]
    fn prod_out_of_bounds() {
        let tree = SegTree::new(4, Sum::<i64>::new());
        tree.prod(0..5);
    }

    fn tree_100() -> SegTree<Sum<i64>> {
        SegTree::from_vec((0..100).collect(), Sum::<i64>::new())
    }

    /// n = 100 のときの範囲指定の扱い。
    #[test]
    fn range_semantics() {
        let tree = tree_100();
        // 末尾省略は n までとして扱う
        assert_eq!(tree.prod(50..), tree.prod(50..100));
        assert_eq!(tree.prod(50..), (50..100).sum::<i64>());
        // 逆順は空区間なので単位元
        let (l, r) = (50, 10);
        assert_eq!(tree.prod(l..r), 0);
    }

    /// 空でない区間が n を超える場合は落とす。
    #[test]
    #[should_panic]
    fn range_over_n_panics() {
        tree_100().prod(50..200);
    }
}
