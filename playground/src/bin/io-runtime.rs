//! 出力がヘルパー・クロージャ・別スレッドでも同じバッファを使う例。
use cplib_lang_io::{self as io, out, outln};

fn main() {
    io::run(solve);
}

fn solve() {
    out!("main ");
    io::run(helper);
    let closure = || outln!("closure");
    closure();
    std::thread::spawn(|| outln!("thread")).join().unwrap();
    outln!([1, 2, 3]);
    assert_eq!(
        io::stdout().buffer(),
        b"main helper closure\nthread\n1 2 3\n"
    );
}

fn helper() {
    out!("helper ");
}
