//! バックエンド間の結果一致。`NaiveCopy` / `PostorderTour` / `LinkedTree` の3種。
//!
//! 差分更新のバグは静かに壊れるので、素朴実装との突き合わせが最も強力な検出手段。
//!
//! # 何が一致して何が一致しないか
//!
//! トイ問題の `Logic::select` は `(得点, ハッシュ)` で**全順序**に並べてある
//! （`common/mod.rs`）。そのため**毎ターンの候補集合を `(得点, ハッシュ)` の多重集合
//! として見ると完全に一致する**。既定の `dedup_top_k` は `value` だけで切るので、
//! 同点候補のどれが残るかが候補の構築順で決まり、バックエンド間の一致は保証されない。
//! ここで検査したいのは `Tree` なので、選択を決定的にしてから比べる。
//!
//! 一方で**手順（`Vec<Action>`）の一致は保証されない**。同じ `(value, hash)` に至る
//! 別経路の候補があると、重複除去がどちらを代表に残すかが候補の並び順で決まる
//! ためである。例えば「5→4→4」と「5→5→4」はどちらも位置4・訪問済み{4,5}なので
//! 同じハッシュ・同じ得点になるが、辺の列は違う。状態としては同一なので以後の探索は
//! 変わらない。
//!
//! そこでバックエンド間では `(得点, ハッシュ)` を突き合わせ、経路の正しさは
//! [`restored_paths_are_consistent`] が各バックエンドの内部で検査する
//! （復元した経路を根から再生し、`Node` が主張する得点と一致するか）。

mod common;

use beam::{BeamSearch, FixedWidth, LinkedTree, Logic, NaiveCopy, PostorderTour, Tree};
use common::{Context, Toy};

fn run<T>(ctx: Context, width: usize) -> (Vec<u16>, i64)
where
    T: Tree<Toy>,
{
    let t = ctx.t;
    let mut s = BeamSearch::<Toy, T, _>::new(ctx, FixedWidth::new(width));
    let path = s.run(t);
    let score = s.ctx().replay(&path);
    (path, score)
}

/// 各ターンの候補を「全候補の (得点, ハッシュ, 復元経路)」として記録する。
///
/// `protect` を全候補について呼ぶので、復元規則が全経路について正しいかを検査できる。
/// `BeamSearch` 経由だと最終ターンの1本しか復元されないため、ここは `Tree` を直に叩く。
type Log = Vec<Vec<(i64, u64, Vec<u16>)>>;

fn drive<T>(ctx: &mut Context, width: usize, turns: usize) -> Log
where
    T: Tree<Toy>,
{
    let (mut state, root) = Toy::root(ctx);
    let mut tree = T::new(root, width);
    let mut selector = beam::Selector::<Toy>::new();
    let mut chosen: Vec<u32> = Vec::new();
    let mut log = Log::new();

    for _ in 0..turns {
        tree.begin();
        while tree.walk(&mut state, ctx) {
            Toy::expand(&mut state, ctx, &mut tree);
        }
        if tree.candidates().is_empty() {
            break;
        }

        let mut rec = Vec::with_capacity(tree.candidates().len());
        for i in 0..tree.candidates().len() {
            let node = tree.candidates()[i];
            let guard = tree.protect(i as u32);
            let path = tree.path(&guard);
            tree.release(guard);
            rec.push((node.score, node.hash, path));
        }
        rec.sort();
        log.push(rec);

        chosen.clear();
        Toy::select(tree.candidates(), ctx, width, &mut selector, &mut chosen);
        tree.commit(&chosen);
    }
    log
}

/// 経路を落として `(得点, ハッシュ)` の列にする。
fn keys(log: &Log) -> Vec<Vec<(i64, u64)>> {
    log.iter()
        .map(|rec| rec.iter().map(|&(s, h, _)| (s, h)).collect())
        .collect()
}

/// 毎ターンの候補集合が `(得点, ハッシュ)` の多重集合として一致する。ここが本命。
#[test]
fn same_candidates_as_naive() {
    for seed in 0..200 {
        for width in [1, 2, 3, 5, 11, 64] {
            let n = 4 + (seed as usize % 9);
            let t = 1 + (seed as usize % 14);
            let a = drive::<NaiveCopy<Toy>>(&mut Context::generate(seed, n, t), width, t);
            for b in [
                drive::<PostorderTour<Toy>>(&mut Context::generate(seed, n, t), width, t),
                drive::<LinkedTree<Toy>>(&mut Context::generate(seed, n, t), width, t),
            ] {
                assert_eq!(a.len(), b.len(), "seed={seed} n={n} t={t} width={width}");
                for (turn, (x, y)) in keys(&a).iter().zip(&keys(&b)).enumerate() {
                    assert_eq!(x, y, "seed={seed} n={n} t={t} width={width} turn={turn}");
                }
            }
        }
    }
}

/// 復元された経路がすべて実行可能で、`Node` が主張する得点と一致する。
///
/// `PostorderTour` の復元規則（`tour` / `leaf_ofs` の窓）を直接叩く検査。
/// 全候補・全ターンについて確かめるので、経路の一致が保証できない分をここで補う。
#[test]
fn restored_paths_are_consistent() {
    for seed in 0..64 {
        for width in [1, 2, 7, 32] {
            let n = 4 + (seed as usize % 9);
            let t = 1 + (seed as usize % 12);
            let mut ctx = Context::generate(seed, n, t);
            let post = drive::<PostorderTour<Toy>>(&mut ctx, width, t);
            let naive = drive::<NaiveCopy<Toy>>(&mut ctx, width, t);
            let linked = drive::<LinkedTree<Toy>>(&mut ctx, width, t);
            for log in [&post, &naive, &linked] {
                for (turn, rec) in log.iter().enumerate() {
                    for (score, _, path) in rec {
                        assert_eq!(
                            path.len(),
                            turn + 1,
                            "seed={seed} width={width} turn={turn} 経路長が違う"
                        );
                        // replay が経路の連結性も assert する。
                        assert_eq!(
                            ctx.replay(path),
                            *score,
                            "seed={seed} width={width} turn={turn} 得点が違う"
                        );
                    }
                }
            }
        }
    }
}

/// `BeamSearch` 経由でも得点は一致する。
#[test]
fn same_score_as_naive() {
    for seed in 0..200 {
        for width in [1, 2, 3, 5, 11, 64] {
            let n = 4 + (seed as usize % 9);
            let t = 1 + (seed as usize % 14);
            let (pa, sa) = run::<NaiveCopy<Toy>>(Context::generate(seed, n, t), width);
            for (pb, sb) in [
                run::<PostorderTour<Toy>>(Context::generate(seed, n, t), width),
                run::<LinkedTree<Toy>>(Context::generate(seed, n, t), width),
            ] {
                assert_eq!(sa, sb, "seed={seed} n={n} t={t} width={width} 得点が違う");
                assert_eq!(pa.len(), pb.len(), "seed={seed} 手順長が違う");
            }
        }
    }
}

/// 幅を十分取れば PostorderTour も全探索に一致する。
#[test]
fn postorder_matches_brute_force() {
    for seed in 0..64 {
        let ctx = Context::generate(seed, 6, 8);
        let expected = ctx.brute_force();
        let (path, score) = run::<PostorderTour<Toy>>(ctx, 100_000);
        assert_eq!(path.len(), 8, "seed={seed}");
        assert_eq!(score, expected, "seed={seed} 最適解に届いていない");
    }
}

/// コーナーケース。
#[test]
fn corner_cases() {
    // 初回ターンのみ
    let (path, _) = run::<PostorderTour<Toy>>(Context::generate(0, 5, 1), 4);
    assert_eq!(path.len(), 1);

    // 幅1
    let (path, _) = run::<PostorderTour<Toy>>(Context::generate(1, 5, 6), 1);
    assert_eq!(path.len(), 6);

    // マスが1つで動きようがない（毎ターン候補が1つ、全部同じハッシュ）
    let mut ctx = Context::generate(2, 1, 5);
    ctx.start = 0;
    let (path, _) = run::<PostorderTour<Toy>>(ctx, 8);
    assert_eq!(path.len(), 5);

    // ターン数 0
    let ctx = Context::generate(0, 5, 0);
    let mut s = BeamSearch::<Toy, PostorderTour<Toy>, _>::new(ctx, FixedWidth::new(4));
    assert!(s.run(0).is_empty());
}

/// 深い木でも壊れない。`tour` の使い回しが効いているかを見る。
#[test]
fn deep() {
    for seed in 0..16 {
        let a = drive::<NaiveCopy<Toy>>(&mut Context::generate(seed, 12, 60), 8, 60);
        for b in [
            drive::<PostorderTour<Toy>>(&mut Context::generate(seed, 12, 60), 8, 60),
            drive::<LinkedTree<Toy>>(&mut Context::generate(seed, 12, 60), 8, 60),
        ] {
            assert_eq!(keys(&a), keys(&b), "seed={seed}");
        }
        assert_eq!(a.len(), 60);
    }
}
