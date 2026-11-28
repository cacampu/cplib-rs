# io

トークン入力、独自型の読み取り、共通出力バッファをまとめた競プロ用 I/O。
proconio 風の宣言形式に加え、`input!(型)` を値を返す式として使える。

```rust
use io::{input, outln};

fn main() {
    io::run(solve);
}

fn solve() {
    let n = input!(usize);
    input! { mut a: [i64; n] }
    a.sort_unstable();
    outln!(a);
}
```

## 入力の構文

```rust,ignore
input! {
    n: usize,
    mut values: [i64; n],
    edges: [(Usize1, Usize1); n - 1],
    matrix: [[i64; 3]; n],
    lists: [[usize]; n], // 各リストの直前から長さを読む
    s: Chars,
}
let x = input!(i64);
let pair = input!((usize, i64));
let values = input!([i64; n]);
let values = read_value!([i64; n]); // 同じ式形式
```

`Chars`、`Bytes`、`Usize1`、`Isize1` は `io::marker` から読み込む。
入力マクロの `[T; n]` は proconio と同じく `Vec` を作る。
固定長配列を型として読みたい場合は、型エイリアスや `source.read::<[T; N]>()` を使う。
タプル・配列・マーカー・型エイリアス・`mut`・分解パターン・末尾カンマに対応する。
実行時の読み取り指定は `input!(with reader)` / `input! { value: with reader }`。

## 入力元を明示する

```rust
use io::{Scanner, input};
let mut source = Scanner::new("2 1 2 3 4 99");
input! { from &mut source, n: usize, points: [(i64, i64); n] }
assert_eq!(points, [(1, 2), (3, 4)]);
assert_eq!(input!(from &mut source, usize), 99);
```

標準入力はライブラリ内の `pub static INPUT: InputBuffer` を共有する。
`input!`、`input!(from &io::INPUT, ...)`、`io::INPUT.read::<T>()` はすべて同じ読み取り位置を使う。
`Grid::read(h, w, &io::INPUT)` のように、各関数から同じバッファを直接渡せる。
ロックは読み取り操作の間だけ保持し、ローカルのガードを関数間で渡し回す必要はない。

```rust,ignore
use io::{INPUT, input};
fn solve() {
    let (h, w) = input!((usize, usize));
    let g = read_grid(h, w);
    let next = INPUT.read::<usize>();
}
fn read_grid(h: usize, w: usize) -> grid::Grid<i64> {
    grid::Grid::read(h, w, &INPUT)
}
```

テスト用には `InputBuffer::from(Scanner::new("..."))` で独立した共有バッファも作れる。
`InputSource` が共有バッファとローカルな入力元の違いを吸収し、低レベルの `Readable` は
取得された `Source` から読む。複数トークンの値やグリッド全体は、1回のロックで読み取る。

まとめてロックを保持したい場合には従来の `let mut source = io::stdin();` も使える。
そのガード保持中だけは `from &mut source` を使い、同じバッファを再ロックしないこと。
ローカル入力を再利用する場合も `from &mut source` と書く (`from source` は入力元をムーブする)。

共有の標準入力は、保持しているトークンを使い切ったときに次の行を読む。
入力バッファに読み込み済みの行があればそれを使い、標準入力から補充する直前に出力を flush する。
対話でも同じ `io::run` と `outln!`、`input!` を使える。
空行を読み飛ばし、EOF までに必要なトークンがなければエラーになる。

```rust,ignore
outln!("? ...");
let response = input!(i64);
```

EOF まで一括で読み込みたい場合は `Scanner::from_reader(std::io::stdin().lock())` で入力元を作り、
`input!(from &mut source, ...)` に渡す。自分で作る `LineSource` は任意の `BufRead` を逐次読む。
自前の入力元で対話する場合は、応答を待つ前に `io::flush()` する。

## Vector と Grid

```rust,ignore
use io::INPUT;
use vector::Vector;
use grid::Grid;
type P = Vector<i64, 2>;

input! { n: usize, points: [P; n] } // 各 P が X と Y の2トークンを消費
let (h, w) = input!((usize, usize));
let numbers = Grid::<i64>::read(h, w, &INPUT);
// 別形式の入力ではこちらを使う:
let letters = Grid::read_chars(h, w, &INPUT); // .#.. のような行
let bytes = Grid::read_bytes(h, w, &INPUT);
```

共有バッファには `&INPUT`、ローカルな `Scanner` / `LineSource` には `&mut source` を渡す。
`Grid::read` は `h*w` 個の値を読み、改行位置には依存しない。
`read_chars` / `read_bytes` は行ごとのトークン幅を検査する。

独自型は `Readable` を実装する。`FromStr` は文字列の解釈、`Readable` はトークン消費として独立している。
`T: FromStr` 全般への一括実装はないため、両方を実装できる。
独自の1トークン型を `FromStr` だけで読む場合は `io::marker::Token<T>` を指定する。

## 出力・複数ケース

`out!(value)` / `outln!(value)` は `Printable` に従って値を出力する。
`Vec`・スライス・配列・タプルは空白区切り、`Option::None` は `-1`、`Some(x)` は `x`。
複数の値を渡した場合も空白で区切る。引数は借用するため、出力した後も変数を使える。

```rust
use io::{outln, separated, lines, option_or, yes_no};
let a = vec![Some(1), None, Some(3)];
outln!(a);                        // 1 -1 3
outln!(separated(&a, ","));       // 1,-1,3
outln!(lines(&a));                // 1、-1、3 を各行に出力
outln!(42, Some(7));              // 42 7
outln!(yes_no(true));             // Yes
io::print_yes_no(false);          // No と改行を直接出力
outln!(option_or(&None::<i64>, "Impossible"));
outln!("{:.2}", 1.234);           // 1.23: 従来の書式指定も使える
io::flush();
```

先頭が文字列リテラルの場合は標準の `format_args!` と同じ書式指定として扱う。
この形式で配列などを埋め込むときは `outln!("answer: {}", io::printable(&a))` と書く。
文字列自体を値として出したい場合は `outln!(("{literal}"))` のように括弧で包める。
`out!()` は何も出さず、`outln!()` は改行だけを出す。

`separated(&a, separator)` は文字列や文字の区切りに対応し、中間の文字列を作らない。
`lines(&matrix)` は行間を改行にし、各行の配列は空白区切りになる。
`Vector`、`Grid`、静的・動的 `ModInt` にも `Printable` を実装している。
`Grid` は行ごとに改行し、セル間を空白で区切る。
`bool` 自体は `true` / `false`。`yes_no` は `Yes` / `No`、`yes_no_upper` は `YES` / `NO` を返す。

独自型は `Printable` を実装できる。`Display` のみの型には `io::display(value)` を使う。
`io::print(value)` / `io::println(value)` は関数版、`io::printable(value)` は
標準の `format!` / `write!` でも使えるアダプタ。
既存の `line` / `joined` / `matrix` と `write_*` も `Printable` に対応する。

```rust
use io::{debug, separated};
let a = vec![Some(1), None];
debug!(a);                       // stderr に 1 -1 と改行
debug!(separated(&a, ","));       // stderr に 1,-1 と改行
debug!("a={a:?}");               // 標準の書式指定
io::debug(&a);                   // 関数版も同じ Printable の形式
```

`debug!` は `Printable` の出力形式を優先し、`Debug` だけを実装した型には `{:?}` を使う。
たとえば `#[derive(Debug)]` を付けた独自型の `Vec` も `debug!(v)` で出力できる。
実行プロセスに `CP_LOCAL=1` が設定されていると出力する。
デバッグ出力だけ有効にする場合は `CPLIB_DEBUG=1` を使う。
`debug!` は無効時に引数式も評価しない。関数版の `debug` / `debug_line` /
`debug_matrix` では通常の関数と同じく引数式を評価する。

`local! { ... }` は `CP_LOCAL=1` のときだけブロックを実行する。
`io::is_local()` で同じ条件を関数として使える。

```rust,ignore
use io::{debug, local};

debug!(values);
local! {
    for n in 2..=2000 {
        assert!(judge(n, &ans), "WA: {n}");
    }
}
```

`out!` / `outln!` と `io::print` / `io::println` は、ヘルパー関数・クロージャを
含むすべての呼び出し元で同じ `BufWriter` を共有する。`io::run(solve)` の中では
出力をまとめ、最後のバッファリングスコープを抜けるときに flush する。スコープ外では
呼び出しごとに flush する。`#[fastout]` は付けた関数を抜ける時点で flush する。
入力待ち以外のタイミングで送信したい場合や、`std::process::exit` の前には `io::flush()` を呼ぶ。
`io::stdout()` で直接ライターを借りる場合は、標準入力や出力の前にガードを解放する。

```rust
use io::{input, multi_testcase, outln};

fn main() {
    io::run(solve);
}

#[multi_testcase]
fn solve() {
    let n = input!(usize);
    if n == 0 {
        outln!("empty");
        return; // このケースだけ終了。後続ケースは処理する
    }
    let a = input!([i64; n]);
    outln!("{}", a.iter().sum::<i64>());
}
```

`#[multi_testcase]` は先頭のケース数も同じスキャナから読むため、ケース数の後の改行は必須ではない。
返り値が `()` の通常の関数に使い、全ケースで一つのバッファを共有する。
単独で呼べば全ケース終了時に flush し、外側に `io::run` があればそのスコープまで出力をまとめる。
以前の `multi-testcase` クレートからも属性を引き続き import できるが、実行時には `io` の依存が必要。

Cargo.toml で `io` を `cplib-lang-io` などへ改名した場合も、手続きマクロは `proc-macro-crate` で
依存名を解決する。属性では `#[multi_testcase(crate = cplib_io)]` のような明示指定も使える。
