use std::any::type_name;
use std::fmt::Debug;
use std::io::{self, BufRead, Read};
use std::str::FromStr;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// 空白区切りの入力元。トークン取得が進むため、読み取りには可変参照を渡す。
pub trait Source {
    fn next_token(&mut self) -> Option<&str>;

    fn read<T: Readable>(&mut self) -> T::Output {
        T::read(self)
    }
}

/// 入力元を読み取り操作の間だけ取得する。
/// ローカルな `&mut Scanner` と共有バッファの `&INPUT` を同じ API に渡せる。
pub trait InputSource {
    fn lock_source(&mut self) -> impl Source + '_;
}

impl<S: Source + ?Sized> InputSource for S {
    fn lock_source(&mut self) -> impl Source + '_ {
        self
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

/// 共有参照から読める入力バッファ。読み取り操作の間だけ内部をロックする。
/// `new()` は標準入力を遅延初期化し、`From<Scanner>` は独立した入力元を作る。
pub struct InputBuffer {
    source: OnceLock<Mutex<Scanner>>,
}

impl InputBuffer {
    pub const fn new() -> Self {
        Self {
            source: OnceLock::new(),
        }
    }

    /// 複数トークンを読む型も、1回の読み取り操作として処理する。
    pub fn read<T: Readable>(&self) -> T::Output {
        T::read(&mut *self.lock())
    }

    /// まとめて処理したい場合の明示的なロック。
    /// ガード保持中は同じバッファの `read` / `lock` を再度呼ばないこと。
    pub fn lock(&self) -> MutexGuard<'_, Scanner> {
        self.source
            .get_or_init(|| {
                Mutex::new(Scanner::from_reader(io::stdin().lock()).expect("failed to read stdin"))
            })
            .lock()
            .expect("input lock poisoned")
    }
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Scanner> for InputBuffer {
    fn from(source: Scanner) -> Self {
        Self {
            source: OnceLock::from(Mutex::new(source)),
        }
    }
}

impl InputSource for &InputBuffer {
    fn lock_source(&mut self) -> impl Source + '_ {
        self.lock()
    }
}

/// `input!` と共通の標準入力バッファ。`Grid::read(h, w, &INPUT)` のように使う。
pub static INPUT: InputBuffer = InputBuffer::new();

/// まとめ読みのためにロックガードを取得する。通常は `&INPUT` を直接渡せばよい。
/// 保持中は `input!(from &mut source, ...)` を使い、同じバッファを再ロックしないこと。
pub fn stdin() -> MutexGuard<'static, Scanner> {
    INPUT.lock()
}

pub fn read<T: Readable>() -> T::Output {
    INPUT.read::<T>()
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
