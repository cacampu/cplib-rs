use std::{
    array,
    cmp::Ordering,
    iter::Sum,
    ops::{Add, Deref, DerefMut, Mul, Neg, Sub},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vector<T, const N: usize>(pub [T; N]);

impl<T, const N: usize> Vector<T, N>
where
    T: Copy + Mul<T, Output = T> + Sum,
{
    pub fn dot(self, rhs: Self) -> T {
        self.0
            .into_iter()
            .zip(rhs.0.into_iter())
            .map(|(a, b)| a * b)
            .sum()
    }
}
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
pub trait SemiRing: Sized + Add<Self, Output = Self> + Mul<Self, Output = Self> {
    fn zero() -> Self;
    fn one() -> Self;
}

pub trait Ring: SemiRing + Sub<Self, Output = Self> + Neg {}
macro_rules! impl_semiring {
    ($($t:ty),*) => {
        $(
            impl SemiRing for $t{
                fn zero() -> Self{
                    0
                }
                fn one() -> Self{
                    1
                }
            }
        )*
    };
}
impl_semiring!(
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128
);
macro_rules! impl_ring {
    ($($t:ty),*) => {
        $(
            impl Ring for $t{}
        )*
    };
}
impl_ring!(isize, i8, i16, i32, i64, i128);

impl<T, const N: usize> Add<Self> for Vector<T, N>
where
    T: Copy + SemiRing,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(array::from_fn(|i| Add::add(self.0[i], rhs.0[i])))
    }
}
impl<T, const N: usize> Mul<T> for Vector<T, N>
where
    T: Copy + SemiRing,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self(array::from_fn(|i| self.0[i] * rhs))
    }
}

impl<T, const N: usize> Sub<Self> for Vector<T, N>
where
    T: Copy + Ring,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(array::from_fn(|i| self.0[i] - rhs.0[i]))
    }
}
impl<T: Ring + Copy + Ord> Vector<T, 2> {
    pub fn argcmp(&self, rhs: &Self) -> Ordering {
        let [ax, ay] = self.0;
        let [bx, by] = rhs.0;
        let z = T::zero();
        ([ay, ax] < [z, z])
            .cmp(&([by, bx] < [z, z]))
            .then_with(|| (bx * ay).cmp(&(ax * by)))
    }
}
impl<T: Ring + Copy> Vector<T, 2> {
    pub fn cross(self, rhs: Self) -> T {
        let [ax, ay] = self.0;
        let [bx, by] = rhs.0;
        ax * by - ay * bx
    }
}
impl<T: Ring + Copy> Vector<T, 3> {
    pub fn cross(self, rhs: Self) -> Self {
        Self(array::from_fn(|i| {
            let [x, y] = [(i + 1) % 3, (i + 2) % 3];
            self[x] * rhs[y] - self[y] * rhs[x]
        }))
    }
}
