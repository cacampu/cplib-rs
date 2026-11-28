use std::fmt::{self, Display};

/// 競プロ用の出力形式。Vec・配列は空白区切り、Option の None は `-1`。
/// 独自型にも実装できる。Display のみの型は `display(value)` で包める。
pub trait Printable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<T: Printable + ?Sized> Printable for &T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: Printable + ?Sized> Printable for &mut T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

macro_rules! scalar {
    ($($t:ty),* $(,)?) => {$(
        impl Printable for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                Display::fmt(self, f)
            }
        }
    )*};
}
scalar!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    bool,
    char,
    str,
    String,
    fmt::Arguments<'_>
);

impl<T: Printable> Printable for [T] {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, value) in self.iter().enumerate() {
            if i != 0 {
                f.write_str(" ")?;
            }
            Printable::fmt(value, f)?;
        }
        Ok(())
    }
}

impl<T: Printable> Printable for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Printable::fmt(self.as_slice(), f)
    }
}

impl<T: Printable, const N: usize> Printable for [T; N] {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Printable::fmt(self.as_slice(), f)
    }
}

impl<T: Printable> Printable for Option<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Some(value) => Printable::fmt(value, f),
            None => f.write_str("-1"),
        }
    }
}

impl Printable for () {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

macro_rules! tuple {
    ($first:ident $(, $rest:ident)* $(,)?) => {
        impl<$first: Printable, $($rest: Printable),*> Printable for ($first, $($rest,)*) {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                #[allow(non_snake_case)]
                let ($first, $($rest,)*) = self;
                Printable::fmt($first, f)?;
                $(f.write_str(" ")?; Printable::fmt($rest, f)?;)*
                Ok(())
            }
        }
    };
}
tuple!(A);
tuple!(A, B);
tuple!(A, B, C);
tuple!(A, B, C, D);
tuple!(A, B, C, D, E);
tuple!(A, B, C, D, E, F);
tuple!(A, B, C, D, E, F, G);
tuple!(A, B, C, D, E, F, G, H);

/// Printable の形式を std::format! / write! などでも使うアダプタ。
pub struct Printed<T>(T);

pub fn printable<T: Printable>(value: T) -> Printed<T> {
    Printed(value)
}

/// `debug!` から Printable と Debug を解決するためのラッパー。
#[doc(hidden)]
pub struct DebugValue<'a, T: ?Sized>(pub &'a T);

#[doc(hidden)]
pub trait DebugDisplay {
    fn as_debug_display(self) -> impl Display;
}

impl<T: Printable + ?Sized> DebugDisplay for &&DebugValue<'_, T> {
    fn as_debug_display(self) -> impl Display {
        printable(self.0)
    }
}

impl<T: fmt::Debug + ?Sized> DebugDisplay for &DebugValue<'_, T> {
    fn as_debug_display(self) -> impl Display {
        struct DisplayDebug<'a, T: ?Sized>(&'a T);

        impl<T: fmt::Debug + ?Sized> Display for DisplayDebug<'_, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(self.0, f)
            }
        }

        DisplayDebug(self.0)
    }
}

impl<T: Printable> Display for Printed<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Printable::fmt(&self.0, f)
    }
}

impl<T: Printable> Printable for Printed<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Printable::fmt(&self.0, f)
    }
}

/// 任意の Display 型を Printable として扱うアダプタ。
pub struct DisplayValue<T>(T);

pub fn display<T: Display>(value: T) -> DisplayValue<T> {
    DisplayValue(value)
}

impl<T: Display> Printable for DisplayValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T: Display> Display for DisplayValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/// 各要素を指定した区切り文字で出力する。元の配列を借用し、文字列は作らない。
pub struct Separated<'a, T, S> {
    values: &'a [T],
    separator: S,
}

/// `separated(&values, ",")` / `separated(&values, '\n')` など。
pub fn separated<T: Printable, S: Display>(values: &[T], separator: S) -> Separated<'_, T, S> {
    Separated { values, separator }
}

impl<T: Printable, S: Display> Printable for Separated<'_, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, value) in self.values.iter().enumerate() {
            if i != 0 {
                Display::fmt(&self.separator, f)?;
            }
            Printable::fmt(value, f)?;
        }
        Ok(())
    }
}

impl<T: Printable, S: Display> Display for Separated<'_, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Printable::fmt(self, f)
    }
}

pub fn lines<T: Printable>(values: &[T]) -> Separated<'_, T, &'static str> {
    separated(values, "\n")
}

/// None の出力だけを変更するアダプタ。
pub struct OptionOr<'a, T, F> {
    value: &'a Option<T>,
    fallback: F,
}

pub fn option_or<T: Printable, F: Printable>(value: &Option<T>, fallback: F) -> OptionOr<'_, T, F> {
    OptionOr { value, fallback }
}

impl<T: Printable, F: Printable> Printable for OptionOr<'_, T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value {
            Some(value) => Printable::fmt(value, f),
            None => Printable::fmt(&self.fallback, f),
        }
    }
}

impl<T: Printable, F: Printable> Display for OptionOr<'_, T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Printable::fmt(self, f)
    }
}
