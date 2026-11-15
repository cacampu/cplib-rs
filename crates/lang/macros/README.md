# macros

`chmin!` / `chmax!` / `mat!` と、再帰・呼び出し時借用のマクロ。
追加の依存クレートや手続きマクロは不要。`use macros::{recurfn, lazyfn, lazymac};` で使う。
`cplib-lang-macros` という依存名なら `use cplib_lang_macros::{...};`。

| マクロ | 用途 | 呼び出し | 借用・制御フロー |
| --- | --- | --- | --- |
| `recurfn!` | 再帰するクロージャを保持する | `f(args)` | 読み取りキャプチャはクロージャの最終使用まで。可変状態は引数で渡す |
| `lazyfn!` | 呼び出したときだけ外部変数を借用する | `f!(args)` / `f!(fn)` | 可変キャプチャ可。`return` / `?` はクロージャ内に作用 |
| `lazyfn!(recur ...)` | 呼び出し時借用と再帰の組合せ | 同上 | 再帰時の可変状態は第1引数。読み取りキャプチャは呼び出しの間だけ |
| `lazymac!` | Mo の追加・削除など、本体をその場へ展開する | `f!(args)` | 型省略可。`return` / `?` / `break` / `continue` は呼び出し元に作用 |

## 原案から変えたところ

元の `closure_macros.rs` の raw pointer で `FnMut` を再帰呼び出しする方式は採用していない。
同じクロージャへの可変参照を再帰時に重ねる問題を避けるため、ライブラリ版は共有参照の `Fn` で再帰する。
ライブラリと生成コードに `unsafe` はない。クロージャを別の変数に移動して呼んでも自己ポインタは壊れない。

このため、再帰中に変える配列などは外部から直接可変キャプチャせず、最初の引数に `state: &mut T` と書く。
本体では通常の `f(state, ...)` で再帰する。参照を再帰の前後で保持できるかは Rust が検査する。
状態にはスライス、タプル、独自構造体などを使え、読み取り専用のグラフなどは引き続きキャプチャできる。

`lazyfn!` は通常の可変キャプチャも使いやすいよう、非再帰を標準とし、再帰版だけ `recur` を明示する。
引数式は左から1回ずつ、全て評価してから仮引数を束縛する。`f!(1, x)` の `x` が
第1引数の仮引数名と同じでも、後続の実引数がすり替わらない。

## 再帰と可変状態

```rust
use macros::recurfn;

let g = [vec![1, 2], vec![], vec![]];
recurfn!(dfs = |sub: &mut [usize], u: usize| -> usize {
    sub[u] = 1;
    for &v in &g[u] {
        sub[u] += dfs(sub, v);
    }
    sub[u]
});
let mut sub = [0; 3];
assert_eq!(dfs(&mut sub, 0), 3);
```

引数の型は必須で、戻り値を省略すると `()`。引数なしの `||` にも対応する。
再帰は普通の関数と同じくコールスタックを消費する。

## 呼び出しの間だけ借用する

```rust
use macros::lazyfn;

let mut limit = 10;
let mut calls = 0;
lazyfn!(check = |x: i64| -> bool {
    calls += 1;
    x * x <= limit
});
assert!(check!(3));
limit = 5;
assert!(!check!(3));
assert_eq!(calls, 2);
```

`check!(fn)` は `FnMut` を受け取る二分探索などへそのまま渡せる。
クロージャ値を変数に保持した場合は、その値の最終使用まで借用が続く。

```rust
use macros::lazyfn;

let mut weight = 2i64;
lazyfn!(recur fill = |xs: &mut [i64], i: usize| {
    if i == xs.len() { return; }
    xs[i] = weight;
    fill(xs, i + 1);
});
let mut xs = [0; 3];
fill!(&mut xs, 0);
weight = 5;
fill!(&mut xs, 1);
assert_eq!(xs, [2, 5, 5]);
```

ローカルマクロは定義以降のテキストスコープで使う。別関数へ渡す場合は `name!(fn)` を使う。
`lazymac!` はクロージャ値を返さず、再帰もできない。

## 既存のヘルパー

`chmin!` / `chmax!` は比較候補と更新先の式をそれぞれ1回だけ評価する。
候補を先に計算するので、`chmin!(dist[v], dist[u] + 1)` のように同じ配列から読める。
多次元配列の `mat!` は、マクロだけを import した場合や依存名を変更した場合も使える。

使用例は `playground/src/bin/closure-macros.rs`。
`cargo run -p cplib-playground --bin closure-macros` で実行し、`just bundle closure-macros` で単一ファイル化できる。
