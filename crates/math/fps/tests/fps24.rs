//! FPS 24 の公開サンプルを使った組合せテスト。
//! https://atcoder.jp/contests/fps-24/tasks

use fps::{Factorials, Fps, bostan_mori, convolution_dp, power_sums, sum_rationals};
use modint::ModInt998244353 as Mi;

type F = Fps<998244353>;

// G: スライドする硬貨の集合を (1-x^k) との乗除算で更新する。
fn coin(n: usize, m: usize, length: usize) -> Vec<Mi> {
    let mut f = F::from_ints(&[1]).pre(n + 1);
    for k in 1..=length {
        f.div_binomial_assign(k, Mi::new(-1));
    }
    let mut result = vec![f[n]];
    for left in 1..=m - length {
        f.mul_binomial_assign(left, Mi::new(-1));
        f.div_binomial_assign(left + length, Mi::new(-1));
        result.push(f[n]);
    }
    result
}

#[test]
fn g_coin_samples() {
    assert_eq!(coin(5, 3, 2), [3, 1].map(Mi::new));
    assert_eq!(
        coin(5000, 2500, 2495),
        [
            878712345, 520404421, 886134625, 125526485, 307727973, 257205353
        ]
        .map(Mi::new)
    );
}

#[test]
fn i_score_samples() {
    let score = |a: &[i32], k| a.iter().map(|&a| F::from_ints(&[1, a])).product::<F>()[k];
    assert_eq!(score(&[2, 3, 5], 2), Mi::new(31));
    assert_eq!(score(&(1..=10).collect::<Vec<_>>(), 5), Mi::new(902055));
}

fn sugoroku(n: usize, dice: &[usize], traps: &[usize]) -> Mi {
    let mut kernel = vec![Mi::new(0); n + 1];
    for &a in dice {
        kernel[a] = Mi::new(dice.len()).inv();
    }
    let mut initial = vec![Mi::new(0); n * 2];
    initial[0] = Mi::new(1);
    let mut is_trap = vec![false; n];
    for &b in traps {
        is_trap[b] = true;
    }
    let mut result = Mi::new(0);
    convolution_dp(&initial, &kernel, |i, value| {
        if i >= n {
            result += value;
            Mi::new(0)
        } else if is_trap[i] {
            Mi::new(0)
        } else {
            value
        }
    });
    result
}

#[test]
fn j_sugoroku_samples() {
    assert_eq!(sugoroku(2, &[1, 2], &[1]), Mi::new(499122177));
    assert_eq!(
        sugoroku(
            250000,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 100000],
            &[21994, 47718, 98917, 104184, 160670, 204838, 205220, 207793]
        ),
        Mi::new(718440495)
    );
}

fn ball(n: usize, m: usize, k: usize) -> Vec<Mi> {
    let table = Factorials::<998244353>::new(n);
    let mut f = F::zeros(n + 1);
    for i in 0..=k {
        f[n - i] = table.binom(n, i);
    }
    let points: Vec<_> = (1..=m).map(Mi::new).collect();
    f.multipoint_eval(&points)
}

#[test]
fn p_ball_samples() {
    assert_eq!(ball(3, 2, 1), [4, 20].map(Mi::new));
    assert_eq!(
        ball(12345, 5, 6789),
        [583034791, 982161077, 613932842, 770852810, 194914198].map(Mi::new)
    );
}

fn dice(a: &[i32], b: &[i32], k: usize) -> Vec<Mi> {
    let table = Factorials::<998244353>::new(k);
    let egf = |values: &[i32]| {
        let values: Vec<_> = values.iter().copied().map(Mi::new).collect();
        let mut f = power_sums(&values, k + 1);
        for i in 0..=k {
            f[i] *= table.inv_fact[i];
        }
        f
    };
    let sum = egf(a).mul_truncated(&egf(b), k + 1);
    let probability = Mi::new(a.len() * b.len()).inv();
    (1..=k)
        .map(|i| sum[i] * table.fact[i] * probability)
        .collect()
}

#[test]
fn q_dice_samples() {
    assert_eq!(
        dice(&[1, 2, 3], &[4, 5], 3),
        [499122183, 166374102, 499122469].map(Mi::new)
    );
    assert_eq!(
        dice(
            &[
                26056440, 37770730, 49119845, 54471601, 59187620, 86450214, 95205990, 97771081
            ],
            &[
                12521133, 14054302, 19786177, 21524935, 39265456, 53591023, 65158870, 86176948,
                87221915
            ],
            5
        ),
        [981084750, 45312313, 562611236, 522662310, 196292405].map(Mi::new)
    );
}

fn random_walk(n: usize, t: u64, x: usize) -> Mi {
    let size = 1 << (n + 1);
    let mut step = F::zeros(size);
    let half = Mi::new(2).inv();
    step[1] = half;
    step[size - 1] = half;
    let distribution = step.pow_cyclic(t, size);
    let position = x - 1;
    if position == 0 || position == size / 2 {
        distribution[position]
    } else {
        distribution[position] + distribution[size - position]
    }
}

#[test]
fn r_random_walk_samples() {
    assert_eq!(random_walk(2, 3, 2), Mi::new(249561089));
    assert_eq!(random_walk(10, 12345, 678), Mi::new(530802129));
}

fn colorful(a: &[i32], t: u64) -> Mi {
    let (p, q) = sum_rationals(
        a[1..]
            .iter()
            .map(|&a| (F::from_ints(&[a]), F::from_ints(&[1, a]))),
    );
    // 色1を途中に含まない帰還を1つの区間として分解する。
    // J = (Q-xP) / (Q-xP-a[0] x^2 P) は色1への帰還数の母関数。
    let numerator = q - p.shift_up(1);
    let denominator = numerator.clone() - p.shift_up(2) * Mi::new(a[0]);
    bostan_mori(&numerator, &denominator, t) / Mi::new(a[0])
}

#[test]
fn t_colorful_samples() {
    assert_eq!(colorful(&[2, 1, 2], 3), Mi::new(4));
    assert_eq!(
        colorful(
            &[
                766294630, 440423914, 59187620, 725560241, 585990757, 965580536, 623321126,
                550925214, 122410709, 549392045
            ],
            31415926535897932
        ),
        Mi::new(66487687)
    );
    assert_eq!(colorful(&[5], 100), Mi::new(0));
}
