//! `NaiveCopy` の正当性。
//!
//! トイ問題は幅を十分大きくすれば最適解に一致するので、全探索と突き合わせられる。

mod common;

use beam::{BeamSearch, FixedWidth, NaiveCopy};
use common::{Context, Toy};

type Naive = BeamSearch<Toy, NaiveCopy<Toy>, FixedWidth>;

fn run(ctx: Context, width: usize) -> (Vec<u16>, i64) {
    let t = ctx.t;
    let mut search = Naive::new(ctx, FixedWidth::new(width));
    let path = search.run(t);
    let score = search.ctx().replay(&path);
    (path, score)
}

/// 幅を十分に取れば全探索と一致する。
#[test]
fn matches_brute_force() {
    for seed in 0..64 {
        let ctx = Context::generate(seed, 6, 8);
        let expected = ctx.brute_force();
        let (path, score) = run(ctx, 100_000);
        assert_eq!(path.len(), 8, "seed={seed} 手順長が違う");
        assert_eq!(score, expected, "seed={seed} 最適解に届いていない");
    }
}

/// 幅を絞っても、返る手順は必ず実行可能で、最適解を超えない。
#[test]
fn narrow_width_is_feasible() {
    for seed in 0..64 {
        for width in [1, 2, 3, 7] {
            let ctx = Context::generate(seed, 8, 10);
            let optimum = ctx.brute_force();
            let (path, score) = run(ctx, width);
            // replay 側で経路の連結性を assert しているので、ここは得点だけ見る。
            assert_eq!(path.len(), 10, "seed={seed} width={width}");
            assert!(score <= optimum, "seed={seed} width={width} 最適解を超えた");
        }
    }
}

/// 同じ入力からは同じ解が出る。
///
/// # 幅の単調性は成り立たない
///
/// 「幅を広げれば悪化しない」は**偽**なので、テストに書いてはいけない。幅 w+1 の
/// 候補集合は幅 w のものを値の意味で上から抑えるが、残る**頂点そのもの**が
/// 違うため、その先の展開が変わって最終的に悪化しうる。実際 seed=23, n=10, t=12 で
/// 幅 4 が 476、幅 8 が 359 になる。
///
/// 保証できるのは「幅が候補数を上回れば全探索と一致する」ことだけで、それは
/// [`matches_brute_force`] が見ている。
#[test]
fn deterministic() {
    for seed in 0..32 {
        for width in [1, 3, 16] {
            let (path_a, score_a) = run(Context::generate(seed, 10, 12), width);
            let (path_b, score_b) = run(Context::generate(seed, 10, 12), width);
            assert_eq!(path_a, path_b, "seed={seed} width={width}");
            assert_eq!(score_a, score_b, "seed={seed} width={width}");
        }
    }
}

/// コーナーケース。
#[test]
fn corner_cases() {
    // 初回ターンのみ
    let ctx = Context::generate(0, 5, 1);
    let (path, _) = run(ctx, 4);
    assert_eq!(path.len(), 1);

    // 幅1
    let ctx = Context::generate(1, 5, 6);
    let (path, _) = run(ctx, 1);
    assert_eq!(path.len(), 6);

    // マスが1つしかなく、動きようがない
    let mut ctx = Context::generate(2, 1, 5);
    ctx.start = 0;
    let (path, _) = run(ctx, 8);
    assert_eq!(path.len(), 5);
}

/// ターン数 0 なら空の手順が返る。
#[test]
fn zero_turns() {
    let ctx = Context::generate(0, 5, 0);
    let mut search = Naive::new(ctx, FixedWidth::new(4));
    assert!(search.run(0).is_empty());
}
