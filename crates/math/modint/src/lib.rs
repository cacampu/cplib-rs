//! 剰余環 Z/mZ の元。
//!
//! アルゴリズムは AtCoder Library (ac-library-rs, CC0-1.0) を参考にしているが、
//! 法をマーカー型 + トレイトではなく const generics で持たせている。
//! そのため `ModInt<1_000_000_009>` のように任意の法をその場で書ける。
//!
//! この型自体はモノイドではない。どの演算を二項演算とみなすかは型では決まらないので、
//! `algebra::Sum<ModInt<M>>` や `algebra::Prod<ModInt<M>>` として表す。
//! `Add` / `Mul` と `Zero` / `One` を実装しているため、algebra 側の blanket impl が
//! そのまま適用され、SegTree などに追加の実装なしで載る。
//!
//! プリミティブ整数型 (`u8`..`u128`, `i8`..`i128`, `usize`, `isize`) との四則演算も
//! 左右どちらの順でも書ける (`x * 2`, `2 - x`, `x /= 3` など)。整数側は剰余を取ってから
//! 演算するので、負数や法より大きい値でも正しい。
//! ただし整数型ごとに impl を並べている都合上、演算結果に直接メソッドを呼ぶ
//! `(x + 5).val()` のような式ではリテラルの型が決まらずエラーになる。
//! `(x + 5u32).val()` のように接尾辞を付けるか、いったん束縛すればよい。

use std::cell::Cell;
use std::fmt;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use algebra::{One, Zero};

/// 法 `M` の剰余環の元。`M` は `1 <= M < 2^31` であること。
///
/// 加算で `u32` が溢れないよう上限を `2^31` に制限している。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct ModInt<const M: u32> {
    val: u32,
}

pub type ModInt998244353 = ModInt<998_244_353>;
pub type ModInt1000000007 = ModInt<1_000_000_007>;

impl<const M: u32> ModInt<M> {
    /// `M` の妥当性をコンパイル時に検査する。
    const ASSERT_MOD: () = assert!(M >= 1 && M < (1 << 31), "modulus must be in [1, 2^31)");

    /// `0 <= value < M` を満たす値から剰余を取らずに構築する。
    #[inline]
    pub fn raw(value: u32) -> Self {
        () = Self::ASSERT_MOD;
        debug_assert!(value < M, "raw value must be less than the modulus");
        Self { val: value }
    }

    #[inline]
    pub fn val(self) -> u32 {
        self.val
    }

    #[inline]
    pub fn modulus() -> u32 {
        M
    }
}

impl<const M: u32> Add for ModInt<M> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        let mut val = self.val + rhs.val;
        if val >= M {
            val -= M;
        }
        Self::raw(val)
    }
}

impl<const M: u32> Sub for ModInt<M> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        let mut val = self.val.wrapping_sub(rhs.val);
        if val >= M {
            // 借りが出た場合はラップアラウンドしているので戻す
            val = val.wrapping_add(M);
        }
        Self::raw(val)
    }
}

impl<const M: u32> Mul for ModInt<M> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::raw((self.val as u64 * rhs.val as u64 % M as u64) as u32)
    }
}

/// 実行時に法を設定できる modint。`ID` が違えば別の法を同時に扱える。
///
/// 法は `set_modulus` を呼ぶまで未設定で、未設定のまま演算するとパニックする
/// (黙って誤った値を返さない)。法はスレッドごとに保持される。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct DynamicModInt<const ID: usize = 0> {
    val: u32,
}

/// 法をひとつだけ使う場合のエイリアス。
pub type DynModInt = DynamicModInt<0>;

/// 同時に使える法の個数。
pub const MAX_DYNAMIC_IDS: usize = 16;

thread_local! {
    static BARRETTS: [Cell<Barrett>; MAX_DYNAMIC_IDS] =
        const { [const { Cell::new(Barrett::UNSET) }; MAX_DYNAMIC_IDS] };
}

impl<const ID: usize> DynamicModInt<ID> {
    const ASSERT_ID: () = assert!(ID < MAX_DYNAMIC_IDS, "ID must be less than MAX_DYNAMIC_IDS");

    /// 法を設定する。既に作った値は古い法のままなので、使い始める前に呼ぶこと。
    pub fn set_modulus(m: u32) {
        () = Self::ASSERT_ID;
        assert!(
            (1..(1 << 31)).contains(&m),
            "modulus must be in [1, 2^31), got {m}"
        );
        BARRETTS.with(|bs| bs[ID].set(Barrett::new(m)));
    }

    #[inline]
    fn barrett() -> Barrett {
        () = Self::ASSERT_ID;
        let b = BARRETTS.with(|bs| bs[ID].get());
        assert!(
            b.m != 0,
            "modulus of DynamicModInt<{ID}> is not set; call set_modulus first"
        );
        b
    }

    #[inline]
    pub fn raw(value: u32) -> Self {
        debug_assert!(value < Self::modulus());
        Self { val: value }
    }

    #[inline]
    pub fn val(self) -> u32 {
        self.val
    }

    #[inline]
    pub fn modulus() -> u32 {
        Self::barrett().m
    }
}

impl<const ID: usize> Add for DynamicModInt<ID> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        let m = Self::modulus();
        let mut val = self.val + rhs.val;
        if val >= m {
            val -= m;
        }
        Self { val }
    }
}

impl<const ID: usize> Sub for DynamicModInt<ID> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        let m = Self::modulus();
        let mut val = self.val.wrapping_sub(rhs.val);
        if val >= m {
            val = val.wrapping_add(m);
        }
        Self { val }
    }
}

impl<const ID: usize> Mul for DynamicModInt<ID> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            val: Self::barrett().mul(self.val, rhs.val),
        }
    }
}

/// 法が固定なら `%` より速い剰余乗算。ACL の barrett reduction。
#[derive(Clone, Copy)]
struct Barrett {
    m: u32,
    im: u64,
}

impl Barrett {
    /// `m == 0` は「未設定」を表す。
    const UNSET: Self = Self { m: 0, im: 0 };

    fn new(m: u32) -> Self {
        Self {
            m,
            im: (!0u64 / m as u64).wrapping_add(1),
        }
    }

    /// `a * b % m`。`a, b < m < 2^31` を仮定する。
    #[inline]
    fn mul(&self, a: u32, b: u32) -> u32 {
        let z = a as u64 * b as u64;
        // z / m の近似を上位64bitとして取り出す
        let x = ((z as u128 * self.im as u128) >> 64) as u64;
        let v = z.wrapping_sub(x.wrapping_mul(self.m as u64)) as u32;
        // 近似が1だけ大きいことがあるので補正する
        if v >= self.m {
            v.wrapping_add(self.m)
        } else {
            v
        }
    }
}

/// 法の持ち方が違うだけで共通に書ける部分をまとめて生成する。
macro_rules! impl_modint_common {
    ($ty:ident, $param:ident, $ptype:ty) => {
        impl<const $param: $ptype> $ty<$param> {
            /// 剰余を取ってから構築する。負の値も受け付ける。
            #[inline]
            pub fn new<T: RemEuclidU32>(value: T) -> Self {
                Self::raw(value.rem_euclid_u32(Self::modulus()))
            }

            /// `self^n`。繰り返し二乗法で `O(log n)`。
            pub fn pow(self, mut n: u64) -> Self {
                let mut ret = Self::new(1u32);
                let mut base = self;
                while n > 0 {
                    if n & 1 == 1 {
                        ret *= base;
                    }
                    base *= base;
                    n >>= 1;
                }
                ret
            }

            /// 平方根のひとつ。存在しなければ `None`。
            /// **法が素数であることを仮定する** (Tonelli-Shanks)。
            ///
            /// 解が2つある場合にどちらを返すかは決めていない。
            /// もう一方は `-r` で得られる。
            pub fn sqrt(self) -> Option<Self> {
                let m = Self::modulus();
                if m == 2 || self.val <= 1 {
                    return Some(self);
                }
                let one = Self::new(1u32);
                // オイラーの規準。平方非剰余なら解なし
                if self.pow(((m - 1) / 2) as u64) != one {
                    return None;
                }
                // m - 1 = q * 2^s (q は奇数)
                let mut q = m - 1;
                let mut s = 0u32;
                while q % 2 == 0 {
                    q /= 2;
                    s += 1;
                }
                if s == 1 {
                    return Some(self.pow(((m + 1) / 4) as u64));
                }
                // 平方非剰余をひとつ見つける
                let mut z = Self::new(2u32);
                while z.pow(((m - 1) / 2) as u64) == one {
                    z += one;
                }
                let mut level = s;
                let mut c = z.pow(q as u64);
                let mut t = self.pow(q as u64);
                let mut r = self.pow(q.div_ceil(2) as u64);
                while t != one {
                    // t^(2^i) = 1 となる最小の i を探す
                    let mut i = 0u32;
                    let mut t2 = t;
                    while t2 != one {
                        t2 *= t2;
                        i += 1;
                    }
                    let b = c.pow(1u64 << (level - i - 1));
                    level = i;
                    c = b * b;
                    t *= c;
                    r *= b;
                }
                Some(r)
            }

            /// 乗法逆元。法と互いに素でないとパニックする。
            /// 法が素数でなくても互いに素なら求まる。
            pub fn inv(self) -> Self {
                let m = Self::modulus();
                let (g, x) = inv_gcd(self.val as i64, m as i64);
                assert_eq!(g, 1, "{} is not invertible modulo {}", self.val, m);
                Self::raw(x as u32)
            }
        }

        impl<const $param: $ptype> Zero for $ty<$param> {
            #[inline]
            fn zero() -> Self {
                Self::raw(0)
            }
        }

        impl<const $param: $ptype> One for $ty<$param> {
            #[inline]
            fn one() -> Self {
                Self::new(1u32)
            }
        }

        impl<const $param: $ptype> Neg for $ty<$param> {
            type Output = Self;
            #[inline]
            fn neg(self) -> Self {
                Self::raw(0) - self
            }
        }

        #[allow(clippy::suspicious_arithmetic_impl)]
        impl<const $param: $ptype> Div for $ty<$param> {
            type Output = Self;
            #[inline]
            fn div(self, rhs: Self) -> Self {
                self * rhs.inv()
            }
        }

        impl<const $param: $ptype> fmt::Display for $ty<$param> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.val.fmt(f)
            }
        }

        impl<const $param: $ptype> fmt::Debug for $ty<$param> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.val.fmt(f)
            }
        }

        impl<const $param: $ptype> Sum for $ty<$param> {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::raw(0), |a, b| a + b)
            }
        }

        impl<'a, const $param: $ptype> Sum<&'a Self> for $ty<$param> {
            fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                iter.fold(Self::raw(0), |a, b| a + *b)
            }
        }

        impl<const $param: $ptype> Product for $ty<$param> {
            fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::new(1u32), |a, b| a * b)
            }
        }

        impl<'a, const $param: $ptype> Product<&'a Self> for $ty<$param> {
            fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                iter.fold(Self::new(1u32), |a, b| a * *b)
            }
        }

        impl_modint_common!(@assign $ty, $param, $ptype, AddAssign, add_assign, +);
        impl_modint_common!(@assign $ty, $param, $ptype, SubAssign, sub_assign, -);
        impl_modint_common!(@assign $ty, $param, $ptype, MulAssign, mul_assign, *);
        impl_modint_common!(@assign $ty, $param, $ptype, DivAssign, div_assign, /);

        impl_modint_common!(@ref $ty, $param, $ptype, Add, add);
        impl_modint_common!(@ref $ty, $param, $ptype, Sub, sub);
        impl_modint_common!(@ref $ty, $param, $ptype, Mul, mul);
        impl_modint_common!(@ref $ty, $param, $ptype, Div, div);

        impl_modint_common!(@from $ty, $param, $ptype,
            u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

        impl_modint_common!(@scalar $ty, $param, $ptype,
            u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    };

    (@assign $ty:ident, $param:ident, $ptype:ty, $trait:ident, $method:ident, $op:tt) => {
        impl<const $param: $ptype> $trait for $ty<$param> {
            #[inline]
            fn $method(&mut self, rhs: Self) {
                *self = *self $op rhs;
            }
        }
    };

    (@ref $ty:ident, $param:ident, $ptype:ty, $trait:ident, $method:ident) => {
        impl<const $param: $ptype> $trait<&$ty<$param>> for $ty<$param> {
            type Output = $ty<$param>;
            #[inline]
            fn $method(self, rhs: &$ty<$param>) -> $ty<$param> {
                $trait::$method(self, *rhs)
            }
        }

        impl<const $param: $ptype> $trait<$ty<$param>> for &$ty<$param> {
            type Output = $ty<$param>;
            #[inline]
            fn $method(self, rhs: $ty<$param>) -> $ty<$param> {
                $trait::$method(*self, rhs)
            }
        }

        impl<const $param: $ptype> $trait<&$ty<$param>> for &$ty<$param> {
            type Output = $ty<$param>;
            #[inline]
            fn $method(self, rhs: &$ty<$param>) -> $ty<$param> {
                $trait::$method(*self, *rhs)
            }
        }
    };

    (@from $ty:ident, $param:ident, $ptype:ty, $($t:ty),*) => {$(
        impl<const $param: $ptype> From<$t> for $ty<$param> {
            #[inline]
            fn from(value: $t) -> Self {
                Self::new(value)
            }
        }
    )*};

    (@scalar $ty:ident, $param:ident, $ptype:ty, $($t:ty),*) => {$(
        impl_modint_common!(@scalar_ops $ty, $param, $ptype, $t, Add, add, AddAssign, add_assign);
        impl_modint_common!(@scalar_ops $ty, $param, $ptype, $t, Sub, sub, SubAssign, sub_assign);
        impl_modint_common!(@scalar_ops $ty, $param, $ptype, $t, Mul, mul, MulAssign, mul_assign);
        impl_modint_common!(@scalar_ops $ty, $param, $ptype, $t, Div, div, DivAssign, div_assign);
    )*};

    // 整数側は `new` で剰余を取ってから通常の演算に落とす。
    // 整数を左辺に置く形 (`2 * x`) は孤児則の範囲内 (右辺がローカル型) なので書ける。
    (@scalar_ops $ty:ident, $param:ident, $ptype:ty, $t:ty,
     $trait:ident, $method:ident, $atrait:ident, $amethod:ident) => {
        impl<const $param: $ptype> $trait<$t> for $ty<$param> {
            type Output = $ty<$param>;
            #[inline]
            fn $method(self, rhs: $t) -> $ty<$param> {
                $trait::$method(self, <$ty<$param>>::new(rhs))
            }
        }

        impl<const $param: $ptype> $trait<$t> for &$ty<$param> {
            type Output = $ty<$param>;
            #[inline]
            fn $method(self, rhs: $t) -> $ty<$param> {
                $trait::$method(*self, <$ty<$param>>::new(rhs))
            }
        }

        impl<const $param: $ptype> $trait<$ty<$param>> for $t {
            type Output = $ty<$param>;
            #[inline]
            fn $method(self, rhs: $ty<$param>) -> $ty<$param> {
                $trait::$method(<$ty<$param>>::new(self), rhs)
            }
        }

        impl<const $param: $ptype> $trait<&$ty<$param>> for $t {
            type Output = $ty<$param>;
            #[inline]
            fn $method(self, rhs: &$ty<$param>) -> $ty<$param> {
                $trait::$method(<$ty<$param>>::new(self), *rhs)
            }
        }

        impl<const $param: $ptype> $atrait<$t> for $ty<$param> {
            #[inline]
            fn $amethod(&mut self, rhs: $t) {
                *self = $trait::$method(*self, <$ty<$param>>::new(rhs));
            }
        }
    };
}

impl_modint_common!(ModInt, M, u32);
impl_modint_common!(DynamicModInt, ID, usize);

/// 各整数型から法 `m` での剰余を取るためのトレイト。
pub trait RemEuclidU32 {
    fn rem_euclid_u32(self, m: u32) -> u32;
}

macro_rules! impl_rem_euclid {
    (unsigned: $($t:ty),*) => {$(
        impl RemEuclidU32 for $t {
            #[inline]
            fn rem_euclid_u32(self, m: u32) -> u32 {
                (self as u128 % m as u128) as u32
            }
        }
    )*};
    (signed: $($t:ty),*) => {$(
        impl RemEuclidU32 for $t {
            #[inline]
            fn rem_euclid_u32(self, m: u32) -> u32 {
                (self as i128).rem_euclid(m as i128) as u32
            }
        }
    )*};
}

impl_rem_euclid!(unsigned: u8, u16, u32, u64, u128, usize);
impl_rem_euclid!(signed: i8, i16, i32, i64, i128, isize);

/// 拡張ユークリッドの互除法。`(gcd(a, b), x)` を返す。
/// `x` は `a * x ≡ gcd(a, b) (mod b)` を満たす `0 <= x < b / gcd` の値。
pub fn inv_gcd(a: i64, b: i64) -> (i64, i64) {
    let a = a.rem_euclid(b);
    if a == 0 {
        return (b, 0);
    }
    let (mut s, mut t) = (b, a);
    let (mut m0, mut m1) = (0i64, 1i64);
    while t != 0 {
        let u = s / t;
        s -= t * u;
        m0 -= m1 * u;
        std::mem::swap(&mut s, &mut t);
        std::mem::swap(&mut m0, &mut m1);
    }
    if m0 < 0 {
        m0 += b / s;
    }
    (s, m0)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Mi = ModInt998244353;
    const P: u64 = 998_244_353;

    fn xorshift() -> impl FnMut() -> u64 {
        let mut state = 88172645463325252u64;
        move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        }
    }

    /// 素朴な u64 演算と一致するか。
    #[test]
    fn ops_match_naive() {
        let mut next = xorshift();
        for _ in 0..2000 {
            let (a, b) = (next() % P, next() % P);
            let (ma, mb) = (Mi::new(a), Mi::new(b));
            assert_eq!((ma + mb).val() as u64, (a + b) % P);
            assert_eq!((ma - mb).val() as u64, (a + P - b) % P);
            assert_eq!((ma * mb).val() as u64, a * b % P);
            assert_eq!((-ma).val() as u64, (P - a) % P);
        }
    }

    #[test]
    fn inv_and_div() {
        for x in 1..100u32 {
            let m = Mi::new(x);
            assert_eq!((m * m.inv()).val(), 1);
            assert_eq!((m / m).val(), 1);
        }
        // 合成数の法でも互いに素なら逆元が求まる
        type M10 = ModInt<10>;
        assert_eq!((M10::new(3u32) * M10::new(3u32).inv()).val(), 1);
    }

    #[test]
    #[should_panic]
    fn inv_of_non_coprime_panics() {
        ModInt::<10>::new(2u32).inv();
    }

    #[test]
    fn pow_matches_naive() {
        let base = Mi::new(3u32);
        let mut expected = Mi::new(1u32);
        for e in 0..200u64 {
            assert_eq!(base.pow(e), expected);
            expected *= base;
        }
        // フェルマーの小定理
        assert_eq!(Mi::new(12345u32).pow(P - 1).val(), 1);
    }

    #[test]
    fn negative_and_large_inputs() {
        assert_eq!(Mi::new(-1i64).val() as u64, P - 1);
        assert_eq!(Mi::new(-(P as i64) - 5).val() as u64, P - 5);
        assert_eq!(Mi::new(u64::MAX).val() as u64, u64::MAX % P);
        assert_eq!(Mi::from(-3i32), -Mi::new(3u32));
    }

    #[test]
    fn sum_and_product_iterators() {
        let xs: Vec<Mi> = (1..=10u32).map(Mi::new).collect();
        assert_eq!(xs.iter().copied().sum::<Mi>().val(), 55);
        assert_eq!(xs.iter().sum::<Mi>().val(), 55);
        assert_eq!(xs.iter().copied().product::<Mi>().val(), 3628800);
        assert_eq!(xs.iter().product::<Mi>().val(), 3628800);
    }

    /// 参照どうしの演算が書けることの確認なので op_ref は意図的。
    #[test]
    #[allow(clippy::op_ref)]
    fn assign_and_ref_ops() {
        let mut a = Mi::new(10u32);
        a += Mi::new(5u32);
        assert_eq!(a.val(), 15);
        a -= Mi::new(3u32);
        assert_eq!(a.val(), 12);
        a *= Mi::new(2u32);
        assert_eq!(a.val(), 24);
        a /= Mi::new(4u32);
        assert_eq!(a.val(), 6);
        let b = Mi::new(2u32);
        assert_eq!((&a + &b).val(), 8);
        assert_eq!((a + &b).val(), 8);
        assert_eq!((&a * b).val(), 12);
    }

    /// 整数リテラル・整数型との混合演算。左右どちらに置いても書ける。
    #[test]
    #[allow(clippy::op_ref)]
    fn ops_with_primitive_integers() {
        let a = Mi::new(10u32);
        assert_eq!(a + 5, Mi::new(15u32));
        assert_eq!(5 + a, Mi::new(15u32));
        assert_eq!(a - 15, Mi::new(-5i64));
        assert_eq!(5 - a, Mi::new(-5i64));
        assert_eq!(a * 3, Mi::new(30u32));
        assert_eq!(3 * a, Mi::new(30u32));
        assert_eq!(a / 4, Mi::new(10u32) * Mi::new(4u32).inv());
        assert_eq!(4 / a, Mi::new(4u32) * a.inv());

        // 参照どうし・整数型のバリエーション
        assert_eq!(&a + 5u64, Mi::new(15u32));
        assert_eq!(5usize + &a, Mi::new(15u32));
        assert_eq!(a + (-1i32), Mi::new(9u32));
        assert_eq!(a * u64::MAX, a * Mi::new(u64::MAX));

        // 複合代入
        let mut b = Mi::new(1u32);
        b += 10;
        assert_eq!(b.val(), 11);
        b -= 3u8;
        assert_eq!(b.val(), 8);
        b *= 2i64;
        assert_eq!(b.val(), 16);
        b /= 4;
        assert_eq!(b.val(), 4);
    }

    #[test]
    fn dynamic_ops_with_primitive_integers() {
        type D = DynamicModInt<6>;
        D::set_modulus(13);
        let a = D::new(10u32);
        // 演算結果に直接メソッドを呼ぶ場合はリテラルに型が必要 (モジュールドキュメント参照)
        assert_eq!((a + 5u32).val(), 2);
        assert_eq!((5u32 + a).val(), 2);
        assert_eq!((5u32 - a).val(), 8);
        assert_eq!((3u32 * a).val(), 30 % 13);
        assert_eq!(a + 5, D::new(2u32));
        let mut b = a;
        b *= 2;
        assert_eq!(b.val(), 20 % 13);
    }

    #[test]
    fn modulus_one_is_degenerate() {
        type M1 = ModInt<1>;
        assert_eq!(M1::new(5u32).val(), 0);
        assert_eq!(M1::new(5u32).pow(3).val(), 0);
        assert_eq!((M1::new(1u32) + M1::new(1u32)).val(), 0);
    }

    #[test]
    fn inv_gcd_basic() {
        assert_eq!(inv_gcd(0, 7), (7, 0));
        assert_eq!(inv_gcd(3, 7).0, 1);
        assert_eq!(inv_gcd(4, 6).0, 2);
        for b in 2..50i64 {
            for a in 0..50i64 {
                let (g, x) = inv_gcd(a, b);
                assert_eq!(g, gcd(a.rem_euclid(b), b));
                assert_eq!((a.rem_euclid(b) * x - g).rem_euclid(b), 0, "a={a} b={b}");
            }
        }
    }

    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }

    #[test]
    fn sqrt_matches_square() {
        // 平方剰余なら r^2 == x、非剰余なら None
        let mut found = 0;
        for x in 0..200u32 {
            let v = Mi::new(x);
            match v.sqrt() {
                Some(r) => {
                    assert_eq!(r * r, v, "x = {x}");
                    found += 1;
                }
                None => {
                    // 非剰余であることをオイラーの規準で確認
                    assert_ne!(v.pow((P - 1) / 2), Mi::new(1u32), "x = {x}");
                }
            }
        }
        assert!(found > 50, "平方剰余が少なすぎる: {found}");
    }

    #[test]
    fn sqrt_on_small_primes() {
        // s = 1 の分岐 (p ≡ 3 mod 4)
        type M7 = ModInt<7>;
        for x in 0..7u32 {
            let v = M7::new(x);
            if let Some(r) = v.sqrt() {
                assert_eq!(r * r, v, "x = {x} mod 7");
            }
        }
        // s > 1 の分岐 (p ≡ 1 mod 8)
        type M17 = ModInt<17>;
        for x in 0..17u32 {
            let v = M17::new(x);
            if let Some(r) = v.sqrt() {
                assert_eq!(r * r, v, "x = {x} mod 17");
            }
        }
        assert!(M7::new(3u32).sqrt().is_none());
    }

    /// modint 自体はモノイドではなく、`Sum` / `Prod` で包んで使う。
    /// algebra 側は blanket impl なので modint に Monoid の実装は無い。
    #[test]
    fn works_with_algebra_wrappers() {
        use algebra::{Group, Monoid, Prod};

        let s = algebra::Sum::<Mi>::new();
        assert_eq!(s.identity().val(), 0);
        assert_eq!(s.binary_op(&Mi::new(3u32), &Mi::new(4u32)).val(), 7);
        assert_eq!(
            s.inv_binary_op(&Mi::new(3u32), &Mi::new(4u32)),
            Mi::new(-1i64)
        );
        assert_eq!(s.inverse(&Mi::new(3u32)), Mi::new(-3i64));

        let p = Prod::<Mi>::new();
        assert_eq!(p.identity().val(), 1);
        assert_eq!(p.binary_op(&Mi::new(3u32), &Mi::new(4u32)).val(), 12);
    }

    // --- DynamicModInt ---

    /// 他のテストと ID が衝突しないよう、ID ごとに使い道を固定する。
    #[test]
    fn dynamic_basic_ops() {
        type D = DynamicModInt<1>;
        D::set_modulus(13);
        assert_eq!(D::modulus(), 13);
        assert_eq!((D::new(7u32) + D::new(9u32)).val(), 3);
        assert_eq!((D::new(3u32) - D::new(9u32)).val(), 7);
        assert_eq!((D::new(5u32) * D::new(6u32)).val(), 4);
        assert_eq!((D::new(5u32) / D::new(5u32)).val(), 1);
        assert_eq!((-D::new(5u32)).val(), 8);
        assert_eq!(D::new(2u32).pow(10).val(), 1024 % 13);
        assert_eq!(D::new(-1i64).val(), 12);
    }

    /// ID が違えば別の法を同時に使える。
    #[test]
    fn dynamic_ids_are_independent() {
        type A = DynamicModInt<2>;
        type B = DynamicModInt<3>;
        A::set_modulus(7);
        B::set_modulus(11);
        assert_eq!(A::modulus(), 7);
        assert_eq!(B::modulus(), 11);
        assert_eq!((A::new(5u32) + A::new(5u32)).val(), 3);
        assert_eq!((B::new(5u32) + B::new(5u32)).val(), 10);
        // 片方を書き換えてももう片方は影響を受けない
        A::set_modulus(5);
        assert_eq!(A::modulus(), 5);
        assert_eq!(B::modulus(), 11);
    }

    #[test]
    #[should_panic(expected = "is not set")]
    fn dynamic_without_modulus_panics() {
        DynamicModInt::<15>::new(1u32);
    }

    /// barrett reduction が素朴な剰余と一致するか。
    #[test]
    fn dynamic_matches_naive() {
        type D = DynamicModInt<4>;
        let mut next = xorshift();
        for m in [1u32, 2, 3, 998_244_353, 1_000_000_007, (1 << 31) - 1] {
            D::set_modulus(m);
            for _ in 0..500 {
                let (a, b) = ((next() % m as u64) as u32, (next() % m as u64) as u32);
                let (da, db) = (D::raw(a), D::raw(b));
                assert_eq!(
                    (da * db).val() as u64,
                    a as u64 * b as u64 % m as u64,
                    "m={m} a={a} b={b}"
                );
                assert_eq!((da + db).val() as u64, (a as u64 + b as u64) % m as u64);
            }
        }
    }

    /// DynamicModInt も Zero / One を実装しているので algebra に載る。
    #[test]
    fn dynamic_works_with_algebra_wrappers() {
        use algebra::Monoid;
        type D = DynamicModInt<5>;
        D::set_modulus(97);
        let s = algebra::Sum::<D>::new();
        assert_eq!(s.identity().val(), 0);
        assert_eq!(s.binary_op(&D::new(90u32), &D::new(20u32)).val(), 13);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use algebra::{Prod, Sum as SumOf};
    use prefix_sum::PrefixSum;
    use segtree::SegTree;

    type Mi = ModInt998244353;

    /// modint 側に Monoid の実装を書かずに SegTree へ載る。
    #[test]
    fn segtree_over_modint() {
        let a: Vec<Mi> = (1..=8u32).map(Mi::new).collect();
        let tree = SegTree::from_vec(a.clone(), Prod::<Mi>::new());
        for l in 0..a.len() {
            for r in l..=a.len() {
                let expected: Mi = a[l..r].iter().copied().product();
                assert_eq!(tree.prod(l..r), expected, "{l}..{r}");
            }
        }
        assert_eq!(tree.all_prod().val(), 40320);
    }

    /// Sum は Sub も実装しているので Group になり、累積和も使える。
    #[test]
    fn prefix_sum_over_modint() {
        let a: Vec<Mi> = (1..=10u32).map(Mi::new).collect();
        let ps = PrefixSum::new(a.clone(), SumOf::<Mi>::new());
        for l in 0..=a.len() {
            for r in l..=a.len() {
                let expected: Mi = a[l..r].iter().copied().sum();
                assert_eq!(ps.prod(l..r), expected, "{l}..{r}");
            }
        }
        assert_eq!(ps.all_prod().val(), 55);
    }
}
