use super::{Printable, printable};
use std::fmt;
use std::io::{self, BufWriter, Stdout, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock};

static STDOUT: OnceLock<Mutex<BufWriter<Stdout>>> = OnceLock::new();
static BUFFERING_SCOPES: AtomicUsize = AtomicUsize::new(0);

/// 関数の実行中は出力をバッファリングし、最後の実行スコープを抜けるときに flush する。
/// 標準入力の補充前にも自動で flush するので、対話問題でも同じ入口を使える。
pub fn run<R>(solve: impl FnOnce() -> R) -> R {
    let _guard = FlushGuard::new(false);
    solve()
}

fn lock_stdout() -> MutexGuard<'static, BufWriter<Stdout>> {
    STDOUT
        .get_or_init(|| Mutex::new(BufWriter::new(io::stdout())))
        .lock()
        .expect("stdout lock poisoned")
}

/// 全関数・スレッドで共有する出力バッファ。標準入力や出力の前にガードを解放する。
/// 直接書き込む場合は `run` のスコープ内で使うか、ライターを明示的に flush する。
pub fn stdout() -> MutexGuard<'static, BufWriter<Stdout>> {
    lock_stdout()
}

pub fn flush() {
    lock_stdout().flush().expect("failed to flush stdout");
}

pub(crate) fn flush_if_initialized() {
    // 入力しか行っていないときは stdout の初期化もロックも不要。
    if let Some(stdout) = STDOUT.get() {
        stdout
            .lock()
            .expect("stdout lock poisoned")
            .flush()
            .expect("failed to flush stdout");
    }
}

#[doc(hidden)]
pub fn print_args(args: fmt::Arguments<'_>) {
    write_stdout(args, false);
}

#[doc(hidden)]
pub fn print_args_ln(args: fmt::Arguments<'_>) {
    write_stdout(args, true);
}

fn write_stdout(args: fmt::Arguments<'_>, newline: bool) {
    let mut writer = lock_stdout();
    writer.write_fmt(args).expect("failed to write stdout");
    if newline {
        writer.write_all(b"\n").expect("failed to write stdout");
    }
    // 最後のスコープも同じロックを取得して flush する。
    if BUFFERING_SCOPES.load(Ordering::Relaxed) == 0 {
        writer.flush().expect("failed to flush stdout");
    }
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

/// 実行プロセスが `CP_LOCAL=1` を受け取ったかを返す。
pub fn is_local() -> bool {
    static LOCAL: LazyLock<bool> =
        LazyLock::new(|| std::env::var_os("CP_LOCAL").is_some_and(|value| value == "1"));
    *LOCAL
}

pub fn debug_enabled() -> bool {
    static DEBUG: LazyLock<bool> =
        LazyLock::new(|| std::env::var_os("CPLIB_DEBUG").is_some_and(|value| value == "1"));
    is_local() || *DEBUG
}

/// スコープ終了時に共通の出力バッファを flush するガード。
/// `#[fastout]` はこれを自動で作る。
#[must_use = "bind this guard to keep it alive until the end of the scope"]
pub struct FlushGuard {
    flush_at_scope_end: bool,
}

pub fn flush_guard() -> FlushGuard {
    FlushGuard::new(true)
}

impl FlushGuard {
    fn new(flush_at_scope_end: bool) -> Self {
        BUFFERING_SCOPES.fetch_add(1, Ordering::Relaxed);
        Self { flush_at_scope_end }
    }
}

impl Drop for FlushGuard {
    fn drop(&mut self) {
        let last_scope = BUFFERING_SCOPES.fetch_sub(1, Ordering::Relaxed) == 1;
        if !last_scope && !self.flush_at_scope_end {
            return;
        }
        if std::thread::panicking() {
            // 元のパニックを出力エラーで上書きしない。
            if let Some(Ok(mut writer)) = STDOUT.get().map(Mutex::lock) {
                let _ = writer.flush();
            }
        } else {
            flush_if_initialized();
        }
    }
}
