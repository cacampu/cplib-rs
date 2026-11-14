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

/// 共通バッファへ改行なしで出力する。
#[macro_export]
macro_rules! out {
    ($($args:tt)*) => { $crate::__print_args(format_args!($($args)*)) };
}

/// 共通バッファへ改行付きで出力する。
#[macro_export]
macro_rules! outln {
    () => { $crate::__print_args(format_args!("\n")) };
    ($($args:tt)*) => {
        $crate::__print_args(format_args!("{}\n", format_args!($($args)*)))
    };
}
