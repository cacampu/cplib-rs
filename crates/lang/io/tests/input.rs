use io::marker::{Bytes, Chars, Token, Usize1};
use io::{InputBuffer, LineSource, Readable, RuntimeReadable, Scanner, Source, input, read_value};

#[test]
fn declarations_expressions_arrays_tuples_and_markers() {
    let mut source = Scanner::new("3 10 20 30 2 7 8 3 9 10 11 2 4 5 6 7 3 xyz あいう 99");
    input! { from &mut source, n: usize, mut values: [i64; n] }
    values[0] += 1;
    assert_eq!(values, [11, 20, 30]);
    assert_eq!(input!(from &mut source, [usize]), vec![7, 8]);
    input! { from &mut source, from: usize, (a, b, c): (i32, i32, i32), matrix: [[i32; 2]; 2] }
    assert_eq!((from, a, b, c), (3, 9, 10, 11));
    assert_eq!(matrix, [vec![2, 4], vec![5, 6]]);
    assert_eq!(read_value!(from &mut source, Usize1), 6);
    input! { from &mut source, index: Usize1, bytes: Bytes, chars: Chars, last: std::primitive::u64, }
    assert_eq!(index, 2);
    assert_eq!(bytes, b"xyz");
    assert_eq!(chars, ['あ', 'い', 'う']);
    assert_eq!(last, 99);
    assert!(source.is_empty());
}

#[test]
fn aliases_generics_and_source_access() {
    type Pair = (i32, i64);
    type Triple = [u8; 3];
    let mut source = Scanner::new("1 2 3 4 5 6 -7");
    assert_eq!(input!(from &mut source, Pair), (1, 2));
    // 型の配列は固定長、input! の [T; n] 構文は proconio と同じ Vec。
    let fixed: Triple = source.read::<Triple>();
    assert_eq!(fixed, [3, 4, 5]);
    let singleton = input!(from &mut source, (u32,));
    assert_eq!(singleton, (6,));
    assert_eq!(
        input!(from &mut source, Token<std::num::NonZeroI64>),
        std::num::NonZeroI64::new(-7).unwrap()
    );
    let () = input!(from &mut source, ());
    let _: Vec<i32> = input!(from &mut source, [i32; 0]);
    assert!(source.is_empty());
}

struct Pair(i64, i64);

// FromStr と複数トークンの Readable が同時に実装できることを確認する。
impl std::str::FromStr for Pair {
    type Err = ();
    fn from_str(_: &str) -> Result<Self, Self::Err> {
        Err(())
    }
}

impl Readable for Pair {
    type Output = Self;
    fn read<S: Source + ?Sized>(source: &mut S) -> Self {
        Self(source.read::<i64>(), source.read::<i64>())
    }
}

struct Many(usize);
impl RuntimeReadable for Many {
    type Output = Vec<Pair>;
    fn read<S: Source + ?Sized>(self, source: &mut S) -> Self::Output {
        (0..self.0).map(|_| Pair::read(source)).collect()
    }
}

#[test]
fn custom_types_and_runtime_readers() {
    let mut source = Scanner::new("1 2 3 4 5 6 7 8");
    input! { from &mut source, pairs: [Pair; 2], more: with Many(1) }
    assert_eq!(
        (pairs[0].0, pairs[0].1, pairs[1].0, pairs[1].1),
        (1, 2, 3, 4)
    );
    assert_eq!((more[0].0, more[0].1), (5, 6));
    let last = input!(from &mut source, with Many(1));
    assert_eq!((last[0].0, last[0].1), (7, 8));
}

#[test]
fn sources_agree_across_blank_lines_and_eof() {
    let text = "\r\n  1\t2\n\nあいう\r\n-4  ";
    let mut once = Scanner::new(text);
    let mut lines = LineSource::new(std::io::Cursor::new(text));
    assert_eq!(
        input!(from &mut once, (i32, i32, String, i32)),
        input!(from &mut lines, (i32, i32, String, i32))
    );
    assert_eq!(lines.next_token(), None);
    assert_eq!(lines.next_token(), None);
    assert_eq!(once.next_token(), None);
}

#[test]
#[should_panic(expected = "unexpected EOF")]
fn missing_input_has_context() {
    input!(from &mut Scanner::new(""), i32);
}

#[test]
fn shared_buffer_interleaves_macro_direct_and_helper_reads() {
    fn pair(buffer: &InputBuffer) -> (i64, i64) {
        buffer.read::<(i64, i64)>()
    }
    let buffer = InputBuffer::from(Scanner::new("1 2 3 4 5 6 7 8"));
    input! { from &buffer, first: i64 }
    assert_eq!(first, 1);
    assert_eq!(pair(&buffer), (2, 3));
    assert_eq!(input!(from & buffer, [i64; 2]), [4, 5]);
    input! { from &buffer, sixth: i64, last: (i64, i64) }
    assert_eq!((sixth, last), (6, (7, 8)));
    assert!(buffer.lock().is_empty());
}

#[test]
fn shared_buffer_reads_each_compound_value_under_one_lock() {
    let data = (0..100)
        .map(|i| format!("{i} {i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let buffer = InputBuffer::from(Scanner::new(data));
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let buffer = &buffer;
            scope.spawn(move || {
                for _ in 0..25 {
                    let (a, b) = buffer.read::<(usize, usize)>();
                    assert_eq!(a, b);
                }
            });
        }
    });
    assert!(buffer.lock().is_empty());
}
