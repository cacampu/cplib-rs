# cplib を実問題で試す

このパッケージはワークスペース内の cplib に `path` で依存する。
ライブラリを編集したら、そのままビルド・サンプルテスト・提出用コードの生成に反映される。
GitHub への push、依存更新ワークフロー、`cargo update` は不要。
AtCoder と同じ `indexmap = "=2.11.0"` も依存に含め、入出力マクロの間接依存と
衝突しないことをビルド時に検証する。

## 用意した例

| エイリアス | 問題 | 見るところ |
| --- | --- | --- |
| `vector` | [ABC234 B](https://atcoder.jp/contests/abc234/tasks/abc234_b) | `type P = Vector<i64, 2>` と `a: [P; n]` |
| `grid` | [ABC007 C](https://atcoder.jp/contests/abc007/tasks/abc007_3) | `Grid::read_bytes(h, w, &INPUT)`、近傍を走査しながら距離を更新 |
| `numeric-grid` | [ABC356 B](https://atcoder.jp/contests/abc356/tasks/abc356_b) | 別関数から `Grid::read(h, w, &INPUT)`、`all_pos()` |
| `multi-testcase` | [ARC163 A](https://atcoder.jp/contests/arc163/tasks/arc163_a) | ケース数を共通入力から読む属性、ケースごとの `return`、別関数からの出力 |

これらは `cargo compete add` で追加し、公開サンプルを保存したもの。
`src/main.rs` は問題に紐づかない試行用、`src/bin/io-runtime.rs` は出力共有の例。
`src/bin/shared-input.rs` は `#[multi_testcase]` → `solve()` → グリッド読み取り関数へ
処理を分けても `&INPUT` で同じバッファを使う例。入力元を引数に渡し回す必要はない。
`src/bin/closure-macros.rs` は `recurfn!` / `lazyfn!` / `lazymac!` と二分探索・I/O の組合せ。
`cargo run -p cplib-playground --bin closure-macros` で実行し、`just bundle closure-macros` で単一ファイル化できる。
`src/bin/output.rs` は `swap!`、`Vec<Option<T>>`、区切り指定、Yes/No、
`Vector`・`Grid`・`ModInt` とデバッグ出力の例。
`cargo run -p cplib-playground --bin output` と `just bundle output` で試せる。

## 使い方

リポジトリのルートから:

```sh
just try vector
just try grid
just try numeric-grid
just try multi-testcase
just bundle vector  # target/vector-bundled.rs を生成し、コンパイルも検証
```

直接 cargo-compete を使う場合:

```sh
cd playground
cargo compete add https://atcoder.jp/contests/abc145/tasks/abc145_c traveling
# src/bin/traveling.rs を編集
cargo compete test traveling
```

ルートの `just problem URL ALIAS` からも追加できる。生成されるコードは自前の I/O を使う。
初期設定は Rust 1.89.0、テストは release ビルド。追加した問題が誤差判定を要求する場合は
`testcases/ALIAS.yml` の `match` を `Float` に変更する (`vector.yml` を参照)。
cargo-compete の警告にあるとおり、ABC の誤差条件を自動取得できないことがある。

追加のライブラリが必要なら、そのクレートをローカルのパスで追加する:

```sh
cd playground
cargo add --rename cplib-math-fps --path ../crates/math/fps
```

依存宣言の一覧はルートで `just local-deps`。全ライブラリを初めから追加する必要はない。

## 提出用の単一ファイル

`just bundle ALIAS` はローカルの変更を含む単一ファイルを `target/ALIAS-bundled.rs` に作る。
cargo-equip がコードをまとめ、生成コードをコンパイルしてから保存する。
インストール済みの cargo-equip はプロシージャルマクロの展開プロセスに Cargo の環境変数を
渡さないため、レシピと `[submit]` 設定で `CARGO_MANIFEST_DIR` を明示している。
入出力マクロの依存名解決も、まとめた後のクレート参照も実際の生成・コンパイルで検証する。

現在インストールされている cargo-equip では、手続きマクロを通る文字列リテラルの
`\n` などが二重にエスケープされる場合がある。今回の `outln!` の改行は実行時に付けるため
影響しないが、`#[fastout]` などの本体にエスケープを含むリテラルを直接書く場合は、
生成コードの実行結果も確認すること。

提出するときは `playground` 内で `cargo compete submit ALIAS`。
この設定でも最新のローカル実装を bundle する。サンプルの確認と提出は別の操作。

## 既存の AtCoder 用リポジトリからもローカル実装を使う

既存の Git 依存を維持して開発時だけ手元の cplib に切り替えるなら、Cargo の
[`[patch]`](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section) を使える。
このリポジトリで `just local-patch` を実行すると、次の形式の設定を出力する:

```toml
[patch."https://github.com/cacampu/cplib-rs"]
io = { path = "/path/to/cplib-rs/crates/lang/io" }
vector = { path = "/path/to/cplib-rs/crates/math/vector" }
# ... 全ライブラリ分
```

既存側の `.cargo/config.toml` にこの表を置くと、Cargo を起動する cargo-compete /
cargo-equip もローカル実装を使う。変更直後の確認に GitHub の更新待ちは入らない。
絶対パスは各マシン固有なので共有設定とは分けて管理する。
切り替え時に以前の Git ソースがロックに残る場合だけ、既存側で `cargo update` して切り替える。
今回、参照先の AtCoder 用リポジトリのファイルは変更していない。
