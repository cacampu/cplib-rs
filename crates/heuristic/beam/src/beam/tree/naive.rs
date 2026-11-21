//! `State` をコピーする素朴な実装。
//!
//! `State` が小さい問題では差分更新より速いことがある。それ以上に、
//! [`PostorderTour`](super::PostorderTour) の**正当性の照合先**として重要。同じ
//! `Logic` を回して解が一致するかというテストが、差分更新のバグに最も強い。
//!
//! # なぜ「コピー」なのに `apply` を呼ぶのか
//!
//! [`Cursor::push_at`] にも [`Tree::commit`] にも `ctx` が渡らないため、候補を
//! 生成した時点で子の状態を作ることができない。そこで各葉について
//! `(親の葉番号, 辺)` だけを記録しておき、`ctx` が手に入る [`Tree::walk`] の中で
//! 「前世代の葉の状態を clone して `apply` を1回」という形で実体化する。
//!
//! 結果として葉ごとに毎ターン clone が2回（実体化用と呼び出し側への引き渡し用）
//! 走る。速度比較に使うときはこの定数倍を差し引いて読むこと。

use super::super::logic::{Cursor, Logic};
use super::Tree;

const NONE: usize = usize::MAX;

pub struct NaiveCopy<L: Logic> {
    /// 現世代の葉。`walk` が順に提示する。
    nodes: Vec<L::Node>,
    /// `nodes[i]` の (親の葉番号, 辺)。turn 0 では空。
    src: Vec<(u32, L::Action)>,
    /// 根から `nodes[i]` への経路。
    paths: Vec<Vec<L::Action>>,

    /// 前世代の葉の状態。`walk` が現世代を作るのに使う。
    prev_states: Vec<L::State>,
    /// 現世代の葉の状態。`walk` が訪問順に push する。
    states: Vec<L::State>,

    /// このターンに生成された候補。
    next_nodes: Vec<L::Node>,
    next_src: Vec<(u32, L::Action)>,

    /// `walk` が現在提示している葉。`NONE` は未開始。
    cur: usize,
    turn: usize,
}

impl<L: Logic> Cursor<L> for NaiveCopy<L>
where
    L::State: Clone,
{
    fn node(&self) -> &L::Node {
        &self.nodes[self.cur]
    }

    fn push_at(&mut self, step: u32, act: L::Action, node: L::Node) {
        assert_eq!(step, 1, "NaiveCopy は世代スキップに対応していない");
        self.next_nodes.push(node);
        self.next_src.push((self.cur as u32, act));
    }
}

impl<L: Logic> Tree<L> for NaiveCopy<L>
where
    L::State: Clone,
{
    /// 経路そのものを持つ。`commit` を跨いでも無効化されない。
    type Guard = Vec<L::Action>;

    fn new(root: L::Node, max_width: usize) -> Self {
        Self {
            nodes: vec![root],
            src: Vec::new(),
            paths: vec![Vec::new()],
            prev_states: Vec::new(),
            states: Vec::with_capacity(max_width),
            next_nodes: Vec::new(),
            next_src: Vec::new(),
            cur: NONE,
            turn: 0,
        }
    }

    fn begin(&mut self) {
        self.cur = NONE;
        // 今ターンの walk は「前世代の状態」から現世代を作る。
        std::mem::swap(&mut self.prev_states, &mut self.states);
        self.states.clear();
        self.next_nodes.clear();
        self.next_src.clear();
    }

    fn walk(&mut self, state: &mut L::State, ctx: &L::Context) -> bool {
        self.cur = if self.cur == NONE { 0 } else { self.cur + 1 };
        if self.cur >= self.nodes.len() {
            return false;
        }

        if self.turn == 0 {
            // 根。呼び出し側の state がそのまま根の状態になっている。
            self.states.push(state.clone());
        } else {
            let (parent, act) = self.src[self.cur];
            let mut st = self.prev_states[parent as usize].clone();
            L::apply(&mut st, act, ctx);
            *state = st.clone();
            self.states.push(st);
        }
        true
    }

    fn candidates(&self) -> &[L::Node] {
        &self.next_nodes
    }

    /// `chosen` の順序はそのまま新しい世代の並び順になる。`walk` はこの順に提示し、
    /// 親番号もこの並びで振られるので、どんな順序で渡されても整合する。
    fn commit(&mut self, chosen: &[u32]) {
        let mut nodes = Vec::with_capacity(chosen.len());
        let mut src = Vec::with_capacity(chosen.len());
        let mut paths = Vec::with_capacity(chosen.len());
        for &i in chosen {
            let (parent, act) = self.next_src[i as usize];
            nodes.push(self.next_nodes[i as usize]);
            src.push((parent, act));
            let mut path = self.paths[parent as usize].clone();
            path.push(act);
            paths.push(path);
        }
        self.nodes = nodes;
        self.src = src;
        self.paths = paths;
        self.turn += 1;
    }

    fn protect(&mut self, i: u32) -> Self::Guard {
        let (parent, act) = self.next_src[i as usize];
        let mut path = self.paths[parent as usize].clone();
        path.push(act);
        path
    }

    fn release(&mut self, _g: Self::Guard) {}

    fn path(&self, g: &Self::Guard) -> Vec<L::Action> {
        g.clone()
    }
}
