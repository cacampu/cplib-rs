use std::any::type_name;
use std::fmt::Debug;
use std::io::{self, BufRead, BufReader, Read};
use std::ops::Range;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock};

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

/// 文字列からトークンを読む。共有の標準入力では必要に応じて次の行を補充する。
#[derive(Clone, Debug)]
pub struct Scanner {
    input: String,
    position: usize,
    stdin: Option<StdinRefill>,
}

#[derive(Clone, Debug, Default)]
struct StdinRefill {
    peeked: OnceLock<Option<String>>,
    eof: bool,
}

impl Scanner {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            position: 0,
            stdin: None,
        }
    }

    fn from_stdin() -> Self {
        Self {
            stdin: Some(StdinRefill::default()),
            ..Self::new(String::new())
        }
    }

    /// EOF まで入力を一括で読み込む。
    pub fn from_reader(mut reader: impl Read) -> io::Result<Self> {
        let mut input = String::new();
        reader.read_to_string(&mut input)?;
        Ok(Self::new(input))
    }

    pub fn read<T: Readable>(&mut self) -> T::Output {
        T::read(self)
    }

    /// 未消費のトークンがないか。標準入力では必要に応じて先読みし、トークンは消費しない。
    pub fn is_empty(&self) -> bool {
        if self.input.as_bytes()[self.position..]
            .iter()
            .any(|byte| !byte.is_ascii_whitespace())
        {
            return false;
        }
        self.stdin.as_ref().is_none_or(|stdin| {
            stdin.eof
                || stdin
                    .peeked
                    .get_or_init(|| {
                        let mut line = String::new();
                        read_stdin_line(&mut line).then_some(line)
                    })
                    .is_none()
        })
    }
}

impl Source for Scanner {
    fn next_token(&mut self) -> Option<&str> {
        loop {
            if let Some(token) = token_range(&self.input, &mut self.position) {
                return Some(&self.input[token]);
            }
            let stdin = self.stdin.as_mut()?;
            if stdin.eof {
                return None;
            }
            self.position = 0;
            let found = match stdin.peeked.take() {
                Some(Some(line)) => {
                    self.input = line;
                    true
                }
                Some(None) => false,
                None => read_stdin_line(&mut self.input),
            };
            if !found {
                self.input.clear();
                stdin.eof = true;
                return None;
            }
        }
    }
}

fn token_range(text: &str, position: &mut usize) -> Option<Range<usize>> {
    let bytes = text.as_bytes();
    while *position < bytes.len() && bytes[*position].is_ascii_whitespace() {
        *position += 1;
    }
    let start = *position;
    while *position < bytes.len() && !bytes[*position].is_ascii_whitespace() {
        *position += 1;
    }
    (start < *position).then_some(start..*position)
}

struct StdinRefillReader;

impl Read for StdinRefillReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // BufReader に入力が残っている間は、行をまたいでもここには来ない。
        super::output::flush_if_initialized();
        io::stdin().read(buf)
    }
}

fn read_stdin_line(line: &mut String) -> bool {
    static STDIN: LazyLock<Mutex<BufReader<StdinRefillReader>>> =
        LazyLock::new(|| Mutex::new(BufReader::new(StdinRefillReader)));
    let mut stdin = STDIN.lock().expect("stdin reader lock poisoned");
    loop {
        line.clear();
        if stdin.read_line(line).expect("failed to read stdin") == 0 {
            return false;
        }
        if line
            .as_bytes()
            .iter()
            .any(|byte| !byte.is_ascii_whitespace())
        {
            return true;
        }
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
            if let Some(token) = token_range(&self.line, &mut self.position) {
                return Some(&self.line[token]);
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
/// `new()` は標準入力を必要な分だけ読み、`From<Scanner>` は独立した入力元を作る。
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
            .get_or_init(|| Mutex::new(Scanner::from_stdin()))
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
