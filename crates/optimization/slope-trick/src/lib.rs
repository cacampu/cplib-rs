//! 区分線形凸関数を管理する slope trick。
//!
//! [`SlopeTrick`] は通常の slope trick で、傾きの変化点 (x 座標) を2つの
//! ヒープで管理する。**傾きの取りうる範囲 (= 変化点の個数) が小さい**場合に
//! 効率的。
//!
//! [`ConvexConjugate`] はその双対にあたる表現で、**定義域が小さい整数の
//! 範囲**であるとみなし、各点での傾き (差分) の値そのものを2つのヒープで
//! 管理する。差分 (傾き) がどれほど大きくても、定義域の点数が小さければ
//! 効率的に扱える。「ちょうど k 個使う」のような小さい添字に対する DP を
//! 高速化する場合に向いている。
//!
//! 参考:
//! - <https://noshi91.hatenablog.com/entry/2023/11/20/052227>
//! - <https://maspypy.com/slope-trick-1-解説編>
//! - <https://maspypy.com/slope-trick-3-slope-trick-の凸共役>

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// 傾きが存在しない側の境界を表す番兵。
const INF: i64 = i64::MAX / 4;

/// 区分線形凸関数 `f: Z -> Z` を管理する (通常の slope trick)。
///
/// `f(x) = min_f + Σ_{l∈L}(l-x)₊ + Σ_{r∈R}(x-r)₊` という形で表現する。
/// `L` は傾きが負またはゼロの区間の変化点の多重集合 (最大値を `l0` とする)、
/// `R` は傾きが正またはゼロの区間の変化点の多重集合 (最小値を `r0` とする)。
/// 最小値を達成する区間はちょうど `[l0, r0]`。
///
/// `N = |L| + |R|` として、平行移動以外の基本操作は `O(log N)`。
/// 平行移動は遅延加算 (`add_l` / `add_r`) により `O(1)` にしている。
#[derive(Clone, Debug)]
pub struct SlopeTrick {
    min_f: i64,
    // 実際の値は `x - add_l`。
    l: BinaryHeap<i64>,
    // 実際の値は `x + add_r`。
    r: BinaryHeap<Reverse<i64>>,
    add_l: i64,
    add_r: i64,
}

impl Default for SlopeTrick {
    fn default() -> Self {
        Self::new()
    }
}

impl SlopeTrick {
    /// 零関数 (`f(x) = 0`)。
    pub fn new() -> Self {
        Self {
            min_f: 0,
            l: BinaryHeap::new(),
            r: BinaryHeap::new(),
            add_l: 0,
            add_r: 0,
        }
    }

    /// 変化点の個数 `|L| + |R|`。
    #[inline]
    pub fn len(&self) -> usize {
        self.l.len() + self.r.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 現在の最小値。`O(1)`。
    #[inline]
    pub fn min(&self) -> i64 {
        self.min_f
    }

    /// 最小値を達成する区間 `[l0, r0]`。変化点がない側は `±INF`。`O(1)`。
    #[inline]
    pub fn argmin(&self) -> (i64, i64) {
        (self.top_l(), self.top_r())
    }

    #[inline]
    fn top_l(&self) -> i64 {
        self.l.peek().map_or(-INF, |&x| x + self.add_l)
    }

    #[inline]
    fn top_r(&self) -> i64 {
        self.r.peek().map_or(INF, |&Reverse(x)| x + self.add_r)
    }

    #[inline]
    fn push_l(&mut self, x: i64) {
        self.l.push(x - self.add_l);
    }

    #[inline]
    fn push_r(&mut self, x: i64) {
        self.r.push(Reverse(x - self.add_r));
    }

    /// `L` の最大値を取り除いて返す。空なら `-INF`。
    fn pop_l(&mut self) -> i64 {
        let v = self.top_l();
        self.l.pop();
        v
    }

    /// `R` の最小値を取り除いて返す。空なら `INF`。
    fn pop_r(&mut self) -> i64 {
        let v = self.top_r();
        self.r.pop();
        v
    }

    fn pop_l_opt(&mut self) -> Option<i64> {
        self.l.pop().map(|x| x + self.add_l)
    }

    fn pop_r_opt(&mut self) -> Option<i64> {
        self.r.pop().map(|Reverse(x)| x + self.add_r)
    }

    /// 定数 `c` を加算する: `f(x) += c`。`O(1)`。
    pub fn add_const(&mut self, c: i64) {
        self.min_f += c;
    }

    /// `(x-a)₊` を加算する。`O(log N)`。
    pub fn add_right(&mut self, a: i64) {
        if self.l.is_empty() || self.top_l() <= a {
            self.push_r(a);
        } else {
            let l0 = self.pop_l();
            self.min_f += l0 - a;
            self.push_l(a);
            self.push_r(l0);
        }
    }

    /// `(a-x)₊` を加算する。`O(log N)`。
    pub fn add_left(&mut self, a: i64) {
        if self.r.is_empty() || a <= self.top_r() {
            self.push_l(a);
        } else {
            let r0 = self.pop_r();
            self.min_f += a - r0;
            self.push_r(a);
            self.push_l(r0);
        }
    }

    /// `|x-a|` を加算する。`O(log N)`。
    pub fn add_abs(&mut self, a: i64) {
        self.add_right(a);
        self.add_left(a);
    }

    /// `g(x) = min_{y<=x} f(y)` に置き換える (右側を定数にする)。`O(1)` (遅延、`R` を捨てるだけ)。
    pub fn clear_right(&mut self) {
        self.r.clear();
    }

    /// `g(x) = min_{y>=x} f(y)` に置き換える (左側を定数にする)。`O(1)`。
    pub fn clear_left(&mut self) {
        self.l.clear();
    }

    /// `g(x) = min_{x-b<=y<=x-a} f(y)` に置き換える (`a <= b`)。スライド最小値。`O(1)`。
    ///
    /// `a == b` なら単純な平行移動 `g(x) = f(x-a)` になる。
    pub fn shift(&mut self, a: i64, b: i64) {
        assert!(a <= b, "shift requires a <= b");
        self.add_l += a;
        self.add_r += b;
    }

    /// 単純な平行移動 `g(x) = f(x-a)`。`O(1)`。
    pub fn translate(&mut self, a: i64) {
        self.shift(a, a);
    }

    /// `x` での値を `O(N)` で評価する (デバッグ・テスト用)。
    pub fn eval(&self, x: i64) -> i64 {
        let mut v = self.min_f;
        for &l in self.l.iter() {
            v += (l + self.add_l - x).max(0);
        }
        for &Reverse(r) in self.r.iter() {
            v += (x - (r + self.add_r)).max(0);
        }
        v
    }

    /// `other` を加算する (`f += other`)。小さい方を大きい方へ挿入するので
    /// 償却 `O(min(N,M) log(N+M))`。
    pub fn merge(&mut self, mut other: Self) {
        if self.len() < other.len() {
            std::mem::swap(self, &mut other);
        }
        self.min_f += other.min_f;
        while let Some(v) = other.pop_l_opt() {
            self.push_l(v);
        }
        while let Some(v) = other.pop_r_opt() {
            self.push_r(v);
        }
        // l > r になっている箇所を潰して正準形に戻す。
        // (l-x)₊ + (x-r)₊ = (l-r) + (r-x)₊ + (x-l)₊ (l > r のとき)
        while self.top_l() > self.top_r() {
            let l = self.pop_l();
            let r = self.pop_r();
            self.min_f += l - r;
            self.push_l(r);
            self.push_r(l);
        }
    }
}

/// `SlopeTrick` の凸共役にあたる構造体。
///
/// `f: Z -> Z` を「`argmin` での値 `min_f`」と「`argmin` から見た差分の
/// 多重集合」として管理する。差分は符号で2つのヒープに分ける:
/// - `neg`: `argmin` より左側の傾き (差分、`<= 0`) の多重集合。最大値 (`argmin`
///   に最も近いもの) を管理する max-heap。
/// - `pos`: `argmin` より右側の傾き (差分、`> 0`) の多重集合。最小値 (`argmin`
///   に最も近いもの) を管理する min-heap。
///
/// `N = |neg| + |pos|` (= 定義域の点数 - 1) が小さい場合に効率的。
/// 差分の値そのものは任意の大きさでよい。
#[derive(Clone, Debug, Default)]
pub struct ConvexConjugate {
    min_f: i64,
    neg: BinaryHeap<i64>,
    pos: BinaryHeap<Reverse<i64>>,
}

impl ConvexConjugate {
    /// 単一点上の零関数 (`f(argmin) = 0`、定義域は1点のみ)。
    pub fn new() -> Self {
        Self {
            min_f: 0,
            neg: BinaryHeap::new(),
            pos: BinaryHeap::new(),
        }
    }

    /// 現在の定義域の点数。
    #[inline]
    pub fn len(&self) -> usize {
        self.neg.len() + self.pos.len() + 1
    }

    /// 定義域は常に1点以上あるので常に `false`。
    #[inline]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// 現在の最小値。`O(1)`。
    #[inline]
    pub fn min(&self) -> i64 {
        self.min_f
    }

    /// 定数 `c` を加算する。`O(1)`。
    pub fn add_const(&mut self, c: i64) {
        self.min_f += c;
    }

    /// 差分 `d` を1つ挿入する (`g(x) = min(f(x), f(x-1)+d)` との合成)。定義域が
    /// 1点増える。`O(log N)`。
    ///
    /// ```
    /// use slope_trick::ConvexConjugate;
    /// let mut cc = ConvexConjugate::new();
    /// cc.insert(5);
    /// cc.insert(-3);
    /// assert_eq!(cc.min(), -3);
    /// ```
    pub fn insert(&mut self, d: i64) {
        self.min_f += d.min(0);
        if d <= 0 {
            self.neg.push(d);
        } else {
            self.pos.push(Reverse(d));
        }
    }

    /// `other` との min-plus 畳み込みを行う (2つの凸関数を合成する)。
    /// 差分の多重集合をそのまま合併すればよい。`O((|other|) log(N))` 程度。
    pub fn convolve(&mut self, mut other: Self) {
        self.min_f += other.min_f;
        self.neg.extend(other.neg.drain());
        self.pos.extend(other.pos.drain());
    }

    /// `argmin` から相対位置 `k` (負も可) での値を `O(N log N)` で評価する。
    pub fn eval_offset(&self, k: i64) -> i64 {
        if k == 0 {
            return self.min_f;
        }
        if k > 0 {
            // into_sorted_vec は Reverse の昇順 = 元の値の降順なので反転する。
            let mut sorted = self.pos.clone().into_sorted_vec();
            sorted.reverse();
            let k = k as usize;
            assert!(k <= sorted.len(), "offset out of range");
            self.min_f + sorted[..k].iter().map(|&Reverse(v)| v).sum::<i64>()
        } else {
            // argmin に近い (= 0 に近い) 順に足すため降順に取る。
            let mut sorted = self.neg.clone().into_sorted_vec();
            sorted.reverse();
            let k = (-k) as usize;
            assert!(k <= sorted.len(), "offset out of range");
            self.min_f - sorted[..k].iter().sum::<i64>()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xorshift() -> impl FnMut() -> u64 {
        let mut state = 88172645463325252u64;
        move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        }
    }

    /// 愚直な区間 `[-LIM, LIM]` 上の配列で `SlopeTrick` と同じ操作を行う参照実装。
    struct Naive {
        lo: i64,
        f: Vec<i64>,
    }

    const LIM: i64 = 60;
    const NAIVE_INF: i64 = 1 << 40;

    impl Naive {
        fn new() -> Self {
            Self {
                lo: -LIM,
                f: vec![0; (2 * LIM + 1) as usize],
            }
        }

        fn idx(&self, x: i64) -> usize {
            (x - self.lo) as usize
        }

        fn add_const(&mut self, c: i64) {
            for v in self.f.iter_mut() {
                if *v < NAIVE_INF {
                    *v += c;
                }
            }
        }

        fn add_right(&mut self, a: i64) {
            for x in -LIM..=LIM {
                let add = (x - a).max(0);
                let i = self.idx(x);
                if self.f[i] < NAIVE_INF {
                    self.f[i] += add;
                }
            }
        }

        fn add_left(&mut self, a: i64) {
            for x in -LIM..=LIM {
                let add = (a - x).max(0);
                let i = self.idx(x);
                if self.f[i] < NAIVE_INF {
                    self.f[i] += add;
                }
            }
        }

        fn clear_right(&mut self) {
            let mut best = NAIVE_INF;
            for x in -LIM..=LIM {
                let i = self.idx(x);
                best = best.min(self.f[i]);
                self.f[i] = best;
            }
        }

        fn clear_left(&mut self) {
            let mut best = NAIVE_INF;
            for x in (-LIM..=LIM).rev() {
                let i = self.idx(x);
                best = best.min(self.f[i]);
                self.f[i] = best;
            }
        }

        fn min(&self) -> i64 {
            *self.f.iter().min().unwrap()
        }

        fn eval(&self, x: i64) -> i64 {
            self.f[self.idx(x)]
        }
    }

    #[test]
    fn add_right_left_abs_matches_naive() {
        let mut next = xorshift();
        let mut st = SlopeTrick::new();
        let mut naive = Naive::new();
        for _ in 0..200 {
            let a = (next() % (2 * LIM as u64 + 1)) as i64 - LIM;
            match next() % 4 {
                0 => {
                    st.add_right(a);
                    naive.add_right(a);
                }
                1 => {
                    st.add_left(a);
                    naive.add_left(a);
                }
                2 => {
                    st.add_abs(a);
                    naive.add_right(a);
                    naive.add_left(a);
                }
                _ => {
                    let c = a % 10;
                    st.add_const(c);
                    naive.add_const(c);
                }
            }
            assert_eq!(st.min(), naive.min());
            for x in [-LIM, -1, 0, 1, LIM] {
                assert_eq!(st.eval(x), naive.eval(x), "x={x}");
            }
        }
    }

    #[test]
    fn add_const_works() {
        let mut st = SlopeTrick::new();
        st.add_abs(3);
        st.add_const(5);
        assert_eq!(st.min(), 5);
        assert_eq!(st.eval(3), 5);
        assert_eq!(st.eval(0), 8);
    }

    #[test]
    fn clear_right_and_left() {
        let mut next = xorshift();
        let mut st = SlopeTrick::new();
        let mut naive = Naive::new();
        for _ in 0..30 {
            let a = (next() % (2 * LIM as u64 + 1)) as i64 - LIM;
            st.add_abs(a);
            naive.add_right(a);
            naive.add_left(a);
        }
        if next() % 2 == 0 {
            st.clear_right();
            naive.clear_right();
        } else {
            st.clear_left();
            naive.clear_left();
        }
        assert_eq!(st.min(), naive.min());
        for x in [-LIM, -10, 0, 10, LIM] {
            assert_eq!(st.eval(x), naive.eval(x), "x={x}");
        }
    }

    #[test]
    fn shift_translate() {
        let mut st = SlopeTrick::new();
        st.add_abs(0); // f(x) = |x|
        st.translate(5); // g(x) = f(x-5) = |x-5|
        assert_eq!(st.min(), 0);
        assert_eq!(st.argmin(), (5, 5));
        assert_eq!(st.eval(5), 0);
        assert_eq!(st.eval(8), 3);
        assert_eq!(st.eval(2), 3);
    }

    #[test]
    fn shift_window_min() {
        let mut st = SlopeTrick::new();
        st.add_abs(0); // f(x) = |x|
        st.shift(2, 5); // g(x) = min_{x-5<=y<=x-2} |y|
        // [2,5] の範囲でフラットになるはず。
        assert_eq!(st.eval(2), 0);
        assert_eq!(st.eval(5), 0);
        assert_eq!(st.eval(3), 0);
        // x=1 なら区間は [-4,-1] なので min|y| = 1
        assert_eq!(st.eval(1), 1);
        // x=6 なら区間は [1,4] なので min|y| = 1
        assert_eq!(st.eval(6), 1);
    }

    #[test]
    fn merge_sums_functions() {
        let mut next = xorshift();
        for _ in 0..20 {
            let mut a = SlopeTrick::new();
            let mut b = SlopeTrick::new();
            let mut naive_a = Naive::new();
            let mut naive_b = Naive::new();
            for _ in 0..10 {
                let x = (next() % (2 * LIM as u64 + 1)) as i64 - LIM;
                a.add_abs(x);
                naive_a.add_right(x);
                naive_a.add_left(x);
            }
            for _ in 0..10 {
                let x = (next() % (2 * LIM as u64 + 1)) as i64 - LIM;
                b.add_abs(x);
                naive_b.add_right(x);
                naive_b.add_left(x);
            }
            a.merge(b);
            for x in [-LIM, -5, 0, 5, LIM] {
                assert_eq!(a.eval(x), naive_a.eval(x) + naive_b.eval(x), "x={x}");
            }
        }
    }

    // --- ConvexConjugate ---

    /// 愚直な差分配列による参照実装。
    struct NaiveConj {
        f: Vec<i64>, // f[argmin_idx + k] が argmin からの相対位置 k での値
        argmin_idx: usize,
    }

    impl NaiveConj {
        fn new() -> Self {
            Self {
                f: vec![0],
                argmin_idx: 0,
            }
        }

        fn min(&self) -> i64 {
            *self.f.iter().min().unwrap()
        }

        fn insert(&mut self, d: i64) {
            let n = self.f.len();
            let mut g = vec![i64::MAX / 2; n + 1];
            for (i, &v) in self.f.iter().enumerate() {
                g[i] = g[i].min(v); // h(0) = 0
                g[i + 1] = g[i + 1].min(v + d); // h(1) = d
            }
            self.f = g;
            self.argmin_idx = self
                .f
                .iter()
                .enumerate()
                .min_by_key(|&(_, &v)| v)
                .unwrap()
                .0;
        }

        fn eval_offset(&self, k: i64) -> i64 {
            let idx = self.argmin_idx as i64 + k;
            self.f[idx as usize]
        }
    }

    #[test]
    fn conjugate_insert_matches_naive() {
        let mut next = xorshift();
        let mut cc = ConvexConjugate::new();
        let mut naive = NaiveConj::new();
        for _ in 0..200 {
            let d = (next() % 2001) as i64 - 1000;
            cc.insert(d);
            naive.insert(d);
            assert_eq!(cc.min(), naive.min());
            let n = (cc.len() - 1) as i64;
            // argmin からの相対位置で全点チェック (argmin の位置はナイーブ側から復元)
            let left = naive.argmin_idx as i64;
            let right = (naive.f.len() - 1 - naive.argmin_idx) as i64;
            assert_eq!(left + right, n);
            for k in -left..=right {
                assert_eq!(cc.eval_offset(k), naive.eval_offset(k), "k={k}");
            }
        }
    }

    #[test]
    fn conjugate_convolve_matches_naive() {
        let mut next = xorshift();
        let mut cc_a = ConvexConjugate::new();
        let mut naive_a = NaiveConj::new();
        let mut cc_b = ConvexConjugate::new();
        let mut naive_b = NaiveConj::new();
        for _ in 0..15 {
            let d = (next() % 201) as i64 - 100;
            cc_a.insert(d);
            naive_a.insert(d);
        }
        for _ in 0..15 {
            let d = (next() % 201) as i64 - 100;
            cc_b.insert(d);
            naive_b.insert(d);
        }
        cc_a.convolve(cc_b);

        // min-plus convolution の愚直計算
        let offsets_a: Vec<i64> = (0..naive_a.f.len() as i64)
            .map(|i| i - naive_a.argmin_idx as i64)
            .collect();
        let offsets_b: Vec<i64> = (0..naive_b.f.len() as i64)
            .map(|i| i - naive_b.argmin_idx as i64)
            .collect();
        let mut best: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
        for (&oa, &va) in offsets_a.iter().zip(naive_a.f.iter()) {
            for (&ob, &vb) in offsets_b.iter().zip(naive_b.f.iter()) {
                let e = best.entry(oa + ob).or_insert(i64::MAX / 2);
                *e = (*e).min(va + vb);
            }
        }
        let min_offset = *best.keys().min().unwrap();
        let max_offset = *best.keys().max().unwrap();
        let global_min = *best.values().min().unwrap();

        assert_eq!(cc_a.min(), global_min);
        for k in min_offset..=max_offset {
            if let Some(&expected) = best.get(&k) {
                assert_eq!(cc_a.eval_offset(k), expected, "k={k}");
            }
        }
    }
}
