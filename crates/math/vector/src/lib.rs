use prim::IntN;
use std::{
    array,
    cmp::Ordering,
    iter::Sum,
    ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vector<T, const N: usize = 2>(pub [T; N]);

/// broadcast演算のためのスカラーラッパー。
/// `Vector<T> op Scalar<S>` は `T op S` が定義されていれば自動的に実装される。
/// 例: `Vector<ModInt> * Scalar(3i64)` は `ModInt: Mul<i64, Output=ModInt>` があれば動く。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scalar<T>(pub T);

impl<T, const N: usize> Deref for Vector<T, N> {
    type Target = [T; N];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T, const N: usize> DerefMut for Vector<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Vector<T> op Vector<S> → Vector<T::Output> (zip、owned/ref の4組合せ)
macro_rules! impl_binop {
    ($($Trait:ident :: $method:ident),* $(,)?) => {$(
        impl<T, S, const N: usize> $Trait<Vector<S, N>> for Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            type Output = Vector<<T as $Trait<S>>::Output, N>;
            fn $method(self, rhs: Vector<S, N>) -> Self::Output {
                Vector(array::from_fn(|i| $Trait::$method(self.0[i], rhs.0[i])))
            }
        }
        impl<T, S, const N: usize> $Trait<&Vector<S, N>> for Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            type Output = Vector<<T as $Trait<S>>::Output, N>;
            fn $method(self, rhs: &Vector<S, N>) -> Self::Output { $Trait::$method(self, *rhs) }
        }
        impl<T, S, const N: usize> $Trait<Vector<S, N>> for &Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            type Output = Vector<<T as $Trait<S>>::Output, N>;
            fn $method(self, rhs: Vector<S, N>) -> Self::Output { $Trait::$method(*self, rhs) }
        }
        impl<T, S, const N: usize> $Trait<&Vector<S, N>> for &Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            type Output = Vector<<T as $Trait<S>>::Output, N>;
            fn $method(self, rhs: &Vector<S, N>) -> Self::Output { $Trait::$method(*self, *rhs) }
        }
    )*};
}

/// op Vector<T> → Vector<T> (単項、owned/ref の2組合せ)
macro_rules! impl_unary_op {
    ($($Trait:ident :: $method:ident),* $(,)?) => {$(
        impl<T: $Trait<Output = T> + Copy, const N: usize> $Trait for Vector<T, N> {
            type Output = Self;
            fn $method(self) -> Self { Vector(self.0.map(|x| $Trait::$method(x))) }
        }
        impl<T: $Trait<Output = T> + Copy, const N: usize> $Trait for &Vector<T, N> {
            type Output = Vector<T, N>;
            fn $method(self) -> Vector<T, N> { $Trait::$method(*self) }
        }
    )*};
}

/// Vector<T> op= Vector<S> (代入 zip、owned/ref の2組合せ)
macro_rules! impl_binop_assign {
    ($($Trait:ident :: $method:ident),* $(,)?) => {$(
        impl<T, S, const N: usize> $Trait<Vector<S, N>> for Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            fn $method(&mut self, rhs: Vector<S, N>) {
                for i in 0..N { $Trait::$method(&mut self.0[i], rhs.0[i]); }
            }
        }
        impl<T, S, const N: usize> $Trait<&Vector<S, N>> for Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            fn $method(&mut self, rhs: &Vector<S, N>) { $Trait::$method(self, *rhs); }
        }
    )*};
}

/// Vector<T> op Scalar<S> → Vector<T::Output> (broadcast、owned/ref の2組合せ)
macro_rules! impl_scalar_broadcast {
    ($($Trait:ident :: $method:ident),* $(,)?) => {$(
        impl<T, S, const N: usize> $Trait<Scalar<S>> for Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            type Output = Vector<<T as $Trait<S>>::Output, N>;
            fn $method(self, Scalar(rhs): Scalar<S>) -> Self::Output {
                Vector(self.0.map(|x| $Trait::$method(x, rhs)))
            }
        }
        impl<T, S, const N: usize> $Trait<Scalar<S>> for &Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            type Output = Vector<<T as $Trait<S>>::Output, N>;
            fn $method(self, rhs: Scalar<S>) -> Self::Output { $Trait::$method(*self, rhs) }
        }
    )*};
}

/// Vector<T> op= Scalar<S> (代入 broadcast)
macro_rules! impl_scalar_broadcast_assign {
    ($($Trait:ident :: $method:ident),* $(,)?) => {$(
        impl<T, S, const N: usize> $Trait<Scalar<S>> for Vector<T, N>
        where T: $Trait<S> + Copy, S: Copy {
            fn $method(&mut self, Scalar(rhs): Scalar<S>) {
                for i in 0..N { $Trait::$method(&mut self.0[i], rhs); }
            }
        }
    )*};
}

impl_binop!(Add::add, Sub::sub, Mul::mul, Div::div);
impl_unary_op!(Neg::neg);
impl_binop_assign!(
    AddAssign::add_assign,
    SubAssign::sub_assign,
    MulAssign::mul_assign,
    DivAssign::div_assign,
);
impl_scalar_broadcast!(Add::add, Sub::sub, Mul::mul, Div::div);
impl_scalar_broadcast_assign!(
    AddAssign::add_assign,
    SubAssign::sub_assign,
    MulAssign::mul_assign,
    DivAssign::div_assign,
);

impl<T: Copy, const N: usize> Vector<T, N> {
    pub fn dot<S>(self, rhs: Vector<S, N>) -> <T as Mul<S>>::Output
    where
        T: Mul<S>,
        S: Copy,
        <T as Mul<S>>::Output: Sum,
    {
        self.0.into_iter().zip(rhs.0).map(|(a, b)| a * b).sum()
    }
}

impl<T> Vector<T, 2>
where
    T: Mul<Output = T> + Sub<Output = T> + Copy,
{
    pub fn cross(self, rhs: Self) -> T {
        let [ax, ay] = self.0;
        let [bx, by] = rhs.0;
        ax * by - ay * bx
    }
}

impl<T> Vector<T, 3>
where
    T: Mul<Output = T> + Sub<Output = T> + Copy,
{
    pub fn cross(self, rhs: Self) -> Self {
        Self(array::from_fn(|i| {
            let [x, y] = [(i + 1) % 3, (i + 2) % 3];
            self[x] * rhs[y] - self[y] * rhs[x]
        }))
    }
}

impl<T> Vector<T, 2>
where
    T: Mul<Output = T> + Sub<Output = T> + Ord + Default + Copy,
{
    pub fn argcmp(&self, rhs: &Self) -> Ordering {
        let [ax, ay] = self.0;
        let [bx, by] = rhs.0;
        let z = T::default();
        ([ay, ax] < [z, z])
            .cmp(&([by, bx] < [z, z]))
            .then_with(|| (bx * ay).cmp(&(ax * by)))
    }
}

impl<T: Copy, const N: usize> IntN<N> for Vector<T, N>
where
    [T; N]: IntN<N>,
{
    #[inline]
    fn to_ints(self) -> [isize; N] {
        self.0.to_ints()
    }
}
