/// 傾き・問い合わせの両方が単調である場合に使える Convex Hull Trick。
///
/// 内部的には常に最小値として管理し、`max` モードでは直線を
/// `(a, b) -> (-a, -b)` と反転させてから挿入し、問い合わせ結果を反転して
/// 返す (`max_x(ax+b) = -min_x((-a)x+(-b))`)。
///
/// - `min` モードで追加する直線の傾き `a` は**非増加**順であること。
/// - `max` モードで追加する直線の傾き `a` は**非減少**順であること。
/// - 問い合わせる `x` はどちらのモードでも**非減少**順であること。
///
/// これらを満たせば `add_line` / `query` は償却 `O(1)`。
pub struct MonotoneCht {
    maximize: bool,
    // 内部的には常に「最小値を取る直線集合」として、傾き非増加順に保持する。
    lines: Vec<(i64, i64)>,
    ptr: usize,
}

impl MonotoneCht {
    /// 最小値を問い合わせるための CHT。
    pub fn new_min() -> Self {
        Self {
            maximize: false,
            lines: Vec::new(),
            ptr: 0,
        }
    }

    /// 最大値を問い合わせるための CHT。
    pub fn new_max() -> Self {
        Self {
            maximize: true,
            lines: Vec::new(),
            ptr: 0,
        }
    }

    #[inline]
    fn eval(line: (i64, i64), x: i64) -> i64 {
        line.0 * x + line.1
    }

    /// `l2` が `l1` (直前) と `l3` (今追加する直線) によって不要になるか判定する。
    /// 傾きは `l1.0 > l2.0 > l3.0` を仮定する (呼び出し側で保証する)。
    fn is_unnecessary(l1: (i64, i64), l2: (i64, i64), l3: (i64, i64)) -> bool {
        // 交点 x(l1,l3) <= x(l1,l2) <=> (b3-b1)(a1-a2) <= (b2-b1)(a1-a3)
        (l3.1 - l1.1) as i128 * (l1.0 - l2.0) as i128
            <= (l2.1 - l1.1) as i128 * (l1.0 - l3.0) as i128
    }

    /// 直線 `y = a*x + b` を追加する。
    pub fn add_line(&mut self, a: i64, b: i64) {
        let line = if self.maximize { (-a, -b) } else { (a, b) };
        debug_assert!(
            self.lines.last().is_none_or(|&last| last.0 >= line.0),
            "add_line: slope must be monotone (non-increasing internally)"
        );

        // 傾きが同じ直線が既にあれば、良い方だけ残す (小さい切片が常に勝つ)。
        if let Some(&last) = self.lines.last()
            && last.0 == line.0
        {
            if line.1 >= last.1 {
                return;
            }
            self.lines.pop();
        }

        while self.lines.len() >= 2 {
            let l2 = self.lines[self.lines.len() - 1];
            let l1 = self.lines[self.lines.len() - 2];
            if Self::is_unnecessary(l1, l2, line) {
                self.lines.pop();
            } else {
                break;
            }
        }
        self.lines.push(line);
        self.ptr = self.ptr.min(self.lines.len() - 1);
    }

    /// `x` での最小値 (または `max` モードなら最大値) を求める。
    pub fn query(&mut self, x: i64) -> i64 {
        assert!(!self.lines.is_empty(), "query on empty MonotoneCht");
        while self.ptr + 1 < self.lines.len()
            && Self::eval(self.lines[self.ptr], x) >= Self::eval(self.lines[self.ptr + 1], x)
        {
            self.ptr += 1;
        }
        let v = Self::eval(self.lines[self.ptr], x);
        if self.maximize { -v } else { v }
    }

    /// 単調性を仮定せず `O(N)` で問い合わせる (デバッグ・テスト用)。
    pub fn query_naive(&self, x: i64) -> i64 {
        assert!(!self.lines.is_empty(), "query on empty MonotoneCht");
        let v = self.lines.iter().map(|&l| Self::eval(l, x)).min().unwrap();
        if self.maximize { -v } else { v }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
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

    #[test]
    fn min_matches_naive() {
        let mut next = xorshift();
        let mut slopes: Vec<i64> = (0..100).map(|_| (next() % 2001) as i64 - 1000).collect();
        slopes.sort_unstable_by(|a, b| b.cmp(a)); // 非増加
        let mut cht = MonotoneCht::new_min();
        for &a in &slopes {
            let b = (next() % 2001) as i64 - 1000;
            cht.add_line(a, b);
        }
        let mut xs: Vec<i64> = (0..200).map(|_| (next() % 2001) as i64 - 1000).collect();
        xs.sort_unstable();
        for x in xs {
            assert_eq!(cht.query(x), cht.query_naive(x), "x={x}");
        }
    }

    #[test]
    fn max_matches_naive() {
        let mut next = xorshift();
        let mut slopes: Vec<i64> = (0..100).map(|_| (next() % 2001) as i64 - 1000).collect();
        slopes.sort_unstable(); // 非減少
        let mut cht = MonotoneCht::new_max();
        for &a in &slopes {
            let b = (next() % 2001) as i64 - 1000;
            cht.add_line(a, b);
        }
        let mut xs: Vec<i64> = (0..200).map(|_| (next() % 2001) as i64 - 1000).collect();
        xs.sort_unstable();
        for x in xs {
            assert_eq!(cht.query(x), cht.query_naive(x), "x={x}");
        }
    }

    #[test]
    fn duplicate_slopes_keep_the_better_one() {
        let mut cht = MonotoneCht::new_min();
        cht.add_line(2, 5);
        cht.add_line(2, 3); // 同じ傾きでより良い (小さい切片)
        cht.add_line(2, 10); // 同じ傾きでより悪い -> 無視される
        for x in [-10, 0, 10] {
            assert_eq!(cht.query_naive(x), 2 * x + 3);
        }
    }
}
