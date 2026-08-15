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

def_marker_monoid!(Min, Max, Sum, Prod);

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
    )*};
}

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
    fn fn_monoid_captures_constant() {
        let modulo = 998_244_353u64;
        let m = FnMonoid::new(1, move |a: &u64, b: &u64| a * b % modulo);
        assert_eq!(m.identity(), 1);
        assert_eq!(m.binary_op(&(modulo - 1), &2), modulo - 2);
    }
}
