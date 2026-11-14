//! 問題に紐づけずに試すための作業用バイナリ。
use cplib_lang_io::{fastout, input, outln};

#[fastout]
fn main() {
    let n = input!(usize);
    let a = input!([i64; n]);
    outln!("{}", a.iter().sum::<i64>());
}
