//! 差分更新型ビームサーチ。
//!
//! source: hlib-rs `beam/src/beam/`
//! rev:    (未リリース)
//!
//! 状態を1つだけ持ち、探索木を `apply` / `undo` で移動しながら走査する。状態のコピーは
//! 起きない。**ここに書いてあるのは使い方で、なぜそう作ったかは hlib-rs の
//! `docs/設計.md` にある。**
//!
//! # 書くもの
//!
//! 問題側が書くのは [`Logic`] の実装 1 つだけ。実装型は値を持たない空型でよい。
//!
//! ```ignore
//! enum MyLogic {}
//!
//! impl Logic for MyLogic {
//!     type State = Board;        // in-place で更新される唯一の状態
//!     type Context = Input;      // 入力・係数・Zobrist 表・作業領域
//!     type Action = u16;         // 探索木の辺
//!     type Node = Cand;          // 探索木の頂点の荷物
//!     type Value = i32;          // 比較キー
//!     type Hash = u64;           // 重複除去キー
//!
//!     const MINIMIZE: bool = false;
//!
//!     fn root(ctx: &Input) -> (Board, Cand) { .. }
//!     fn apply(state: &mut Board, act: u16, ctx: &Input) { .. }
//!     fn undo(state: &mut Board, act: u16, ctx: &Input) { .. }
//!
//!     fn expand(state: &mut Board, ctx: &Input, cur: &mut impl Cursor<Self>) {
//!         let node = *cur.node();
//!         for (act, child) in 手を列挙(state, ctx, &node) {
//!             cur.push(act, child);
//!         }
//!     }
//!
//!     fn value(node: &Cand, _ctx: &Input) -> i32 { node.score }
//!     fn hash(node: &Cand, _ctx: &Input) -> u64 { node.hash }
//! }
//!
//! let mut search = BeamSearch::<MyLogic, PostorderTour<MyLogic>, _>::new(
//!     ctx, FixedWidth::new(1000));
//! let actions: Vec<u16> = search.run(max_turn);
//! ```
//!
//! 1 ターンは「[`Logic::expand`] で候補を作る → [`Logic::select`] で残すものを選ぶ」の
//! 繰り返し。[`BeamSearch::run`] は最良の葉への手順を返す。
//!
//! # `State` と `Node` と `Action` の分担
//!
//! 判断基準は**可逆性**。`State` は `Action` による群作用として実現されるので `undo` が
//! 定義できる必要があり、`Node` は経路に沿って積むだけなので逆元が要らない。
//!
//! | 置き場所 | 対象 |
//! |---|---|
//! | `State` | 更新も巻き戻しも軽い量。盤面、カウンタ、位置など |
//! | `Node` | 巻き戻しが不可能または高価な量。max の累積、浮動小数点など |
//! | `Action` | 差分更新に必要な最小の識別子。出力にも使う |
//!
//! **浮動小数点を `State` に置いてはいけない。** `y = 1.1 * x` の逆演算が誤差で `x` に
//! 戻らず、静かに壊れる。`Node` 側なら逆演算が要らない。
//!
//! `Action` を細くすると木のコピー量が減るが、そのために情報を `State` へ移すと
//! `apply` / `undo` の維持コストが増える。効くのは辺のサイズそのものではなく**辺 1 本
//! あたりの総仕事量**なので、迷ったら両方測ること。`expand` や `value` から見たい量は
//! `Node` に複製してよい。
//!
//! # バックエンドの選び方
//!
//! [`BeamSearch`] の第2型引数で差し替える。
//!
//! | | 世代スキップ | 速さ | 用途 |
//! |---|---|---|---|
//! | [`PostorderTour`] | 非対応 | 最速 | **既定。** 1 手 1 ターンならこれ |
//! | [`LinkedTree`] | 対応 | 1.5〜2 倍遅い | 手ごとに消費ターンが違う問題 |
//! | [`NaiveCopy`] | 非対応 | `State` 次第 | 正当性の照合先。`State` が小さいときは勝つ |
//!
//! `PostorderTour` と `LinkedTree` は同じ結果を出す（同じ `Logic` なら得点も一致する）。
//! 速さが要らない場面で `NaiveCopy` と突き合わせると、差分更新のバグが出る。
//!
//! # 世代スキップ
//!
//! 手ごとに消費するターン数が違う問題では、[`Cursor::push_at`] に消費ターン数を渡す。
//! `push(act, node)` は `push_at(1, act, node)` と同じ。
//!
//! ```ignore
//! // 頂点 j へ移動する。移動時間は d ターン。
//! cur.push_at(d, act, child);
//! ```
//!
//! ターンを「経過時間」ではなく「消費した資源」と読み替えることになる。**選択は同じ
//! ターンに到達する候補どうしでのみ行われる**ので、候補の比較は常に同じ資源を使った
//! ものどうしになる。
//!
//! `step > 1` を扱えるのは [`LinkedTree`] だけで、他は panic する。
//!
//! # 選択を自分で書く
//!
//! 既定は「ハッシュで重複除去して `value` 上位 `width`」。多様性制約などが要る問題だけ
//! [`Logic::select`] を上書きし、[`Selector`] の部品を使う。
//!
//! | 部品 | 有利な条件 |
//! |---|---|
//! | [`Selector::dedup_top_k`] | 既定。重複が多い（ユニークなハッシュが候補数より十分少ない） |
//! | [`Selector::sorted_filter`] | 重複が少なく、良い順に舐めながら状態を更新したい |
//! | [`Selector::dedup`] / [`Selector::dedup_sorted`] | 上の 2 つで足りないとき |
//!
//! 重複除去は「キー 1 つ・閾値 0 の多様性制約」でしかない。両方要るなら
//! `sorted_filter` の述語の中に一緒に書くと、走査 1 回で済む。
//!
//! # 途中で打ち切れる問題
//!
//! [`Logic::TERMINAL`] を設定し、[`Logic::is_terminal`] を実装する。
//!
//! | | 意味 |
//! |---|---|
//! | [`Terminal::Ignore`] | 既定。`max_turn` まで回して最後の候補から最良を返す |
//! | [`Terminal::HaltOnFirst`] | 最初に見つかったターンで打ち切る。**ターン数が増えると必ず悪くなる問題だけ** |
//! | [`Terminal::KeepBest`] | 見つけても続け、全ターンを通じて最良を返す |
//!
//! 設定しておくと、候補が尽きたときにも解を取り出せる。設定していない問題で候補が
//! 尽きると、空の手順を黙って返さずに panic する。
//!
//! # 途中経過を見る
//!
//! [`Logic::LOG`] を `true` にすると、1 ターンごとの内訳が stderr に出る。
//!
//! ```text
//! turn=799  width=10000 cand=26643 uniq=11583 除去= 56.5% chosen=10000 \
//!   | 反映 0.05 展開 0.88 判定 0.01 選択 0.18 = 1.12ms 累計 0.775s
//! ```
//!
//! | | |
//! |---|---|
//! | `cand` | 展開で生まれた候補の数 |
//! | `uniq` | ハッシュで重複除去したあとの件数（測れないときは `-`） |
//! | `除去` | `1 - uniq/cand`。高いほどハッシュが効いている |
//! | `chosen` | 実際に採用した数 |
//! | `反映` / `展開` / `判定` / `選択` | フェーズ別の所要時間。`反映`だけ**前のターン**のぶん |
//!
//! **どのフェーズで時間を使っているかが分かるので、遅い原因を一発で絞れる。**
//! `選択` だけが跳ねるなら `Logic::select` の中、`展開` なら木の走査か `Logic::expand`。
//!
//! **`chosen` が `width` に届いていないなら、幅ではなく重複除去や `select` の制約が
//! 効いている。**そこで幅を広げても何も増えない。
//!
//! `false` なら分岐ごと消えるので、切っている間のコストはゼロ。**提出前とチューニング
//! 前は `false` に戻すこと**（stderr からスコアを読む採点器では出力が壊れる）。
//!
//! # 守らないと静かに壊れる約束
//!
//! - `apply(a)` と `undo(a)` は厳密な逆操作であること。
//! - [`Logic::expand`] は `state` を一時的に書き換えてよいが、**復帰させる**こと。
//! - [`Logic::advance`] で `hash` の計算結果を変えないこと。ターンを跨いでハッシュが
//!   変わると重複除去が破綻する。変えてよいのは `value` に関わる係数と作業領域だけ。
//! - [`Logic::select`] が `out` に書くのは候補の添字。**順序は問わない**。
//! - `L::Hash` が 64bit 未満で、自前のハッシュ表を使うときは [`hash_key`] を通すこと。
//!
//! デバッグでは [`NaiveCopy`] との結果一致が最も強力。`apply` の直後に `undo` して
//! 状態が戻るかの検査も、`debug_assert` で入れておくと安い。
//!
//! # コンテスト中の変更
//!
//! 問題ごとの候補生成や評価は `Logic` の実装に置く。探索器の内部を変える場合は、
//! コンテストの Cargo.toml が参照する cplib のチェックアウトを編集する。

pub mod logic;
pub mod search;
pub mod selector;
pub mod tree;
pub mod width;

pub use logic::{Cursor, Logic, Terminal};
pub use search::BeamSearch;
pub use selector::{FlatHashMap, FlatHashSet, Selector, hash_key};
pub use tree::{LinkedTree, NaiveCopy, PostorderTour, Tree};
pub use width::{BayesianWidth, FixedWidth, SqrtFeedback, WidthPolicy};
