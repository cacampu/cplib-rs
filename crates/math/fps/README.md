# fps

`Fps<M>` は `ModInt<M>` の係数列を持つ形式的冪級数。法は NTT に対応した素数を使う。
末尾の零係数は保持し、`pre(n)`、`mul_truncated(rhs, n)`、`div_series(rhs, n)` などは
ちょうど `n` 項を返す。`f[i]` で係数を読み書きできる。

`inv` / `div_series` は分母の定数項が非零、`log` は定数項が 1、`exp` は定数項が 0 であること。
積分・log・exp を使うときは必要な整数の逆元が存在する範囲 (`n < M`) に収める。
NTT の変換長も `M - 1` の約数である必要がある。

## FPS 24 を見て追加した機能

[問題文](https://atcoder.jp/contests/fps-24/tasks_print?lang=ja) と
[公式解説](https://atcoder.jp/contests/fps-24/editorial?lang=ja) を参照。
各問題の数え上げの式そのものは利用側で組み立て、共通する計算をライブラリで扱う。

| 問題 | 主な計算 | 対応する API・実装範囲 |
| --- | --- | --- |
| A お菓子 | 二項係数の積の和 | `Factorials::binom` |
| B 整数の組 | 有理式の整理 | 式を整理すると答えは `N+1` |
| C 数列 | 二項係数・負の二項級数 | `Factorials::binom`、`pow` |
| D 数列 2 | 二項係数の積の和 | `Factorials::binom` |
| E 数列 3 | 指数型母関数の積 | `Factorials::inv_fact`、`mul_truncated` |
| F 色紙 | 指数型母関数の偶奇分離 | 式の整理後に `ModInt::pow` |
| G 硬貨 | `(1-x^k)` との乗除算 | `mul_binomial_assign`、`div_binomial_assign` |
| H ジャンプ | 二変数母関数の係数を二項係数に整理 | `Factorials::binom` |
| I スコア | 多数の一次式の総積 | `Iterator::product::<Fps<M>>()` |
| J スゴロク | 各項を補正する畳み込み DP | `convolution_dp` |
| K 順列 | FPS の逆元 | `inv` |
| L 順列 2 | 連結成分の集合を exp で数える | `exp`、`Factorials` |
| M 連結グラフ | 全グラフの EGF に log を適用 | `log`、`Factorials` |
| N 硬貨 2 | 積の log を調和級数で構築して exp | `exp`、`inverse_table` |
| O 根付き木 | FPS の整数冪 | `pow`、`Factorials`。素数列挙は別途行う |
| P ボール | 多項式の多点評価 | `multipoint_eval` |
| Q サイコロ | 冪和の列挙と EGF の積 | `power_sums`、`Factorials`、`mul_truncated` |
| R ランダムウォーク | `x^n-1` を法とする累乗 | `pow_cyclic` |
| S ゲーム | exp を含む方程式の Newton 法 | `exp`、`inv`。Newton 更新式は問題ごとに記述 |
| T カラフル | 有理式の和・巨大な次数の係数 | `sum_rationals`、`bostan_mori`。部分点には BM-BM も使える |
| U 録画機 | 行列の母関数・二変数係数の列挙 | 専用の分割統治は未追加 |
| V 12 方向 | 二項係数の変形と畳み込み | `Factorials`、`mul_truncated`。座標の変換は利用側で行う |
| W 閉路 | 集合冪級数と連結・二重頂点連結変換 | SPS は通常の FPS と別の構造なので未追加 |
| X 関数的平方根 | 関数合成・power projection・BBLA | 専用のアルゴリズムは未追加。通常の `sqrt` とは異なる |

G/I/J/P/Q/R/T は両方の公開サンプルを `tests/fps24.rs` で検証する。
その他の追加機能は `tests/advanced.rs` で素朴な計算との比較・恒等式を検証する。
この表は各問題の解法に必要な操作の対応表で、全24問の提出・AC を確認したものではない。

## 使い分け

- `mul_sparse` / `div_sparse`: `(次数, 係数)` の列で疎な式を渡す。順序・重複次数の制約はなく、
  計算量は項数 `n` と疎な式の項数 `k` に対して `O(nk)`。
- `mul_binomial_assign(k, c)` / `div_binomial_assign(k, c)`: `(1+c*x^k)` との乗除算を
  `O(n)`、追加配列なしで行う。保持している項数を変えないため、先に `pre(n)` で精度を揃える。
- `Iterator::product`: 多項式を短いものから畳み込み、総積を求める。空積は 1。
- `sum_rationals`: `(分子, 分母)` の列から和の `(分子, 分母)` を作る。約分は行わない。
- `power_sums(values, n)`: `sum(a_i^k)` を `k=0..n` についてまとめて返す。第0項は要素数。
- `multipoint_eval`: 同じ多項式を多数の点で評価する。点の重複も可。
  `interpolate(points, values)` は逆に多項式を復元する。こちらの点は法 `M` で相異なる必要がある。
- `bostan_mori(p, q, n)`: `p/q` の第 `n` 係数だけを求める。`n` は `u64`。
  分母の定数項が非零なら、分子の次数が分母以上でもよい。
- `berlekamp_massey`: 接頭辞から最小次数の線形漸化式を復元する。
  `linear_recurrence(initial, coefficients, n)` へそのまま渡せる係数順で返す。
  係数 `c[j]` は `a[i-1-j]` に対応する。
- `pow_cyclic(e, n)`: 長さ `n` の NTT 上で累乗する。`n` は正の2冪かつ `M-1` の約数。
  `pow(e, n)` が高次の項を捨てるのに対し、こちらは次数を `n` で割った余りへ折り返す。

指数型母関数では個数 `a[i]` に `inv_fact[i]` を掛けて FPS の係数にし、
求めた係数に `fact[i]` を掛けて個数に戻す。

```rust
use fps::{berlekamp_massey, linear_recurrence};
use modint::ModInt998244353 as Mi;

let prefix = [0, 1, 1, 2, 3, 5, 8, 13].map(Mi::new);
let c = berlekamp_massey(&prefix);
let answer = linear_recurrence(&prefix[..c.len()], &c, 1_000_000_000_000_000_000);
```

畳み込み DP のコールバックは添字の昇順に1回ずつ呼ばれる。
J のように終端到達時の値を集計した後で伝播を止める場合は、外の変数へ加算して 0 を返す。
具体例は `tests/fps24.rs` の `sugoroku` を参照。
