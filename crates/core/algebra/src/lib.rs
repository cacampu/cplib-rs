use std::marker::PhantomData;

/// モノイド。単位元 `identity` と結合的な二項演算 `binary_op` を持つ。
///
/// メソッドが `&self` を取るため、実装型のインスタンスに mod の法や
/// 事前計算テーブルなど実行時に決まる定数を保持させて共有できる。
pub trait Monoid {
    type T: Clone;
    fn identity(&self) -> Self::T;
    fn binary_op(&self, a: &Self::T, b: &Self::T) -> Self::T;
}

/// 群。モノイドに逆元を加えたもの。累積和のように差を取る操作に使う。
pub trait Group: Monoid {
    /// `a ∘ b⁻¹` を返す。整数の加法なら `a - b`。
    ///
    /// 逆元を経由せず直接定義するのは、符号なし整数のように
    /// 逆元単体が表現できない型でも差は正しく計算できるため。
    fn inv_binary_op(&self, a: &Self::T, b: &Self::T) -> Self::T;

    /// `a` の逆元。
    fn inverse(&self, a: &Self::T) -> Self::T {
        self.inv_binary_op(&self.identity(), a)
    }
}

macro_rules! def_marker_monoid {
    ($($name:ident),*) => {$(
        pub struct $name<T>(PhantomData<fn() -> T>);

        impl<T> $name<T> {
            pub fn new() -> Self {
                Self(PhantomData)
            }
        }

        impl<T> Default for $name<T> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<T> Clone for $name<T> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<T> Copy for $name<T> {}
    )*};
}

def_marker_monoid!(Min, Max, MinMax, Sum, Prod, Xor, BitAnd, BitOr, Gcd, Lcm);

impl<T: Clone> MinMax<T> {
    /// 1要素を `(min, max)` の組に持ち上げる。
    #[inline]
    pub fn of(value: T) -> (T, T) {
        (value.clone(), value)
    }
}

macro_rules! impl_num_monoid {
    ($($t:ty),*) => {$(
        impl Monoid for Min<$t> {
            type T = $t;
            #[inline]
            fn identity(&self) -> $t { <$t>::MAX }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { (*a).min(*b) }
        }

        impl Monoid for Max<$t> {
            type T = $t;
            #[inline]
            fn identity(&self) -> $t { <$t>::MIN }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { (*a).max(*b) }
        }

        impl Monoid for Sum<$t> {
            type T = $t;
            #[inline]
            fn identity(&self) -> $t { 0 }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { *a + *b }
        }

        impl Monoid for Prod<$t> {
            type T = $t;
            #[inline]
            fn identity(&self) -> $t { 1 }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { *a * *b }
        }

        /// 区間の最小値と最大値を同時に持つ。要素は `MinMax::of` で作る。
        impl Monoid for MinMax<$t> {
            type T = ($t, $t);
            #[inline]
            fn identity(&self) -> ($t, $t) { (<$t>::MAX, <$t>::MIN) }
            #[inline]
            fn binary_op(&self, a: &($t, $t), b: &($t, $t)) -> ($t, $t) {
                (a.0.min(b.0), a.1.max(b.1))
            }
        }

        impl Monoid for Xor<$t> {
            type T = $t;
            #[inline]
            fn identity(&self) -> $t { 0 }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { *a ^ *b }
        }

        impl Monoid for BitAnd<$t> {
            type T = $t;
            #[inline]
            fn identity(&self) -> $t { !0 }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { *a & *b }
        }

        impl Monoid for BitOr<$t> {
            type T = $t;
            #[inline]
            fn identity(&self) -> $t { 0 }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { *a | *b }
        }

        impl Monoid for Gcd<$t> {
            type T = $t;
            /// gcd(0, x) = x なので単位元は 0。
            #[inline]
            fn identity(&self) -> $t { 0 }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { Gcd::<$t>::of(*a, *b) }
        }

        impl Monoid for Lcm<$t> {
            type T = $t;
            #[inline]
            fn identity(&self) -> $t { 1 }
            #[inline]
            fn binary_op(&self, a: &$t, b: &$t) -> $t { Lcm::<$t>::of(*a, *b) }
        }

        // 符号なし整数でも、差が数学的に非負なら wrapping で正しい値になる。
        // Prod は 0 が逆元を持たないので群にしない。
        impl Group for Sum<$t> {
            #[inline]
            fn inv_binary_op(&self, a: &$t, b: &$t) -> $t { a.wrapping_sub(*b) }
        }

        // xor は各元が自身の逆元。
        impl Group for Xor<$t> {
            #[inline]
            fn inv_binary_op(&self, a: &$t, b: &$t) -> $t { *a ^ *b }
        }
    )*};
}

/// 符号の有無で結果の正規化が違うので gcd / lcm は分けて実装する。
macro_rules! impl_gcd_lcm {
    // 符号付きは負になりうるので絶対値を取る
    (@norm signed $x:expr) => { ($x).wrapping_abs() };
    (@norm unsigned $x:expr) => { $x };
    ($sign:ident: $($t:ty),*) => {$(
        impl Gcd<$t> {
            /// ユークリッドの互除法。結果は非負。
            pub fn of(a: $t, b: $t) -> $t {
                let (mut a, mut b) = (a, b);
                while b != 0 {
                    let r = a % b;
                    a = b;
                    b = r;
                }
                impl_gcd_lcm!(@norm $sign a)
            }
        }

        impl Lcm<$t> {
            /// `lcm(0, x) = 0`。オーバーフローは呼び出し側の責任。
            pub fn of(a: $t, b: $t) -> $t {
                if a == 0 || b == 0 {
                    return 0;
                }
                let g = Gcd::<$t>::of(a, b);
                impl_gcd_lcm!(@norm $sign a / g * b)
            }
        }
    )*};
}

impl_gcd_lcm!(unsigned: usize, u8, u16, u32, u64, u128);
impl_gcd_lcm!(signed: isize, i8, i16, i32, i64, i128);

impl_num_monoid!(
    usize, isize, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128
);

/// クロージャからモノイドを作る。単位元と演算を実行時に決めたい場合に使う。
pub struct FnMonoid<T, F> {
    identity: T,
    op: F,
}

impl<T, F> FnMonoid<T, F>
where
    T: Clone,
    F: Fn(&T, &T) -> T,
{
    pub fn new(identity: T, op: F) -> Self {
        Self { identity, op }
    }
}

impl<T, F> Monoid for FnMonoid<T, F>
where
    T: Clone,
    F: Fn(&T, &T) -> T,
{
    type T = T;
    #[inline]
    fn identity(&self) -> T {
        self.identity.clone()
    }
    #[inline]
    fn binary_op(&self, a: &T, b: &T) -> T {
        (self.op)(a, b)
    }
}

/// クロージャから群を作る。`op` と `inv_op` の組で与える。
pub struct FnGroup<T, F, G> {
    identity: T,
    op: F,
    inv_op: G,
}

impl<T, F, G> FnGroup<T, F, G>
where
    T: Clone,
    F: Fn(&T, &T) -> T,
    G: Fn(&T, &T) -> T,
{
    pub fn new(identity: T, op: F, inv_op: G) -> Self {
        Self {
            identity,
            op,
            inv_op,
        }
    }
}

impl<T, F, G> Monoid for FnGroup<T, F, G>
where
    T: Clone,
    F: Fn(&T, &T) -> T,
    G: Fn(&T, &T) -> T,
{
    type T = T;
    #[inline]
    fn identity(&self) -> T {
        self.identity.clone()
    }
    #[inline]
    fn binary_op(&self, a: &T, b: &T) -> T {
        (self.op)(a, b)
    }
}

impl<T, F, G> Group for FnGroup<T, F, G>
where
    T: Clone,
    F: Fn(&T, &T) -> T,
    G: Fn(&T, &T) -> T,
{
    #[inline]
    fn inv_binary_op(&self, a: &T, b: &T) -> T {
        (self.inv_op)(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_monoids() {
        let min = Min::<i64>::new();
        assert_eq!(min.identity(), i64::MAX);
        assert_eq!(min.binary_op(&3, &5), 3);

        let max = Max::<i64>::new();
        assert_eq!(max.identity(), i64::MIN);
        assert_eq!(max.binary_op(&3, &5), 5);

        let sum = Sum::<i64>::new();
        assert_eq!(sum.identity(), 0);
        assert_eq!(sum.binary_op(&3, &5), 8);

        let prod = Prod::<i64>::new();
        assert_eq!(prod.identity(), 1);
        assert_eq!(prod.binary_op(&3, &5), 15);
    }

    #[test]
    fn minmax() {
        let m = MinMax::<i64>::new();
        assert_eq!(m.identity(), (i64::MAX, i64::MIN));
        assert_eq!(MinMax::of(3), (3, 3));
        let folded = [3, 1, 4, 1, 5]
            .into_iter()
            .map(MinMax::of)
            .fold(m.identity(), |acc, x| m.binary_op(&acc, &x));
        assert_eq!(folded, (1, 5));
    }

    #[test]
    fn bit_monoids() {
        let x = Xor::<u64>::new();
        assert_eq!(x.identity(), 0);
        assert_eq!(x.binary_op(&0b1100, &0b1010), 0b0110);

        let a = BitAnd::<u64>::new();
        assert_eq!(a.identity(), u64::MAX);
        assert_eq!(a.binary_op(&0b1100, &0b1010), 0b1000);

        let o = BitOr::<u64>::new();
        assert_eq!(o.identity(), 0);
        assert_eq!(o.binary_op(&0b1100, &0b1010), 0b1110);

        // 符号付きの and の単位元は全ビット1 = -1
        assert_eq!(BitAnd::<i64>::new().identity(), -1);
    }

    #[test]
    fn xor_is_a_group() {
        let g = Xor::<u64>::new();
        // 各元が自身の逆元
        assert_eq!(g.inverse(&0b1011), 0b1011);
        assert_eq!(g.inv_binary_op(&0b0110, &0b1010), 0b1100);
        assert_eq!(g.binary_op(&g.inv_binary_op(&7, &5), &5), 7);
    }

    #[test]
    fn gcd_lcm() {
        let g = Gcd::<u64>::new();
        assert_eq!(g.identity(), 0);
        assert_eq!(g.binary_op(&12, &18), 6);
        // 単位元との演算は恒等
        assert_eq!(g.binary_op(&0, &7), 7);
        assert_eq!(g.binary_op(&7, &0), 7);

        let l = Lcm::<u64>::new();
        assert_eq!(l.identity(), 1);
        assert_eq!(l.binary_op(&4, &6), 12);
        assert_eq!(l.binary_op(&1, &7), 7);
        // 0 は吸収元
        assert_eq!(l.binary_op(&0, &7), 0);

        // 符号付きは絶対値で返す
        assert_eq!(Gcd::<i64>::of(-12, 18), 6);
        assert_eq!(Gcd::<i64>::of(-12, 0), 12);
        assert_eq!(Lcm::<i64>::of(-4, 6), 12);
    }

    /// 結合則をランダムに確認する。
    #[test]
    fn associativity() {
        let mut state = 88172645463325252u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % 1000
        };
        macro_rules! check {
            ($m:expr) => {{
                let m = $m;
                for _ in 0..200 {
                    let (a, b, c) = (next(), next(), next());
                    assert_eq!(
                        m.binary_op(&m.binary_op(&a, &b), &c),
                        m.binary_op(&a, &m.binary_op(&b, &c))
                    );
                    // 単位元則
                    assert_eq!(m.binary_op(&m.identity(), &a), a);
                    assert_eq!(m.binary_op(&a, &m.identity()), a);
                }
            }};
        }
        check!(Min::<u64>::new());
        check!(Max::<u64>::new());
        check!(Sum::<u64>::new());
        check!(Xor::<u64>::new());
        check!(BitAnd::<u64>::new());
        check!(BitOr::<u64>::new());
        check!(Gcd::<u64>::new());
    }

    /// `PrefixSum::from_slice` などが要求する `Group + Default` を満たす型。
    #[test]
    fn group_and_default_impls() {
        fn assert_group_default<G: Group + Default>() {}
        assert_group_default::<Sum<i64>>();
        assert_group_default::<Sum<usize>>();
        assert_group_default::<Xor<u64>>();
        assert_group_default::<Xor<i64>>();
    }

    #[test]
    fn sum_group() {
        let g = Sum::<i64>::new();
        assert_eq!(g.inv_binary_op(&3, &5), -2);
        assert_eq!(g.inverse(&5), -5);
        // 符号なしでも、差が非負なら正しい
        let u = Sum::<usize>::new();
        assert_eq!(u.inv_binary_op(&10, &4), 6);
        assert_eq!(u.binary_op(&u.inv_binary_op(&10, &4), &4), 10);
    }

    #[test]
    fn fn_group_captures_constant() {
        let modulo = 998_244_353i64;
        let g = FnGroup::new(
            0,
            move |a: &i64, b: &i64| (a + b) % modulo,
            move |a: &i64, b: &i64| (a - b).rem_euclid(modulo),
        );
        assert_eq!(g.binary_op(&(modulo - 1), &2), 1);
        assert_eq!(g.inv_binary_op(&1, &2), modulo - 1);
        assert_eq!(g.inverse(&2), modulo - 2);
    }

    #[test]
    fn fn_monoid_captures_constant() {
        let modulo = 998_244_353u64;
        let m = FnMonoid::new(1, move |a: &u64, b: &u64| a * b % modulo);
        assert_eq!(m.identity(), 1);
        assert_eq!(m.binary_op(&(modulo - 1), &2), modulo - 2);
    }
}
