//! 型そのものとは異なる読み取り方を指定するマーカー。
use std::fmt::Debug;
use std::marker::PhantomData;
use std::str::FromStr;

use super::input::parse_token;
use super::{Readable, Source};

pub struct Usize1;
pub struct Isize1;
pub struct Chars;
pub struct Bytes;

/// 任意の `FromStr` 型を1トークンとして読む。`Readable` の個別実装は不要。
pub struct Token<T>(PhantomData<fn() -> T>);

impl<T: FromStr> Readable for Token<T>
where
    T::Err: Debug,
{
    type Output = T;
    fn read<S: Source + ?Sized>(source: &mut S) -> T {
        parse_token(source)
    }
}

impl Readable for Usize1 {
    type Output = usize;
    fn read<S: Source + ?Sized>(source: &mut S) -> usize {
        usize::read(source)
            .checked_sub(1)
            .expect("Usize1 requires a positive integer")
    }
}

impl Readable for Isize1 {
    type Output = isize;
    fn read<S: Source + ?Sized>(source: &mut S) -> isize {
        isize::read(source)
            .checked_sub(1)
            .expect("Isize1 underflow")
    }
}

impl Readable for Chars {
    type Output = Vec<char>;
    fn read<S: Source + ?Sized>(source: &mut S) -> Self::Output {
        String::read(source).chars().collect()
    }
}

impl Readable for Bytes {
    type Output = Vec<u8>;
    fn read<S: Source + ?Sized>(source: &mut S) -> Self::Output {
        String::read(source).into_bytes()
    }
}
