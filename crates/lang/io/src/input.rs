use std::any::type_name;
use std::fmt::Debug;
use std::io::{self, BufRead, Read};
use std::str::FromStr;
use std::sync::{LazyLock, Mutex, MutexGuard};

/// 空白区切りの入力元。トークン取得が進むため、読み取りには可変参照を渡す。
pub trait Source {
    fn next_token(&mut self) -> Option<&str>;

    fn read<T: Readable>(&mut self) -> T::Output {
        T::read(self)
    }
}

impl<S: Source + ?Sized> Source for &mut S {
    fn next_token(&mut self) -> Option<&str> {
        (**self).next_token()
    }
}

impl<S: Source + ?Sized> Source for MutexGuard<'_, S> {
    fn next_token(&mut self) -> Option<&str> {
        (**self).next_token()
    }
}

/// 入力元から1つの値を読む。1トークンとは限らず、独自型は必要な個数を消費できる。
/// `Output` により `Usize1` などの読み取り指定も扱う。
pub trait Readable {
    type Output;
    fn read<S: Source + ?Sized>(source: &mut S) -> Self::Output;
}

/// 実行時のサイズなどを伴う読み取り指定。`input!(with reader)` から使える。
pub trait RuntimeReadable {
    type Output;
    fn read<S: Source + ?Sized>(self, source: &mut S) -> Self::Output;
}

/// 全入力を保持するスキャナ。標準入力のデフォルト。対話問題には `LineSource` を使う。
#[derive(Clone, Debug)]
pub struct Scanner {
    input: String,
    position: usize,
}

impl Scanner {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            position: 0,
        }
    }

    pub fn from_reader(mut reader: impl Read) -> io::Result<Self> {
        let mut input = String::new();
        reader.read_to_string(&mut input)?;
        Ok(Self::new(input))
    }

    pub fn read<T: Readable>(&mut self) -> T::Output {
        T::read(self)
    }

    /// 未消費のトークンがないか。入力位置は変えない。
    pub fn is_empty(&self) -> bool {
        self.input.as_bytes()[self.position..]
            .iter()
            .all(u8::is_ascii_whitespace)
    }
}

impl Source for Scanner {
    fn next_token(&mut self) -> Option<&str> {
        let bytes = self.input.as_bytes();
        while self.position < bytes.len() && bytes[self.position].is_ascii_whitespace() {
            self.position += 1;
        }
        let start = self.position;
        while self.position < bytes.len() && !bytes[self.position].is_ascii_whitespace() {
            self.position += 1;
        }
        (start < self.position).then(|| &self.input[start..self.position])
    }
}

/// 必要になった行だけ読む入力元。`LineSource::new(BufReader::new(stdin()))` などで作る。
pub struct LineSource<R> {
    reader: R,
    line: String,
    position: usize,
}

impl<R: BufRead> LineSource<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line: String::new(),
            position: 0,
        }
    }

    pub fn read<T: Readable>(&mut self) -> T::Output {
        T::read(self)
    }
}

impl<R: BufRead> Source for LineSource<R> {
    fn next_token(&mut self) -> Option<&str> {
        loop {
            let bytes = self.line.as_bytes();
            while self.position < bytes.len() && bytes[self.position].is_ascii_whitespace() {
                self.position += 1;
            }
            let start = self.position;
            while self.position < bytes.len() && !bytes[self.position].is_ascii_whitespace() {
                self.position += 1;
            }
            if start < self.position {
                return Some(&self.line[start..self.position]);
            }
            self.line.clear();
            self.position = 0;
            if self
                .reader
                .read_line(&mut self.line)
                .expect("failed to read input")
                == 0
            {
                return None;
            }
        }
    }
}

static STDIN: LazyLock<Mutex<Scanner>> = LazyLock::new(|| {
    Mutex::new(Scanner::from_reader(io::stdin().lock()).expect("failed to read stdin"))
});

/// グローバルな入力元のガード。`Grid::read(h, w, &mut source)` などへ渡せる。
/// 保持中は `input!(from &mut source, ...)` を使い、再び `stdin()` をロックしないこと。
pub fn stdin() -> MutexGuard<'static, Scanner> {
    STDIN.lock().expect("stdin lock poisoned")
}

pub fn read<T: Readable>() -> T::Output {
    T::read(&mut *stdin())
}

pub(crate) fn parse_token<T: FromStr, S: Source + ?Sized>(source: &mut S) -> T
where
    T::Err: Debug,
{
    let token = source
        .next_token()
        .unwrap_or_else(|| panic!("unexpected EOF while reading {}", type_name::<T>()));
    token.parse().unwrap_or_else(|error| {
        panic!(
            "failed to parse {token:?} as {}: {error:?}",
            type_name::<T>()
        )
    })
}

// FromStr 全般への blanket impl は、複数トークンを読む独自型の実装と衝突する。
macro_rules! scalar {
    ($($ty:ty),* $(,)?) => {$(
        impl Readable for $ty {
            type Output = Self;
            fn read<S: Source + ?Sized>(source: &mut S) -> Self {
                parse_token(source)
            }
        }
    )*};
}
scalar!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64, bool, char, String
);

impl<T: Readable, const N: usize> Readable for [T; N] {
    type Output = [T::Output; N];
    fn read<S: Source + ?Sized>(source: &mut S) -> Self::Output {
        std::array::from_fn(|_| T::read(source))
    }
}

macro_rules! tuple {
    ($($ty:ident),* $(,)?) => {
        impl<$($ty: Readable),*> Readable for ($($ty,)*) {
            type Output = ($($ty::Output,)*);
            fn read<S: Source + ?Sized>(source: &mut S) -> Self::Output {
                ($($ty::read(source),)*)
            }
        }
    };
}
impl Readable for () {
    type Output = ();
    fn read<S: Source + ?Sized>(_: &mut S) {}
}
tuple!(A);
tuple!(A, B);
tuple!(A, B, C);
tuple!(A, B, C, D);
tuple!(A, B, C, D, E);
tuple!(A, B, C, D, E, F);
tuple!(A, B, C, D, E, F, G);
tuple!(A, B, C, D, E, F, G, H);
