use cplib_lang_io::marker::Bytes;
use cplib_lang_io::{input, multi_testcase, outln};

#[multi_testcase]
fn main() {
    let n = input!(usize);
    let s = input!(Bytes);
    for cut in 1..n {
        if s[..cut] < s[cut..] {
            answer(true);
            return; // 次のテストケースへ進む。
        }
    }
    answer(false);
}

fn answer(ok: bool) {
    // main と同じ出力バッファを、別の関数から使う。
    outln!("{}", cplib_lang_io::yes_no(ok));
}
