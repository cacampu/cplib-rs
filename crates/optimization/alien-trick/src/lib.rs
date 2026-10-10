//! Alien's trick (Lagrangian relaxation / WQS binary search)。
//!
//! 「要素数がちょうど `target` であるときの最小値」を、要素数に `p` の
//! ペナルティを課した (要素数に制約のない) 最適化問題
//! `G(p) = min_x (f(x) + p * count(x))` を解くことで求める。
//!
//! `F(target) = min_{count(x)=target} f(x)` が `target` について下に凸
//! であれば、双対性により
//! `F(target) = max_p (G(p) - p * target)`
//! が成り立つ (強双対性)。この `p` を探す3つの方法を用意している。
//!
//! 参考: <https://noshi91.hatenablog.com/entry/2023/11/20/052227>

/// パターン1: 呼び出し側が `solver(p)` として `(G(p), k)` の両方を返す場合。
///
/// `k` は「ペナルティ `p` のもとでの最適解が達成する要素数」のうち
/// いずれか一つでよいが、**`p` に関して単調非増加**であるように選ぶこと
/// (複数の `k` が同じ値を達成する場合は大きい方の `k` を返すのが安全)。
///
/// `[p_lo, p_hi]` の範囲を整数二分探索し、`O(log(p_hi-p_lo))` 回 `solver`
/// を呼ぶ。
pub fn solve_with_count(
    target: i64,
    mut p_lo: i64,
    mut p_hi: i64,
    mut solver: impl FnMut(i64) -> (i64, i64),
) -> i64 {
    assert!(p_lo <= p_hi);
    while p_lo < p_hi {
        let mid = p_lo + (p_hi - p_lo) / 2;
        let (_, k) = solver(mid);
        if k <= target {
            p_hi = mid;
        } else {
            p_lo = mid + 1;
        }
    }
    let (g, _) = solver(p_lo);
    g - p_lo * target
}

/// パターン2: 呼び出し側が `solver(p)` として `G(p)` だけを返す場合。
///
/// 要素数 `k` を呼び出し側から受け取らず、`k = G(p+1) - G(p)`
/// (離散的な傾き) を自前で計算して二分探索する。`solver` を1回の
/// 傾き計算あたり2回呼ぶため、パターン1の2倍の呼び出し回数になる。
pub fn solve_by_slope(
    target: i64,
    mut p_lo: i64,
    mut p_hi: i64,
    mut solver: impl FnMut(i64) -> i64,
) -> i64 {
    assert!(p_lo <= p_hi);
    while p_lo < p_hi {
        let mid = p_lo + (p_hi - p_lo) / 2;
        let k = solver(mid + 1) - solver(mid);
        if k <= target {
            p_hi = mid;
        } else {
            p_lo = mid + 1;
        }
    }
    solver(p_lo) - p_lo * target
}

/// パターン3: 呼び出し側が `solver(p)` として `G(p)` だけを返す場合に、
/// 要素数 `k` を一切利用せず、`h(p) = G(p) - p * target` を黄金分割探索で
/// 直接最大化する。
///
/// `G` が凹 (`f` の最小化問題) であれば `h` も凹なので、三分探索より
/// 評価回数の少ない黄金分割探索が使える。`target` ちょうどを達成する `k`
/// の計算が不要なため、タイの処理などを気にせずライブラリ化しやすい。
///
/// `iterations` 回だけ区間を縮める (目安: 浮動小数点で十分な精度を得るには
/// 100 回程度で十分)。
pub fn solve_by_golden_section(
    target: f64,
    mut lo: f64,
    mut hi: f64,
    iterations: usize,
    mut solver: impl FnMut(f64) -> f64,
) -> f64 {
    assert!(lo <= hi);
    let gold = (5f64.sqrt() - 1.0) / 2.0;
    let h = |p: f64, solver: &mut dyn FnMut(f64) -> f64| solver(p) - p * target;

    let mut x1 = hi - gold * (hi - lo);
    let mut x2 = lo + gold * (hi - lo);
    let mut f1 = h(x1, &mut solver);
    let mut f2 = h(x2, &mut solver);

    for _ in 0..iterations {
        if f1 < f2 {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + gold * (hi - lo);
            f2 = h(x2, &mut solver);
        } else {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - gold * (hi - lo);
            f1 = h(x1, &mut solver);
        }
    }
    f1.max(f2)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `F(k) = (k - center)^2 * scale + base` という下に凸な整数列を使った
    /// テスト用の問題。`solver(p)` はこれに `p*k` のペナルティを加えた
    /// 最適化を愚直に `0..=n` の全探索で解く。
    struct Problem {
        n: i64,
        center: i64,
        scale: i64,
        base: i64,
    }

    impl Problem {
        fn f(&self, k: i64) -> i64 {
            (k - self.center).pow(2) * self.scale + self.base
        }

        /// (G(p), 最適な k のうち最大のもの)
        fn solve_with_count(&self, p: i64) -> (i64, i64) {
            let mut best_g = i64::MAX;
            let mut best_k = 0;
            for k in 0..=self.n {
                let g = self.f(k) + p * k;
                if g < best_g || (g == best_g && k > best_k) {
                    best_g = g;
                    best_k = k;
                }
            }
            (best_g, best_k)
        }

        fn solve_value_only(&self, p: i64) -> i64 {
            self.solve_with_count(p).0
        }

        fn solve_value_only_f64(&self, p: f64) -> f64 {
            (0..=self.n)
                .map(|k| self.f(k) as f64 + p * k as f64)
                .fold(f64::INFINITY, f64::min)
        }
    }

    const PROBLEMS: [Problem; 3] = [
        Problem {
            n: 30,
            center: 12,
            scale: 3,
            base: 100,
        },
        Problem {
            n: 1,
            center: 0,
            scale: 5,
            base: -7,
        },
        Problem {
            n: 50,
            center: 50,
            scale: 1,
            base: 0,
        },
    ];

    #[test]
    fn solve_with_count_matches_brute_force() {
        for problem in &PROBLEMS {
            for target in 0..=problem.n {
                let expected = problem.f(target);
                let got =
                    solve_with_count(target, -10_000, 10_000, |p| problem.solve_with_count(p));
                assert_eq!(got, expected, "target={target}");
            }
        }
    }

    #[test]
    fn solve_by_slope_matches_brute_force() {
        for problem in &PROBLEMS {
            for target in 0..=problem.n {
                let expected = problem.f(target);
                let got = solve_by_slope(target, -10_000, 10_000, |p| problem.solve_value_only(p));
                assert_eq!(got, expected, "target={target}");
            }
        }
    }

    #[test]
    fn solve_by_golden_section_matches_brute_force() {
        for problem in &PROBLEMS {
            for target in 0..=problem.n {
                let expected = problem.f(target) as f64;
                let got = solve_by_golden_section(target as f64, -10_000.0, 10_000.0, 200, |p| {
                    problem.solve_value_only_f64(p)
                });
                assert!(
                    (got - expected).abs() < 1e-3,
                    "target={target}, got={got}, expected={expected}"
                );
            }
        }
    }
}
