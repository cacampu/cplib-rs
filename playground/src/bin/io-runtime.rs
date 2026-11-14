//! 出力がヘルパー・クロージャ・別スレッドでも同じバッファを使う例。
use cplib_lang_io::{fastout, out, outln};

#[fastout]
fn main() -> std::io::Result<()> {
    out!("main ");
    helper();
    let closure = || outln!("closure");
    closure();
    std::thread::spawn(|| outln!("thread")).join().unwrap();
    let mut writer = cplib_lang_io::stdout();
    cplib_lang_io::write_line(&mut *writer, [1, 2, 3])?;
    Ok(())
}

fn helper() {
    out!("helper ");
}
