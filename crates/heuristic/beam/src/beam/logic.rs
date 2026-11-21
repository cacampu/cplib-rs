//! 問題側が実装するトレイト。

use std::hash::Hash;

use super::selector::Selector;

/// 実行可能解が見つかったときの扱い。
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Terminal {
    /// 終了判定を使わない（既定）。`max_turn` まで回して最後の候補から最良を返す。
    Ignore,
    /// 最初に見つかったターンで打ち切る。ターン数が増えると解が悪くなる問題向け。
    HaltOnFirst,
    /// 見つけても探索を続け、全ターンを通じて `final_value` が最良のものを返す。
    /// いつ止めてもよく、かつ深いほど良いとは限らない問題向け。
    KeepBest,
}

/// 問題固有の処理を束ねるトレイト。
///
/// 実装型は値を持たない空型でよい（例: `enum MyLogic {}`）。
///
/// `State` / `Node` / `Action` の分担はモジュールの doc を参照。
pub trait Logic: Sized {
    /// in-place で apply / undo される唯一の状態。コピーされない。
    type State;
    /// 探索全体で共有される環境。入力、評価係数、Zobrist 表、作業領域。
    type Context;
    /// 探索木の辺。走査のたびにコピーされる。
    type Action: Copy;
    /// 探索木の頂点の荷物。
    type Node: Copy;
    type Value: Ord + Copy;
    /// `Ord` は `(Value, Hash)` を比較キーにするために要る。
    type Hash: Copy + Eq + Ord + Hash;

    /// false なら `Value` は大きいほど良いものとして扱われる。
    const MINIMIZE: bool = true;

    /// 1 ターンごとの内訳を stderr に出す。`false` なら分岐ごと消える。
    ///
    /// 幅・候補数・重複除去率・所要時間が出る。**提出前に `false` に戻すこと**
    /// （stderr からスコアを読む採点器を使っている場合は特に）。
    const LOG: bool = false;

    /// 実行可能解の扱い。既定の `Ignore` なら [`is_terminal`](Self::is_terminal) は
    /// 呼ばれない。
    ///
    /// **`is_terminal` を実装したらここも設定すること。** 忘れると黙って無視される。
    const TERMINAL: Terminal = Terminal::Ignore;

    fn root(ctx: &Self::Context) -> (Self::State, Self::Node);

    /// `apply` と `undo` は厳密に逆操作であること。
    fn apply(state: &mut Self::State, act: Self::Action, ctx: &Self::Context);
    fn undo(state: &mut Self::State, act: Self::Action, ctx: &Self::Context);

    /// `state` は `cur.node()` が指す頂点の状態になっている。
    /// 一時的な書き換えは許されるが、復帰する義務がある。
    fn expand(state: &mut Self::State, ctx: &Self::Context, cur: &mut impl Cursor<Self>);

    fn value(node: &Self::Node, ctx: &Self::Context) -> Self::Value;

    /// 最終解の選択キー。既定は `value`。
    /// 評価に多様性用の乱数などを混ぜている場合に分離する。
    fn final_value(node: &Self::Node, ctx: &Self::Context) -> Self::Value {
        Self::value(node, ctx)
    }

    fn hash(node: &Self::Node, ctx: &Self::Context) -> Self::Hash;

    /// ターン先頭で一度だけ呼ばれる。
    /// `value` に関わる係数の更新と、[`select`](Self::select) が使う作業領域の
    /// リセットを行う。
    ///
    /// `width` はこのターンに [`WidthPolicy`](super::width::WidthPolicy) が決めた
    /// ビーム幅。幅から作る量（多様性制約の閾値など）はここで `ctx` に置いておく。
    /// `value` には幅が渡らないので、必要ならこの経路で運ぶ。
    ///
    /// **`hash` の計算結果を変えてはならない**（ターンを跨いでハッシュが変わると
    /// 重複除去が破綻する）。
    fn advance(_ctx: &mut Self::Context, _turn: usize, _width: usize) {}

    /// この頂点が実行可能解か。[`TERMINAL`](Self::TERMINAL) が `Ignore` なら
    /// 呼ばれない。
    ///
    /// 判定は**選択の前に候補全体**に対して行われるので、幅や重複除去で落ちた解を
    /// 見逃すことがない。
    fn is_terminal(_node: &Self::Node, _ctx: &Self::Context) -> bool {
        false
    }

    /// 候補から採用するものを選ぶ。`out` には `cands` の添字を書く。
    ///
    /// **順序は問わない。** `value` 順に並べるかどうかも実装者が決めてよい。
    ///
    /// 既定は「ハッシュで重複除去して `value` 上位 `width`」
    /// （[`Selector::dedup_top_k`]）。多様性制約などが要る問題だけ上書きし、
    /// [`Selector::dedup_sorted`] などを部品として使う。
    ///
    /// ```ignore
    /// fn select(cands: &[Node], ctx: &mut Context, width: usize,
    ///           sel: &mut Selector<Self>, out: &mut Vec<u32>) {
    ///     for &i in sel.dedup_sorted(cands, ctx) {
    ///         if diversity_ok(&cands[i], ctx) {
    ///             out.push(i);
    ///             if out.len() == width { break; }
    ///         }
    ///     }
    /// }
    /// ```
    fn select(
        cands: &[Self::Node],
        ctx: &mut Self::Context,
        width: usize,
        sel: &mut Selector<Self>,
        out: &mut Vec<u32>,
    ) {
        sel.dedup_top_k(cands, ctx, width, out)
    }
}

/// `expand` から見える面。
///
/// `push` / `push_at` の親は、直前の [`Tree::walk`](super::tree::Tree::walk) が
/// 指した頂点である。呼び出し側が親を指定することはない。
pub trait Cursor<L: Logic> {
    /// 現在展開中の頂点。
    fn node(&self) -> &L::Node;

    /// `step` ターン後に到達する子を追加する。
    ///
    /// `step > 1` を指定できるのは、そのバックエンドが世代スキップに対応している
    /// 場合のみ。非対応のバックエンドは panic する。
    fn push_at(&mut self, step: u32, act: L::Action, node: L::Node);

    fn push(&mut self, act: L::Action, node: L::Node) {
        self.push_at(1, act, node)
    }
}
