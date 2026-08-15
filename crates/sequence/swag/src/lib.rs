use algebra::Monoid;

/// SWAG (Sliding Window Aggregation)。
///
/// モノイド `M` の要素をキューとして保持し、末尾への追加・先頭からの削除と
/// 全体の積を計算する。各操作は償却 `O(1)`。
/// 逆元を必要としないので、`Min` や行列積のように差が取れない演算にも使える。
///
/// 2つのスタックで実装する。`front` は先頭側の要素を、それ以降との積を
/// 添えて保持し、`back` は末尾側の要素を積とともに保持する。
pub struct Swag<M: Monoid> {
    /// (値, その値から front の底までの積)。末尾がキューの先頭。
    front: Vec<(M::T, M::T)>,
    /// 末尾側の要素。先頭がキューの中央寄り。
    back: Vec<M::T>,
    /// `back` 全体の積。
    back_prod: M::T,
    monoid: M,
}

impl<M: Monoid> Swag<M> {
    pub fn new(monoid: M) -> Self {
        let back_prod = monoid.identity();
        Self {
            front: vec![],
            back: vec![],
            back_prod,
            monoid,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.front.len() + self.back.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn monoid(&self) -> &M {
        &self.monoid
    }

    /// 末尾に追加する。`O(1)`。
    pub fn push(&mut self, value: M::T) {
        self.back_prod = self.monoid.binary_op(&self.back_prod, &value);
        self.back.push(value);
    }

    /// 先頭を取り出す。空なら `None`。償却 `O(1)`。
    pub fn pop(&mut self) -> Option<M::T> {
        if self.front.is_empty() {
            self.move_back_to_front();
        }
        self.front.pop().map(|(value, _)| value)
    }

    /// 先頭の要素を見る。
    #[inline]
    pub fn front(&self) -> Option<&M::T> {
        self.front
            .last()
            .map(|(value, _)| value)
            .or_else(|| self.back.first())
    }

    /// キュー全体の積。空なら単位元。`O(1)`。
    pub fn prod(&self) -> M::T {
        match self.front.last() {
            Some((_, cum)) => self.monoid.binary_op(cum, &self.back_prod),
            None => self.back_prod.clone(),
        }
    }

    pub fn clear(&mut self) {
        self.front.clear();
        self.back.clear();
        self.back_prod = self.monoid.identity();
    }

    /// `back` を逆順に `front` へ移し、累積積を張り直す。
    /// `front` が空のときのみ呼ぶこと。各要素は高々1回しか移動しないので
    /// ならし計算量は `O(1)`。
    fn move_back_to_front(&mut self) {
        debug_assert!(self.front.is_empty());
        while let Some(value) = self.back.pop() {
            let cum = match self.front.last() {
                Some((_, prev)) => self.monoid.binary_op(&value, prev),
                None => value.clone(),
            };
            self.front.push((value, cum));
        }
        self.back_prod = self.monoid.identity();
    }
}

/// 幅 `k` の全ての窓の積を返す。返り値の長さは `n - k + 1`。
/// `k == 0` または `k > n` なら空。
pub fn window_prods<M: Monoid>(values: &[M::T], k: usize, monoid: M) -> Vec<M::T> {
    if k == 0 || k > values.len() {
        return vec![];
    }
    let mut swag = Swag::new(monoid);
    let mut ret = Vec::with_capacity(values.len() - k + 1);
    for value in values {
        swag.push(value.clone());
        if swag.len() == k {
            ret.push(swag.prod());
            swag.pop();
        }
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use algebra::{FnMonoid, Min, Sum};

    #[test]
    fn push_pop_prod() {
        let mut swag = Swag::new(Sum::<i64>::new());
        assert!(swag.is_empty());
        assert_eq!(swag.prod(), 0);
        assert_eq!(swag.pop(), None);

        for x in 1..=4 {
            swag.push(x);
        }
        assert_eq!(swag.len(), 4);
        assert_eq!(swag.front(), Some(&1));
        assert_eq!(swag.prod(), 10);

        assert_eq!(swag.pop(), Some(1));
        assert_eq!(swag.prod(), 9);
        // pop で front が空になった後に push しても順序は保たれる
        swag.push(5);
        assert_eq!(swag.prod(), 14);
        assert_eq!(swag.pop(), Some(2));
        assert_eq!(swag.pop(), Some(3));
        assert_eq!(swag.pop(), Some(4));
        assert_eq!(swag.pop(), Some(5));
        assert!(swag.is_empty());
        assert_eq!(swag.prod(), 0);
    }

    #[test]
    fn non_commutative_keeps_order() {
        // 文字列結合は非可換なので順序の誤りを検出できる
        let mut swag = Swag::new(FnMonoid::new(String::new(), |a: &String, b: &String| {
            format!("{a}{b}")
        }));
        for c in ["a", "b", "c", "d"] {
            swag.push(c.to_string());
        }
        assert_eq!(swag.prod(), "abcd");
        swag.pop();
        assert_eq!(swag.prod(), "bcd");
        swag.push("e".to_string());
        assert_eq!(swag.prod(), "bcde");
        swag.pop();
        assert_eq!(swag.prod(), "cde");
    }

    #[test]
    fn sliding_window_min() {
        let a = [3i64, 1, 4, 1, 5, 9, 2, 6];
        for k in 1..=a.len() {
            let expected: Vec<i64> = a.windows(k).map(|w| *w.iter().min().unwrap()).collect();
            assert_eq!(window_prods(&a, k, Min::<i64>::new()), expected, "k = {k}");
        }
        assert_eq!(window_prods(&a, 0, Min::<i64>::new()), Vec::<i64>::new());
        assert_eq!(
            window_prods(&a, a.len() + 1, Min::<i64>::new()),
            Vec::<i64>::new()
        );
    }

    /// push/pop をランダムに混ぜて素朴な実装と比較する。
    #[test]
    fn matches_naive_on_random_ops() {
        let mut state = 88172645463325252u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut swag = Swag::new(Min::<i64>::new());
        let mut naive: std::collections::VecDeque<i64> = Default::default();
        for _ in 0..2000 {
            if next() % 3 == 0 {
                assert_eq!(swag.pop(), naive.pop_front());
            } else {
                let v = (next() % 100) as i64;
                swag.push(v);
                naive.push_back(v);
            }
            assert_eq!(swag.len(), naive.len());
            let expected = naive.iter().copied().min().unwrap_or(i64::MAX);
            assert_eq!(swag.prod(), expected);
            assert_eq!(swag.front(), naive.front());
        }
    }
}
