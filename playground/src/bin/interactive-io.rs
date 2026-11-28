//! 標準入力の補充時に、問い合わせを相手へ送る例。
use cplib_lang_io::{self as io, INPUT, input, outln, stdin};

fn main() {
    io::run(solve);
}

fn solve() {
    let initial = input!(i64);
    outln!("? {initial}");
    let total = initial + input!(i64);

    outln!("? {total}");
    assert!(!stdin().is_empty());
    outln!("! {}", total + INPUT.read::<i64>());
}
