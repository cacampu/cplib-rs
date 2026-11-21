//! 帰りがけ順配列版。主力バックエンド。
//!
//! 1ステップ限定。`push_at` で `step != 1` なら panic する。
//!
//! # 1ターンに3世代が絡む
//!
//! 配列がどの世代に属するかを取り違えると読めなくなるので先に固定する。
//! `turn` フィールドが `T` のとき、
//!
//! | 配列 | 深さ | 役割 |
//! |---|---|---|
//! | `parent_ofs` / `tour` | `T-1` | 経路復元に使う。**親**の世代 |
//! | `cands` / `cand_edges` / `chosen_mask` | `T` | 今ターン展開する候補 |
//! | `next_cands` / `next_cand_edges` | `T+1` | 展開で生まれた候補（`candidates()`） |
//! | `next_tour` / `next_parent_ofs` | `T` | 深さ `T` の頂点を、次ターンの親として記述する |
//!
//! `cand_edges[c] = (辺, 親番号)` で、親番号は `parent_ofs` の添字である。候補番号を
//! そのまま `parent_ofs` の添字に使わないこと。
//!
//! # 中核となる不変条件
//!
//! > `parent_ofs[j+1] - parent_ofs[j]` は、親 j と親 j+1 の LCA から親 j までの距離に
//! > 等しい。`tour[parent_ofs[j] .. parent_ofs[j+1]]` は、その LCA から親 j へ至る
//! > 辺列を、**LCA 側が小さい添字**になるよう並べたもの。
//!
//! ここでいう「親」は、`tour` を作った時点では木の葉だったもの。`tour` は木全体の辺を
//! 持つのではなく、「隣接する2つが共有していない部分」だけを持つ。区間内で深い方が
//! 添字が大きいのは、`trace`（根が添字0）と向きを揃えて `copy_from_slice` で一括
//! コピーするため。
//!
//! # 経路の復元規則
//!
//! 親 j への経路を作る手順（`len` は経路長）:
//!
//! ```ignore
//! let mut prog = 0;
//! for w in parent_ofs[j..].windows(2) {
//!     let rank = w[1] - w[0];
//!     if prog < rank {
//!         dst[len - rank..len - prog].copy_from_slice(&tour[w[0]..w[1] - prog]);
//!         prog = rank;
//!     }
//! }
//! ```
//!
//! 親 j の経路のうち末尾 `rank[j]` 本は区間 j がそのまま持つ。その手前は親 j+1 とも
//! 共有されているので区間 j+1 が持つ。ただし区間 j+1 の深い側 `prog` 本はすでに
//! 埋めたので `w[1] - prog` と終端を削る。区間長が記録を更新したときだけ書き込むのは、
//! 区間長が縮む場合その区間は既に埋めた領域の内側にあるため。
//!
//! この規則は `dst` の初期値が「最後の1つへの経路」であることに依存している。
//! `walk` では `trace`（＝直前に訪問した頂点の経路）が、`protect` では最後に walk した
//! 頂点の経路がその役目を果たす。
//!
//! # `commit` を次の `walk` に融合している
//!
//! [`Tree::commit`] は採用集合をビットマップに落として配列を swap するだけで、
//! **木の構造は何も作らない**。「選ばれた候補を世代として確定させる」作業は
//! 次のターンの `walk` に先送りされ、そこで走査のついでに行われる。
//!
//! 成立する理由は、`walk` が `State` を葉から葉へ動かすために巻き戻す経路が、
//! **そのまま次世代で必要な `tour` の断片と一致する**から。次のターンにおいて
//! 両者の子は隣接し、子同士の LCA は親同士の LCA と一致するため、区間長も中身も
//! 同じものが使える。
//!
//! **移動のために計算した経路が、そのまま構造の記録になる。** 副作用として「非採用の
//! 枝を削除する」処理も存在しない（世代 T の構造はターン T+1 まで作られないので、
//! 消すべきものが無い）。
//!
//! # 採用集合はビットマップで持つ
//!
//! `walk` は候補番号の降順に訪問する（`state_parent` を単調非増加にするため）。
//! この順序は本バックエンドの都合なので `Logic::select` には要求せず、`commit` が
//! 採用集合をビットマップに落として引き受ける。降順の配列を実体化しない。
//!
//! # ダミーの扱い
//!
//! `trace[0]` は「根に入る辺」に相当する、決して適用されない番兵。また各ターンの
//! 先頭では `trace[turn]` が未適用のダミーなので、最初の 1 回だけ undo の範囲から
//! 外す（`first`）。
//!
//! ターンの最初に処理する頂点の extend は `trace[turn]`（ダミー）を含み、
//! `tour[0..parent_ofs[0]]` に収まる。復元規則の窓は `parent_ofs[0]` 以降しか参照
//! しないので読まれない。左隣が存在しない場合分けを、無駄な領域1つで吸収している。

use super::super::logic::{Cursor, Logic};
use super::Tree;

pub struct PostorderTour<L: Logic> {
    /// 根から現在位置までの経路。長さは常に `turn + 1`。`trace[0]` は番兵。
    trace: Vec<L::Action>,
    /// 親ごとの上向き経路の断片。
    tour: Vec<L::Action>,
    /// `tour` の区間境界。親番号で引く。
    parent_ofs: Vec<u32>,

    /// 今ターン展開する候補。
    cands: Vec<L::Node>,
    /// `cands[c]` へ至る辺と、その親番号。
    ///
    /// `walk` はこの2つを必ず同じ添字で同時に引くので、別配列にすると
    /// キャッシュラインを2本触ることになる。1本にまとめてある。
    cand_edges: Vec<(L::Action, u32)>,
    /// 採用された候補のビットマップ。`walk` が上の語から消費する。
    chosen_mask: Vec<u64>,
    /// `chosen_mask` の未処理の語数。0 なら走査終了。
    unscanned: usize,
    /// 今展開中の候補番号。
    cur_cand: usize,

    next_tour: Vec<L::Action>,
    next_parent_ofs: Vec<u32>,
    next_cands: Vec<L::Node>,
    next_cand_edges: Vec<(L::Action, u32)>,

    /// `State` が今いる親の番号。降順走査により単調非増加。
    state_parent: usize,
    /// このターンで最初の `walk` かどうか。`trace[turn]` の undo を抑制する。
    first_walk: bool,
    turn: usize,
}

impl<L: Logic> PostorderTour<L>
where
    L::Action: Default,
{
    /// 今の辺（`trace[turn]`）より**上**で、やり直しが必要な辺の本数。
    ///
    /// 値は「親 `p` と親 `state_parent` の LCA までの距離 - 1」。`trace[turn]` は
    /// 必ず書き換わるので、それを除いた本数を返している。
    ///
    /// 親 `p` から親 `state_parent` まで順に移動すると、途中で最も浅く上がった点が
    /// LCA になる。各ステップで上がる高さが区間長なので、その最大が求める距離。
    fn edges_above(&self, p: usize) -> usize {
        let mut a = 0u32;
        let mut b = self.parent_ofs[self.state_parent];
        for &x in self.parent_ofs[p..self.state_parent].iter().rev() {
            a = a.max(b - x);
            b = x;
        }
        a as usize
    }

    /// 採用集合から次の候補番号を降順に取り出す。
    fn pop_chosen(&mut self) -> Option<usize> {
        while self.unscanned > 0 {
            let w = self.unscanned - 1;
            let bits = self.chosen_mask[w];
            if bits == 0 {
                self.unscanned -= 1;
                continue;
            }
            let b = 63 - bits.leading_zeros() as usize;
            self.chosen_mask[w] = bits & !(1 << b);
            return Some(w * 64 + b);
        }
        None
    }
}

impl<L: Logic> Cursor<L> for PostorderTour<L>
where
    L::Action: Default,
{
    fn node(&self) -> &L::Node {
        &self.cands[self.cur_cand]
    }

    fn push_at(&mut self, step: u32, act: L::Action, node: L::Node) {
        // 世代スキップ非対応。trace の長さが turn に一致することに依存しているため、
        // スキップを許すと同じターンの頂点が異なる深さに散在し、「訪問順に並べて
        // 隣接差分を取る」戦略が成立しない。
        assert_eq!(step, 1, "PostorderTour は世代スキップに対応していない");
        self.next_cands.push(node);
        // 直前に walk した頂点の番号。next_parent_ofs は1つにつき1回 push されている。
        self.next_cand_edges
            .push((act, self.next_parent_ofs.len() as u32 - 1));
    }
}

impl<L: Logic> Tree<L> for PostorderTour<L>
where
    L::Action: Default,
{
    /// `tour` が毎ターン作り替えられるため頂点そのものの保護はできない。
    /// `protect` の時点で経路を確定させる。
    type Guard = Vec<L::Action>;

    fn new(root: L::Node, max_width: usize) -> Self {
        Self {
            trace: vec![L::Action::default()],
            tour: Vec::new(),
            parent_ofs: Vec::new(),
            cands: vec![root],
            cand_edges: vec![(L::Action::default(), 0)],
            chosen_mask: vec![1],
            unscanned: 0,
            cur_cand: 0,
            next_tour: Vec::with_capacity(max_width * 4),
            next_parent_ofs: Vec::with_capacity(max_width),
            next_cands: Vec::with_capacity(max_width * 4),
            next_cand_edges: Vec::with_capacity(max_width * 4),
            state_parent: 0,
            first_walk: true,
            turn: 0,
        }
    }

    fn begin(&mut self) {
        self.unscanned = self.chosen_mask.len();
        self.first_walk = true;
        // 前ターンの walk が最後に訪れた頂点。そこに State がいる。
        self.state_parent = self.parent_ofs.len().saturating_sub(1);
        self.next_tour.clear();
        self.next_parent_ofs.clear();
        self.next_cands.clear();
        self.next_cand_edges.clear();
    }

    fn walk(&mut self, state: &mut L::State, ctx: &L::Context) -> bool {
        let Some(c) = self.pop_chosen() else {
            return false;
        };
        self.cur_cand = c;

        if self.turn == 0 {
            // 根が唯一の頂点。undo も apply も要らず、tour に積むものもない。
            self.next_parent_ofs.push(self.next_tour.len() as u32);
            self.state_parent = 0;
            self.first_walk = false;
            return true;
        }

        let turn = self.turn;
        let (act, p) = self.cand_edges[c];
        let p = p as usize;
        let above = self.edges_above(p);

        // (1) LCA まで戻る
        let f = usize::from(!self.first_walk);
        for act in self.trace[turn - above..turn + f].iter().rev() {
            L::undo(state, *act, ctx);
        }
        self.first_walk = false;

        // (2) 巻き戻した経路をそのまま next_tour に積む
        self.next_tour
            .extend_from_slice(&self.trace[turn - above..]);
        self.next_parent_ofs.push(self.next_tour.len() as u32);

        // (3) trace を「親 p の経路 + 辺」に書き換え、apply しながら進む
        self.trace[turn] = act;
        let mut prog = 0usize;
        for w in self.parent_ofs[p..].windows(2) {
            if prog >= above {
                // trace[turn - above ..] は埋まった。これより浅い側は共有部分。
                break;
            }
            let rank = (w[1] - w[0]) as usize;
            if prog < rank {
                let src = w[0] as usize..w[1] as usize - prog;
                self.trace[turn - rank..turn - prog].copy_from_slice(&self.tour[src]);
                prog = rank;
            }
        }
        for i in turn - above..=turn {
            L::apply(state, self.trace[i], ctx);
        }

        self.state_parent = p;
        true
    }

    fn candidates(&self) -> &[L::Node] {
        &self.next_cands
    }

    fn commit(&mut self, chosen: &[u32]) {
        // 走査順（候補番号の降順）は本バックエンドの都合なので、ここで引き受ける。
        // 降順の配列を作らず、ビットを立てるだけで済ませる。
        self.chosen_mask.clear();
        self.chosen_mask
            .resize(self.next_cands.len().div_ceil(64), 0);
        for &i in chosen {
            self.chosen_mask[i as usize / 64] |= 1 << (i % 64);
        }

        // 非採用の頂点に削除処理は不要。採用されなかった候補は walk で提示されず、
        // next_tour にも載らないので自動的に消える。
        std::mem::swap(&mut self.tour, &mut self.next_tour);
        std::mem::swap(&mut self.parent_ofs, &mut self.next_parent_ofs);
        std::mem::swap(&mut self.cands, &mut self.next_cands);
        std::mem::swap(&mut self.cand_edges, &mut self.next_cand_edges);

        self.turn += 1;
        self.trace.push(L::Action::default());
    }

    fn protect(&mut self, i: u32) -> Self::Guard {
        let i = i as usize;
        let turn = self.turn;
        let (act, parent) = self.next_cand_edges[i];
        // trace は最後に walk した頂点（深さ turn）の経路。そこから復元規則で遡る。
        let mut ret = self.trace[1..=turn].to_vec();
        let mut prog = 0usize;
        for w in self.next_parent_ofs[parent as usize..].windows(2) {
            let rank = (w[1] - w[0]) as usize;
            if prog < rank {
                let src = w[0] as usize..w[1] as usize - prog;
                ret[turn - rank..turn - prog].copy_from_slice(&self.next_tour[src]);
                prog = rank;
            }
        }
        ret.push(act);
        ret
    }

    fn release(&mut self, _g: Self::Guard) {}

    fn path(&self, g: &Self::Guard) -> Vec<L::Action> {
        g.clone()
    }
}
