use super::{Printable, printable};
use std::fmt;
use std::io::{self, BufWriter, Stdout, Write};
use std::sync::{LazyLock, Mutex, MutexGuard};

static STDOUT: LazyLock<Mutex<BufWriter<Stdout>>> =
    LazyLock::new(|| Mutex::new(BufWriter::new(io::stdout())));

/// 全関数・スレッドで共有する出力バッファ。ガード保持中は `out!` などを再び呼ばないこと。
pub fn stdout() -> MutexGuard<'static, BufWriter<Stdout>> {
    STDOUT.lock().expect("stdout lock poisoned")
}

pub fn flush() {
    stdout().flush().expect("failed to flush stdout");
}

#[doc(hidden)]
pub fn print_args(args: fmt::Arguments<'_>) {
    stdout().write_fmt(args).expect("failed to write stdout");
}

#[doc(hidden)]
pub fn print_args_ln(args: fmt::Arguments<'_>) {
    let mut writer = stdout();
    writer.write_fmt(args).expect("failed to write stdout");
    writer.write_all(b"\n").expect("failed to write stdout");
}

/// 値を Printable の形式で出力する。
pub fn print(value: impl Printable) {
    print_args(format_args!("{}", printable(value)));
}

/// 値を Printable の形式で改行付きで出力する。
pub fn println(value: impl Printable) {
    print_args_ln(format_args!("{}", printable(value)));
}

#[doc(hidden)]
pub fn debug_args(args: fmt::Arguments<'_>) {
    let mut writer = io::stderr().lock();
    writer.write_fmt(args).expect("failed to write stderr");
    writer.write_all(b"\n").expect("failed to write stderr");
}

/// スコープ終了時に共通の出力バッファを flush するガード。
/// `#[fastout]` / `#[multi_testcase]` はこれを自動で作る。
#[must_use = "bind this guard to keep it alive until the end of the scope"]
pub struct FlushGuard(());

pub fn flush_guard() -> FlushGuard {
    FlushGuard(())
}

impl Drop for FlushGuard {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // 元のパニックを出力エラーで上書きしない。
            if let Ok(mut writer) = STDOUT.lock() {
                let _ = writer.flush();
            }
        } else {
            flush();
        }
    }
}
