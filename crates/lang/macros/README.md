# macros

`swap!` / `chmin!` / `chmax!` / `mat!` と、再帰・呼び出し時借用のマクロ。
追加の依存クレートや手続きマクロは不要。`use macros::{recurfn, lazyfn, lazymac};` で使う。
`cplib-lang-macros` という依存名なら `use cplib_lang_macros::{...};`。

| マクロ | 用途 | 呼び出し | 借用・制御フロー |
| --- | --- | --- | --- |
| `recurfn!` | 再帰するクロージャを保持する | `f(args)` | 可変状態は第1引数または `[変数名, ...]` で指定する |
| `lazyfn!` | 呼び出したときだけ外部変数を借用する | `f!(args)` / `f!(fn)` | 可変キャプチャ可。`return` / `?` はクロージャ内に作用 |
| `lazyfn!(recur ...)` | 呼び出し時借用と再帰の組合せ | 同上 | 再帰時の可変状態は第1引数。読み取りキャプチャは呼び出しの間だけ |
| `lazymac!` | Mo の追加・削除など、本体をその場へ展開する | `f!(args)` | 型省略可。`return` / `?` / `break` / `continue` は呼び出し元に作用 |

## 原案から変えたところ

元の raw pointer を使う方式では、再帰のたびに同じ `FnMut` への可変参照が重なる。
ライブラリ版は共有参照の `Fn` で再帰し、変更する値を作業ごとに可変借用する。
参照を再帰の前後で保持できるかは Rust が検査する。読み取り専用のグラフなどはキャプチャできる。

`lazyfn!` は通常の可変キャプチャも使いやすいよう、非再帰を標準とし、再帰版だけ `recur` を明示する。
引数式は左から1回ずつ、全て評価してから仮引数を束縛する。`f!(1, x)` の `x` が
第1引数の仮引数名と同じでも、後続の実引数がすり替わらない。

## 再帰と可変状態

外側の変数を更新するときは名前を `[...]` に列挙する。呼び出し側は `f(args)`、
本体内の再帰は `f!(args)` と書く。本体内で列挙した変数は `&mut` として扱う。

```rust
use macros::recurfn;

let mut visited = Vec::new();
let mut count = 0;
recurfn!(dfs = [visited, count] |n: usize| {
    visited.push(n);
    *count += 1;
    if n > 0 { dfs!(n - 1); }
});
dfs(2);
assert_eq!(visited, [2, 1, 0]);
assert_eq!(count, 3);
```

呼び出すたびに渡す状態を替えたいときは、最初の引数に `state: &mut T` と書く。
この形式での再帰呼び出しは `f(state, ...)`。

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

元の raw pointer 方式も `recurfn_unsafe!` で使える。呼び出しを `unsafe { ... }` 内に
書く必要がある。この方式はクロージャを移動すると保存したポインタが無効になり、
同じ `FnMut` への再入自体も未定義動作になりうる。通常の再帰には `recurfn!` を使う。

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

`lazyfn!` に複数の定義を並べる場合は `;` または `,` で区切る。
各呼び出しの間だけ状態を借りるので、同じ変数を使う処理をまとめて書ける。

```rust
use macros::lazyfn;

let mut total = 0;
lazyfn!(
    add = |n: i64| { total += n; };
    get = || -> i64 { total };
);
add!(3);
assert_eq!(get!(), 3);
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

`swap!(a, b)` は変数やフィールドを交換する。`Copy` は不要で、各式は1回だけ評価する。
同じ配列・`Vec` 内の交換には `swap!(a; i, j)` を使う。添字が同じ場合にも使える。

```rust
use macros::swap;
let (mut a, mut b) = (1, 2);
swap!(a, b);
assert_eq!((a, b), (2, 1));
let mut xs = vec![10, 20, 30];
swap!(xs; 0, 2);
assert_eq!(xs, [30, 20, 10]);
```

`chmin!` / `chmax!` は比較候補と更新先の式をそれぞれ1回だけ評価する。
候補を先に計算するので、`chmin!(dist[v], dist[u] + 1)` のように同じ配列から読める。
多次元配列の `mat!` は、マクロだけを import した場合や依存名を変更した場合も使える。

使用例は `playground/src/bin/closure-macros.rs`。
`cargo run -p cplib-playground --bin closure-macros` で実行し、`just bundle closure-macros` で単一ファイル化できる。
