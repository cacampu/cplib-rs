//! 数論変換 (NTT) による畳み込み。
//!
//! アルゴリズムは AtCoder Library (ac-library-rs, CC0-1.0) を参考にしている。
//!
//! 法 `M` は NTT-friendly、すなわち素数かつ `M - 1` が変換長で割り切れる必要がある。
//! 998244353 = 119 * 2^23 + 1 が代表例。任意の法に対しては [`convolution_i64`] を使う。

use modint::ModInt;

/// `m` の原始根。よく使う法は表引きし、それ以外は `m - 1` を素因数分解して求める。
pub const fn primitive_root(m: u32) -> u32 {
    match m {
        2 => 1,
        167_772_161 => 3,
        469_762_049 => 3,
        754_974_721 => 11,
        998_244_353 => 3,
        1_000_000_007 => 5,
        _ => {
            // m - 1 の各素因数 p について g^((m-1)/p) != 1 となる最小の g を探す
            let mut divs = [0u32; 20];
            let mut cnt = 0;
            let mut x = m - 1;
            while x % 2 == 0 {
                x /= 2;
            }
            divs[0] = 2;
            cnt += 1;
            let mut i = 3;
            while i as u64 * i as u64 <= x as u64 {
                if x % i == 0 {
                    divs[cnt] = i;
                    cnt += 1;
                    while x % i == 0 {
                        x /= i;
                    }
                }
                i += 2;
            }
            if x > 1 {
                divs[cnt] = x;
                cnt += 1;
            }
            let mut g = 2;
            loop {
                let mut ok = true;
                let mut i = 0;
                while i < cnt {
                    if pow_mod(g as u64, ((m - 1) / divs[i]) as u64, m as u64) == 1 {
                        ok = false;
                        break;
                    }
                    i += 1;
                }
                if ok {
                    return g;
                }
                g += 1;
            }
        }
    }
}

/// `x^n mod m`。`m < 2^31` を仮定する。
const fn pow_mod(mut x: u64, mut n: u64, m: u64) -> u64 {
    let mut ret = 1u64;
    x %= m;
    while n > 0 {
        if n & 1 == 1 {
            ret = ret * x % m;
        }
        x = x * x % m;
        n >>= 1;
    }
    ret
}

/// `M - 1` が `n` で割り切れるか。NTT が可能な変換長かの判定。
fn assert_ntt_friendly<const M: u32>(n: usize) {
    assert!(n.is_power_of_two(), "NTT length must be a power of two");
    assert!(
        (M - 1) % n as u32 == 0,
        "modulus {M} is not NTT-friendly for length {n}: {} - 1 must be divisible by {n}",
        M
    );
}

/// 数論変換。`a.len()` は2冪であること。
pub fn ntt<const M: u32>(a: &mut [ModInt<M>]) {
    transform(a, false);
}

/// 逆変換。`1/n` 倍まで含む。
pub fn intt<const M: u32>(a: &mut [ModInt<M>]) {
    transform(a, true);
}

/// Cooley-Tukey。各段の回転因子はテーブルに持つ。
///
/// 回転因子をブロックごとに `w *= root` で更新すると、乗算回数が段あたり
/// `n/2` 回になり依存関係も直列になる。段ごとに一度だけ表を作れば
/// 乗算回数は段あたり `len/2` 回で済み、内側のループも並列に動く。
fn transform<const M: u32>(a: &mut [ModInt<M>], inverse: bool) {
    let n = a.len();
    if n <= 1 {
        return;
    }
    assert_ntt_friendly::<M>(n);
    bit_reverse(a);

    let g = ModInt::<M>::new(primitive_root(M));
    let g = if inverse { g.inv() } else { g };

    let mut roots: Vec<ModInt<M>> = Vec::with_capacity(n / 2);
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        // 1の原始 len 乗根
        let root = g.pow(((M - 1) / len as u32) as u64);
        roots.clear();
        roots.push(ModInt::new(1u32));
        for k in 1..half {
            let prev = roots[k - 1];
            roots.push(prev * root);
        }

        for block in a.chunks_mut(len) {
            let (lo, hi) = block.split_at_mut(half);
            for ((x, y), &w) in lo.iter_mut().zip(hi.iter_mut()).zip(roots.iter()) {
                let u = *x;
                let v = *y * w;
                *x = u + v;
                *y = u - v;
            }
        }
        len <<= 1;
    }

    if inverse {
        let n_inv = ModInt::<M>::new(n as u32).inv();
        for x in a.iter_mut() {
            *x *= n_inv;
        }
    }
}

fn bit_reverse<T>(a: &mut [T]) {
    let n = a.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            a.swap(i, j);
        }
    }
}

/// 小さい入力向けの素朴な畳み込み。`O(n m)`。
pub fn convolution_naive<const M: u32>(a: &[ModInt<M>], b: &[ModInt<M>]) -> Vec<ModInt<M>> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut ret = vec![ModInt::raw(0); a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            ret[i + j] += x * y;
        }
    }
    ret
}

/// 畳み込み `c[k] = sum_{i+j=k} a[i] b[j]`。`O((n + m) log(n + m))`。
///
/// 法 `M` は NTT-friendly でなければならない。
pub fn convolution<const M: u32>(a: &[ModInt<M>], b: &[ModInt<M>]) -> Vec<ModInt<M>> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let (n, m) = (a.len(), b.len());
    // 変換の準備のほうが高くつく範囲は素朴に計算する
    if n.min(m) <= 60 {
        return convolution_naive(a, b);
    }

    let len = (n + m - 1).next_power_of_two();
    let mut fa = a.to_vec();
    let mut fb = b.to_vec();
    fa.resize(len, ModInt::raw(0));
    fb.resize(len, ModInt::raw(0));

    ntt(&mut fa);
    ntt(&mut fb);
    for (x, y) in fa.iter_mut().zip(fb.iter()) {
        *x *= *y;
    }
    intt(&mut fa);

    fa.truncate(n + m - 1);
    fa
}

// 任意 mod 畳み込み用の NTT-friendly な3つの法。積は 2^63 を超える。
const MOD1: u32 = 754_974_721; // 2^24 * 45 + 1
const MOD2: u32 = 167_772_161; // 2^25 * 5 + 1
const MOD3: u32 = 469_762_049; // 2^26 * 7 + 1

/// 整数列の畳み込み。真の値が `i64` に収まることを前提とする。
///
/// NTT-friendly な3つの法で畳み込んでから中国剰余定理で復元するので、
/// 法が NTT に向かない場合や法を取らない整数の畳み込みに使える。
pub fn convolution_i64(a: &[i64], b: &[i64]) -> Vec<i64> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }

    let c1 = convolution_mod::<MOD1>(a, b);
    let c2 = convolution_mod::<MOD2>(a, b);
    let c3 = convolution_mod::<MOD3>(a, b);

    let m1 = MOD1 as i128;
    let m2 = MOD2 as i128;
    let m3 = MOD3 as i128;
    let m = m1 * m2 * m3;

    // Garner: x ≡ c1 (mod m1), c2 (mod m2), c3 (mod m3)
    let m1_inv_m2 = inv_mod(m1 % m2, m2);
    let m1m2_inv_m3 = inv_mod(m1 * m2 % m3, m3);

    c1.iter()
        .zip(c2.iter())
        .zip(c3.iter())
        .map(|((&x1, &x2), &x3)| {
            let (x1, x2, x3) = (x1 as i128, x2 as i128, x3 as i128);
            let t1 = (x2 - x1).rem_euclid(m2) * m1_inv_m2 % m2;
            let x12 = x1 + t1 * m1;
            let t2 = (x3 - x12).rem_euclid(m3) * m1m2_inv_m3 % m3;
            let x123 = x12 + t2 * m1 * m2;
            // 負の値も扱えるよう対称な代表元に直す
            let v = if x123 > m / 2 { x123 - m } else { x123 };
            v as i64
        })
        .collect()
}

fn convolution_mod<const M: u32>(a: &[i64], b: &[i64]) -> Vec<u32> {
    let fa: Vec<ModInt<M>> = a.iter().map(|&x| ModInt::new(x)).collect();
    let fb: Vec<ModInt<M>> = b.iter().map(|&x| ModInt::new(x)).collect();
    convolution(&fa, &fb).into_iter().map(|x| x.val()).collect()
}

/// `a` の `m` を法とする逆元。`gcd(a, m) == 1` を仮定する。
fn inv_mod(a: i128, m: i128) -> i128 {
    let (mut s, mut t) = (m, a.rem_euclid(m));
    let (mut m0, mut m1) = (0i128, 1i128);
    while t != 0 {
        let u = s / t;
        s -= t * u;
        m0 -= m1 * u;
        std::mem::swap(&mut s, &mut t);
        std::mem::swap(&mut m0, &mut m1);
    }
    m0.rem_euclid(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Mi = modint::ModInt998244353;

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
    fn primitive_root_table_matches_computation() {
        // 表引きしている法と一般計算の結果が食い違わないこと
        for m in [998_244_353u32, 167_772_161, 469_762_049, 754_974_721] {
            let g = primitive_root(m);
            // g が原始根なら位数は m - 1
            assert_eq!(pow_mod(g as u64, (m - 1) as u64, m as u64), 1);
            let mut x = m - 1;
            let mut p = 2;
            while p as u64 * p as u64 <= x as u64 {
                if x % p == 0 {
                    assert_ne!(pow_mod(g as u64, ((m - 1) / p) as u64, m as u64), 1);
                    while x % p == 0 {
                        x /= p;
                    }
                }
                p += 1;
            }
            if x > 1 {
                assert_ne!(pow_mod(g as u64, ((m - 1) / x) as u64, m as u64), 1);
            }
        }
    }

    #[test]
    fn ntt_then_intt_is_identity() {
        let mut next = xorshift();
        for log in 0..10 {
            let n = 1usize << log;
            let a: Vec<Mi> = (0..n).map(|_| Mi::new(next())).collect();
            let mut b = a.clone();
            ntt(&mut b);
            intt(&mut b);
            assert_eq!(a, b, "n = {n}");
        }
    }

    #[test]
    fn convolution_matches_naive() {
        let mut next = xorshift();
        for n in [1usize, 2, 3, 7, 61, 64, 100, 129] {
            for m in [1usize, 2, 5, 61, 64, 130] {
                let a: Vec<Mi> = (0..n).map(|_| Mi::new(next())).collect();
                let b: Vec<Mi> = (0..m).map(|_| Mi::new(next())).collect();
                assert_eq!(
                    convolution(&a, &b),
                    convolution_naive(&a, &b),
                    "n = {n}, m = {m}"
                );
            }
        }
    }

    /// NTT 経路 (両方 60 より大きい) を必ず通る大きさで確認する。
    #[test]
    fn convolution_large_matches_naive() {
        let mut next = xorshift();
        let (n, m) = (300, 200);
        let a: Vec<Mi> = (0..n).map(|_| Mi::new(next())).collect();
        let b: Vec<Mi> = (0..m).map(|_| Mi::new(next())).collect();
        let expected = convolution_naive(&a, &b);
        let got = convolution(&a, &b);
        assert_eq!(got.len(), n + m - 1);
        assert_eq!(got, expected);
    }

    #[test]
    fn convolution_empty() {
        let a: Vec<Mi> = vec![];
        let b: Vec<Mi> = vec![Mi::new(1u32)];
        assert!(convolution(&a, &b).is_empty());
        assert!(convolution(&b, &a).is_empty());
    }

    #[test]
    fn convolution_known_value() {
        // (1 + 2x)(3 + 4x) = 3 + 10x + 8x^2
        let a: Vec<Mi> = [1u32, 2].iter().map(|&x| Mi::new(x)).collect();
        let b: Vec<Mi> = [3u32, 4].iter().map(|&x| Mi::new(x)).collect();
        let c = convolution(&a, &b);
        assert_eq!(
            c.iter().map(|x| x.val()).collect::<Vec<_>>(),
            vec![3, 10, 8]
        );
    }

    fn naive_i64(a: &[i64], b: &[i64]) -> Vec<i64> {
        if a.is_empty() || b.is_empty() {
            return vec![];
        }
        let mut ret = vec![0i128; a.len() + b.len() - 1];
        for (i, &x) in a.iter().enumerate() {
            for (j, &y) in b.iter().enumerate() {
                ret[i + j] += x as i128 * y as i128;
            }
        }
        ret.into_iter().map(|x| x as i64).collect()
    }

    #[test]
    fn convolution_i64_matches_naive() {
        let mut next = xorshift();
        for (n, m) in [(1usize, 1usize), (5, 7), (61, 3), (100, 100), (129, 200)] {
            let a: Vec<i64> = (0..n).map(|_| (next() % 2001) as i64 - 1000).collect();
            let b: Vec<i64> = (0..m).map(|_| (next() % 2001) as i64 - 1000).collect();
            assert_eq!(convolution_i64(&a, &b), naive_i64(&a, &b), "n={n} m={m}");
        }
    }

    /// 大きな値でも真の値が i64 に収まる限り正しい。
    #[test]
    fn convolution_i64_large_values() {
        let n = 100;
        let mut next = xorshift();
        // 各項 |x| < 2^31, 長さ 100 なら和は 2^69 未満... に収まるよう抑える
        let a: Vec<i64> = (0..n).map(|_| (next() % (1 << 20)) as i64).collect();
        let b: Vec<i64> = (0..n).map(|_| (next() % (1 << 20)) as i64).collect();
        assert_eq!(convolution_i64(&a, &b), naive_i64(&a, &b));

        // 負の値を含む場合
        let a: Vec<i64> = (0..n).map(|_| -((next() % (1 << 20)) as i64)).collect();
        let b: Vec<i64> = (0..n).map(|_| (next() % (1 << 20)) as i64).collect();
        assert_eq!(convolution_i64(&a, &b), naive_i64(&a, &b));
    }

    /// NTT-friendly でない法では落とす。
    #[test]
    #[should_panic(expected = "not NTT-friendly")]
    fn non_ntt_friendly_modulus_panics() {
        // 1000000007 - 1 = 2 * 500000003 なので長さ 4 すら取れない
        type Bad = ModInt<1_000_000_007>;
        let a: Vec<Bad> = (0..100).map(|i| Bad::new(i as u32)).collect();
        let b = a.clone();
        let _ = convolution(&a, &b);
    }
}
