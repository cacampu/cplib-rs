//! 幅の政策が、締切を超えずに時間を使い切るか。
//!
//! 実探索は測るたびに揺れるので、「幅 M のターンは `unit * M` 秒かかる」という
//! 模擬の上で制御だけを見る。`unit` をターンの関数にすると、探索が進むにつれて
//! 1 ターンが重くなる状況を再現できる。

use beam::{BayesianWidth, FixedWidth, SqrtFeedback, WidthPolicy};

/// `(合計秒, 各ターンの幅)`。
fn simulate<W: WidthPolicy>(
    policy: &mut W,
    max_turn: usize,
    unit: impl Fn(usize) -> f64,
) -> (f64, Vec<usize>) {
    let mut elapsed = 0.0;
    let mut widths = Vec::with_capacity(max_turn);
    for turn in 0..max_turn {
        let w = policy.suggest(turn, max_turn);
        elapsed += unit(turn) * w as f64;
        policy.observe(turn, w, elapsed);
        widths.push(w);
    }
    (elapsed, widths)
}

const LIMIT: f64 = 1.0;
const TURNS: usize = 1000;

/// 1 ターンあたり `1e-6 * M` 秒。ちょうど使い切る幅は 1000。
fn constant(_turn: usize) -> f64 {
    1e-6
}

#[test]
fn bayesian_uses_the_budget() {
    let mut p = BayesianWidth::new(100, 1, 100_000, LIMIT);
    let (elapsed, widths) = simulate(&mut p, TURNS, constant);
    assert!(elapsed <= LIMIT, "締切超過: {elapsed}");
    // 使い残しは残りターンに配り直されるので、ほぼ使い切る
    assert!(elapsed > LIMIT * 0.95, "時間を使えていない: {elapsed}");
    // ちょうど使い切る幅は 1000。平均がそこに寄る
    let avg = widths.iter().sum::<usize>() / widths.len();
    assert!(
        (900..=1100).contains(&avg),
        "平均幅が理想から離れている: {avg}"
    );
}

/// 探索が進むと 1 ターンが重くなる場合、幅を絞って追従する。
#[test]
fn bayesian_follows_drift() {
    let mut p = BayesianWidth::new(1000, 1, 100_000, LIMIT);
    let (elapsed, widths) = simulate(&mut p, TURNS, |t| 1e-6 * (1.0 + 3.0 * t as f64 / 1000.0));
    assert!(elapsed <= LIMIT, "締切超過: {elapsed}");
    // 終盤は使い残しの配り直しで跳ねるので、道中で比べる
    assert!(
        widths[750] < widths[100],
        "重くなっても幅が下がらない: {} -> {}",
        widths[100],
        widths[750]
    );
    let avg = widths.iter().sum::<usize>() / widths.len();
    assert!(
        (350..=500).contains(&avg),
        "平均幅が理想 400 から離れている: {avg}"
    );
}

/// ノイズを小さくすると幅の変動が平らになる。総時間はどちらも使い切る。
#[test]
fn bayesian_noise_flattens_width() {
    let spread = |noise: Option<(f64, f64)>| {
        let mut p = BayesianWidth::new(100, 1, 100_000, LIMIT);
        if let Some((o, d)) = noise {
            p = p.with_noise(o, d);
        }
        let (elapsed, w) = simulate(&mut p, TURNS, constant);
        assert!(elapsed <= LIMIT, "締切超過: {elapsed}");
        assert!(elapsed > LIMIT * 0.95, "時間を使えていない: {elapsed}");
        w[TURNS - 1] as f64 / w[10] as f64
    };
    let loose = spread(None);
    let tight = spread(Some((0.001, 0.0001)));
    assert!(
        tight < loose,
        "ノイズを絞っても平らにならない: {tight} vs {loose}"
    );
}

/// 見積もりがどう転んでも下限・上限を守る。
#[test]
fn bayesian_respects_bounds() {
    // 極端に重い: 下限に張り付く
    let mut p = BayesianWidth::new(1000, 64, 100_000, LIMIT);
    let (_, widths) = simulate(&mut p, TURNS, |_| 1.0);
    assert_eq!(*widths.last().unwrap(), 64);

    // 極端に軽い: 上限に張り付く
    let mut p = BayesianWidth::new(10, 1, 5000, LIMIT);
    let (_, widths) = simulate(&mut p, TURNS, |_| 1e-12);
    assert_eq!(*widths.last().unwrap(), 5000);
}

#[test]
fn sqrt_feedback_stays_within_limit() {
    let mut p = SqrtFeedback::new(100, 1, 100_000, LIMIT).with_interval(64);
    let (elapsed, _) = simulate(&mut p, TURNS, constant);
    assert!(elapsed <= LIMIT, "締切超過: {elapsed}");
}

#[test]
fn fixed_width_is_constant() {
    let mut p = FixedWidth::new(123);
    let (_, widths) = simulate(&mut p, 10, constant);
    assert!(widths.iter().all(|&w| w == 123));
}
