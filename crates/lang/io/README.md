# io

トークン入力、独自型の読み取り、共通出力バッファをまとめた競プロ用 I/O。
proconio 風の宣言形式に加え、`input!(型)` を値を返す式として使える。

```rust
use io::{fastout, input, outln};

#[fastout]
fn main() {
    let n = input!(usize);
    input! { mut a: [i64; n] }
    a.sort_unstable();
    outln!("{}", io::line(a));
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

標準入力には `let mut source = io::stdin();` でアクセスできる。
これは共通スキャナのロックガードで、そのまま入力元として渡せる。
ガード保持中はデフォルトの `input!` で再ロックせず、`from &mut source` を使う。
マクロの通常形式では、1回の呼び出しの終了時にロックを解放する。

`Scanner` は入力を一括で読む。対話問題では次のように逐次入力元を明示する。

```rust,ignore
let mut source = io::LineSource::new(std::io::BufReader::new(std::io::stdin()));
loop {
    outln!("? ...");
    io::flush();
    let response = input!(from &mut source, i64);
    // ...
}
```

## Vector と Grid

```rust,ignore
use vector::Vector;
use grid::Grid;
type P = Vector<i64, 2>;

input! { n: usize, points: [P; n] } // 各 P が X と Y の2トークンを消費
let mut source = io::stdin();
let (h, w) = input!(from &mut source, (usize, usize));
let numbers = Grid::<i64>::read(h, w, &mut source);
// 別形式の入力ではこちらを使う:
let letters = Grid::read_chars(h, w, &mut source); // .#.. のような行
let bytes = Grid::read_bytes(h, w, &mut source);
```

入力位置を進めるので入力元は `&mut`。読み取り中に別途グローバル入力を取得する必要はない。
`Grid::read` は `h*w` 個の値を読み、改行位置には依存しない。
`read_chars` / `read_bytes` は行ごとのトークン幅を検査する。

独自型は `Readable` を実装する。`FromStr` は文字列の解釈、`Readable` はトークン消費として独立している。
`T: FromStr` 全般への一括実装はないため、両方を実装できる。
独自の1トークン型を `FromStr` だけで読む場合は `io::marker::Token<T>` を指定する。

## 出力・複数ケース

`out!` / `outln!` は全関数・スレッドで同じ `BufWriter` を使う。
`#[fastout]` はそのバッファを関数終了時に flush するだけで、ローカル変数を捕捉する
`print!` マクロは作らない。ヘルパー関数やクロージャからも `outln!` を使える。
属性なしの場合は最後に `io::flush()` を呼ぶか、`let _guard = io::flush_guard();` を置く。
`std::println!` はこのバッファを通さないので、出力は `outln!` に揃える。

```rust
use io::{input, multi_testcase, outln};

#[multi_testcase]
fn main() {
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
返り値が `()` の通常の関数に使い、最後に共通出力を flush する。`#[fastout]` の併用は不要。
以前の `multi-testcase` クレートからも属性を引き続き import できるが、実行時には `io` の依存が必要。

Cargo.toml で `io` を `cplib-lang-io` などへ改名した場合も、手続きマクロは `proc-macro-crate` で
依存名を解決する。属性では `#[multi_testcase(crate = cplib_io)]` のような明示指定も使える。
