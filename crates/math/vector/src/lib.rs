use prim::IntN;
use std::{
    array,
    cmp::Ordering,
    error::Error,
    fmt,
    iter::Sum,
    ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    str::FromStr,
};

/// 固定長 `N` の数ベクトル。`N` を省略すると 2 次元。
///
/// 初期化は用途に応じて選ぶ:
///
/// ```
/// use vector::Vector;
/// type V2 = Vector<i64, 2>;
///
/// let a = Vector([1, 2]);        // 配列をそのまま包む
/// let b = V2::new(1, 2);         // 要素を並べる (N = 1..=4)
/// let c = V2::from([1, 2]);      // From<[T; N]>
/// let d: V2 = (1, 2).into();     // From<(T, T)> (N = 2, 3)
/// let e = V2::splat(1);          // 全要素同じ値
/// let f = V2::from_fn(|i| i as i64 + 1);
/// let g = V2::default();         // 零ベクトル
/// let h = "1 2".parse::<V2>().unwrap();
/// # let _ = (a, b, c, d, e, f, g, h);
/// ```
///
/// 型エイリアス経由で `V2([1, 2])` とは**書けない**。`type` はタプル構造体の
/// コンストラクタ (値名前空間) を別名にしないため。`V2::new` / `V2::from` を使うか、
/// 型引数を固定しないなら `use vector::Vector as V;` で `V([1, 2])` と書く。
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

impl<T, const N: usize> Vector<T, N> {
    /// 添字から各要素を作る。`Vector::<i64, 3>::from_fn(|i| i as i64)` → `[0, 1, 2]`。
    pub fn from_fn(f: impl FnMut(usize) -> T) -> Self {
        Self(array::from_fn(f))
    }

    /// 全要素を同じ値で埋める。
    pub fn splat(x: T) -> Self
    where
        T: Copy,
    {
        Self([x; N])
    }

    /// 各要素に関数を適用する。要素の型を変えられる。
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Vector<U, N> {
        Vector(self.0.map(f))
    }
}

/// 要素を並べて書くコンストラクタ。低次元でだけ提供する。
macro_rules! impl_new {
    ($($n:literal => ($($arg:ident),*)),* $(,)?) => {$(
        impl<T> Vector<T, $n> {
            pub fn new($($arg: T),*) -> Self { Self([$($arg),*]) }
        }
    )*};
}
impl_new!(1 => (x), 2 => (x, y), 3 => (x, y, z), 4 => (x, y, z, w));

impl<T> From<(T, T)> for Vector<T, 2> {
    fn from((x, y): (T, T)) -> Self {
        Self([x, y])
    }
}

impl<T> From<Vector<T, 2>> for (T, T) {
    fn from(Vector([x, y]): Vector<T, 2>) -> Self {
        (x, y)
    }
}

impl<T> From<(T, T, T)> for Vector<T, 3> {
    fn from((x, y, z): (T, T, T)) -> Self {
        Self([x, y, z])
    }
}

impl<T> From<Vector<T, 3>> for (T, T, T) {
    fn from(Vector([x, y, z]): Vector<T, 3>) -> Self {
        (x, y, z)
    }
}

impl<T, const N: usize> From<[T; N]> for Vector<T, N> {
    fn from(array: [T; N]) -> Self {
        Self(array)
    }
}

impl<T, const N: usize> From<Vector<T, N>> for [T; N] {
    fn from(Vector(array): Vector<T, N>) -> Self {
        array
    }
}

/// 零ベクトル。`Vector::<i64>::default()` あるいは `V2::default()`。
impl<T: Default, const N: usize> Default for Vector<T, N> {
    fn default() -> Self {
        Self(array::from_fn(|_| T::default()))
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
    /// 偏角で比較する。`sort_by(Vector::argcmp)` で偏角ソートになる。
    ///
    /// 偏角 0 (正の x 軸) を始点に反時計回り、範囲は `[0, 2π)`。
    /// 上半分 (`y > 0` または `y == 0 && x >= 0`) を先に置き、
    /// 同じ半平面内は外積の符号で比較するので、三角関数を使わず整数演算で完結する。
    ///
    /// 向きが同じベクトルは長さによらず `Equal`。零ベクトルは偏角 0 と同じ扱いになり、
    /// 上半分のすべてのベクトルと `Equal` になるため、全順序が必要なら事前に除いておくこと。
    pub fn argcmp(&self, rhs: &Self) -> Ordering {
        let [ax, ay] = self.0;
        let [bx, by] = rhs.0;
        let z = T::default();
        ([ay, ax] < [z, z])
            .cmp(&([by, bx] < [z, z]))
            .then_with(|| (bx * ay).cmp(&(ax * by)))
    }
}

/// `Vector<T, N>` のパースに失敗した理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseVectorError<E> {
    /// 空白区切りの要素数が `N` と一致しなかった。
    Arity { expected: usize, found: usize },
    /// 要素の `FromStr` が失敗した。
    Element(E),
}

impl<E: fmt::Display> fmt::Display for ParseVectorError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arity { expected, found } => {
                write!(f, "expected {expected} elements, found {found}")
            }
            Self::Element(e) => write!(f, "invalid element: {e}"),
        }
    }
}

impl<E: Error + 'static> Error for ParseVectorError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Arity { .. } => None,
            Self::Element(e) => Some(e),
        }
    }
}

/// 空白区切りの `N` 要素からパースする。`"1 2 3".parse::<Vector<i64, 3>>()` のように使う。
impl<T: FromStr, const N: usize> FromStr for Vector<T, N> {
    type Err = ParseVectorError<T::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let values = s
            .split_ascii_whitespace()
            .map(T::from_str)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ParseVectorError::Element)?;
        let found = values.len();
        let array = values
            .try_into()
            .map_err(|_| ParseVectorError::Arity { expected: N, found })?;
        Ok(Self(array))
    }
}

impl<T: Copy, const N: usize> IntN<N> for Vector<T, N>
where
    [T; N]: IntN<N>,
{
    #[inline]
    fn to_isizes(self) -> [isize; N] {
        self.0.to_isizes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type V2 = Vector<i64, 2>;

    #[test]
    fn constructs_various_ways() {
        let expected = Vector([1, 2]);
        assert_eq!(V2::new(1, 2), expected);
        assert_eq!(V2::from([1, 2]), expected);
        assert_eq!(V2::from((1, 2)), expected);
        assert_eq!(<[i64; 2]>::from(expected), [1, 2]);
        assert_eq!(<(i64, i64)>::from(expected), (1, 2));
        assert_eq!(V2::from_fn(|i| i as i64 + 1), expected);
        assert_eq!(V2::splat(1), Vector([1, 1]));
        assert_eq!(V2::default(), Vector([0, 0]));
        assert_eq!(Vector::<i64, 3>::new(1, 2, 3), Vector([1, 2, 3]));
        assert_eq!(Vector::<i64, 4>::new(1, 2, 3, 4), Vector([1, 2, 3, 4]));
    }

    #[test]
    fn maps_elements() {
        assert_eq!(Vector([1i64, -2]).map(|x| x.abs()), Vector([1, 2]));
        assert_eq!(
            Vector([1i64, 2]).map(|x| x as f64 / 2.0),
            Vector([0.5, 1.0])
        );
    }

    #[test]
    fn parses_whitespace_separated() {
        assert_eq!("1 -2".parse::<Vector<i64>>(), Ok(Vector([1, -2])));
        assert_eq!(
            "  3\t4\n5 ".parse::<Vector<i64, 3>>(),
            Ok(Vector([3, 4, 5]))
        );
        assert_eq!("2.5 0.5".parse::<Vector<f64>>(), Ok(Vector([2.5, 0.5])));
    }

    #[test]
    fn rejects_wrong_arity() {
        assert_eq!(
            "1 2 3".parse::<Vector<i64>>(),
            Err(ParseVectorError::Arity {
                expected: 2,
                found: 3
            })
        );
        assert_eq!(
            "1".parse::<Vector<i64>>(),
            Err(ParseVectorError::Arity {
                expected: 2,
                found: 1
            })
        );
        assert_eq!(
            "".parse::<Vector<i64>>(),
            Err(ParseVectorError::Arity {
                expected: 2,
                found: 0
            })
        );
    }

    #[test]
    fn reports_element_error() {
        let err = "1 x".parse::<Vector<i64>>().unwrap_err();
        assert!(matches!(err, ParseVectorError::Element(_)));
    }
}
