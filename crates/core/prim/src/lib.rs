/// プリミティブ整数型を `isize` へ変換する識別トレイト。
/// `usize`, `i32` など言語仕様上は別型として存在するものを ℤ の元として同一視する。
pub trait Int: Copy {
    fn to_isize(self) -> isize;
}

macro_rules! impl_int {
    ($($t:ty),*) => {
        $(impl Int for $t {
            #[inline]
            fn to_isize(self) -> isize { self as isize }
        })*
    };
}
impl_int!(
    usize, isize, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128
);

/// N個の整数として解釈できることを保証するトレイト。`Int` のN次元への持ち上げ。
/// `[i32; 2]`, `(usize, usize)`など異なる型を ℤ^N の元として同一視する。
pub trait IntN<const N: usize>: Sized {
    fn to_ints(self) -> [isize; N];

    /// `[usize; N]` に変換する。配列インデックス等に使う。
    /// 負の値は `as usize` でラップアラウンドするため、呼び出し側で範囲チェックを行うこと。
    #[inline]
    fn to_index(self) -> [usize; N] {
        self.to_ints().map(|c| c as usize)
    }
}

impl<I: Int, const N: usize> IntN<N> for [I; N] {
    #[inline]
    fn to_ints(self) -> [isize; N] {
        self.map(|x| x.to_isize())
    }
}

impl<I: Int> IntN<2> for (I, I) {
    #[inline]
    fn to_ints(self) -> [isize; 2] {
        [self.0.to_isize(), self.1.to_isize()]
    }
}

impl<I: Int> IntN<3> for (I, I, I) {
    #[inline]
    fn to_ints(self) -> [isize; 3] {
        [self.0.to_isize(), self.1.to_isize(), self.2.to_isize()]
    }
}
