//! 二重連鎖木版。世代スキップ対応。
//!
//! 定数倍は [`PostorderTour`](super::PostorderTour) に劣るが、`push_at(step > 1)`
//! を扱える。木を配列に畳まず、親子・兄弟のポインタで持つので、同じターンの頂点が
//! 異なる深さに散らばっていても構わない。
//!
//! # 候補は到達ターンまで木に入れない
//!
//! これが世代スキップの要点である。`push_at(step, ..)` された候補は
//! `pending[step - 1]` に積まれるだけで、木には触らない。到達ターンが来て
//! [`Tree::candidates`] として選択に晒され、[`Tree::commit`] で採用されたものだけが
//! 葉として木に加わる。
//!
//! ```text
//! turn t   : expand → push_at(1) → pending[0]   push_at(3) → pending[2]
//!            commit → pending[0] を pop して木に追加、残りは繰り上がる
//! turn t+1 : pending[0] は「t で step 2 を指定した分」＋「t+1 で step 1 を指定した分」
//! ```
//!
//! したがって**選択は同じターンに到達する候補どうしでのみ行われる**。
//!
//! # `expire` — いつまで子が増えうるか
//!
//! 頂点 `v` を展開したターンを `t`、そのとき指定した最大の `step` を `m` とすると、
//! `v` の子は turn `t+1` から `t+m` にかけて**ばらばらに**加わる。`v.expire = t + m`
//! を記録しておき、次の2箇所で参照する。
//!
//! - `update_root` — `expire > turn` の頂点は飛ばせない。飛ばすと、後から
//!   加わった子が探索根より上にぶら下がり、走査が届かなくなる。
//! - `remove_leaf` — `expire > turn` の頂点は、今は子が無くても消せない。
//!   消してから子が来ると親が宙に浮く。
//!
//! step が常に 1 なら `expire` は常に `turn` に一致するので、どちらの判定も素通りする。
//!
//! `Guard` は `protect` の時点で経路を `Vec<Action>` に落とす
//! （[`PostorderTour`](super::PostorderTour) と同じ）。

use std::collections::VecDeque;
use std::ops::{Index, IndexMut};

use super::super::logic::{Cursor, Logic};
use super::Tree;

type Idx = u32;
const NONE: Idx = u32::MAX;

struct TreeNode<L: Logic> {
    act: L::Action,
    node: L::Node,
    parent: Idx,
    /// 子の連結リストの先頭。最後に追加した子が来る。
    child: Idx,
    /// 兄弟リストの前後。
    prev: Idx,
    next: Idx,
    /// この部分木に、今ターン展開すべき葉が残っているか。
    ///
    /// 走査しながら降ろしていく消費物である。`commit` で葉を足すときに、
    /// その葉から根まで立て直す。
    active: bool,
    /// 解放済みか。`collect_garbage` が同じ回の削除で巻き込まれた頂点を弾くのに使う。
    dead: bool,
    /// 子が追加されうる最も遅いターン。
    expire: usize,
}

/// 削除済み添字を再利用する配列。`TreeNode` のフィールドはすべて `Copy` なので、
/// 解放は添字を積むだけでよく、`Drop` も `MaybeUninit` も要らない。
struct Pool<L: Logic> {
    buf: Vec<TreeNode<L>>,
    vacant: Vec<Idx>,
}

impl<L: Logic> Pool<L> {
    fn with_capacity(n: usize) -> Self {
        Self {
            buf: Vec::with_capacity(n),
            vacant: Vec::with_capacity(n),
        }
    }

    fn alloc(&mut self, node: TreeNode<L>) -> Idx {
        match self.vacant.pop() {
            Some(i) => {
                self.buf[i as usize] = node;
                i
            }
            None => {
                self.buf.push(node);
                (self.buf.len() - 1) as Idx
            }
        }
    }

    fn release(&mut self, i: Idx) {
        self.buf[i as usize].dead = true;
        self.vacant.push(i);
    }
}

impl<L: Logic> Index<Idx> for Pool<L> {
    type Output = TreeNode<L>;

    #[inline]
    fn index(&self, i: Idx) -> &TreeNode<L> {
        &self.buf[i as usize]
    }
}

impl<L: Logic> IndexMut<Idx> for Pool<L> {
    #[inline]
    fn index_mut(&mut self, i: Idx) -> &mut TreeNode<L> {
        &mut self.buf[i as usize]
    }
}

/// 同じターンに到達する候補の束。`Node` と辺を分けて持つのは、`candidates()` が
/// `&[L::Node]` を返す契約だから。
struct Pending<L: Logic> {
    nodes: Vec<L::Node>,
    /// `nodes[i]` の (親, 辺)。
    edges: Vec<(Idx, L::Action)>,
}

impl<L: Logic> Pending<L> {
    fn with_capacity(n: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(n),
            edges: Vec::with_capacity(n),
        }
    }

    fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
    }
}

pub struct LinkedTree<L: Logic> {
    pool: Pool<L>,
    /// 探索の起点。根から一本道の部分は降ろしてあり、そこまでは `State` に適用済み。
    root: Idx,
    /// 今展開している葉。
    cur: Idx,
    /// 今展開している葉が指定した最大の `step`。`expire` の計算に使う。
    cur_step: u32,
    /// このターンの走査を開始したか。`begin` で false になる。
    scanning: bool,
    /// `pending[k]` は turn+k+1 に到達する候補。常に 1 要素以上ある。
    pending: VecDeque<Pending<L>>,
    /// `remove_queue[k]` は turn+k+1 に不要になりうる頂点。
    remove_queue: VecDeque<Vec<Idx>>,
    /// `commit` が採用集合を落とすビットマップ。
    chosen_mask: Vec<u64>,
    turn: usize,
    /// 候補バッファの確保サイズ。
    capacity: usize,
}

impl<L: Logic> LinkedTree<L> {
    /// 根から一本道の部分を降ろす。往復の手間を省くためで、これが無いと毎ターン
    /// 共通接頭辞を辿り直して O(ターン数^2) になる。
    ///
    /// **ただし実測では効果が出なかった。** 2000 ターン・幅 1000〜10000 の問題で、
    /// これを無効化した版との差は 1% 未満だった。一本道の往復は分岐予測の当たる
    /// ポインタ追跡で `apply` / `undo` も軽いためで、効くのはターン数が幅を大きく
    /// 上回る場合に限られる。
    fn update_root(&mut self, state: &mut L::State, ctx: &L::Context) {
        loop {
            let r = self.root;
            // 後で子が追加されうる頂点は飛ばせない。
            if self.pool[r].expire > self.turn {
                break;
            }
            let c = self.pool[r].child;
            // 子が居ない、または分岐しているなら止まる。
            if c == NONE || self.pool[c].next != NONE {
                break;
            }
            L::apply(state, self.pool[c].act, ctx);
            self.root = c;
        }
    }

    /// `v` の子孫で最初に見つかる active な葉まで降り、途中で `apply` する。
    ///
    /// 通り道の `active` は倒していく。「active な頂点が葉でないなら active な子を
    /// 持つ」という不変条件があるので、内側のループは必ず止まる。
    fn move_to_leaf(&mut self, mut v: Idx, state: &mut L::State, ctx: &L::Context) -> Idx {
        loop {
            let mut c = self.pool[v].child;
            if c == NONE {
                break;
            }
            while !self.pool[c].active {
                c = self.pool[c].next;
            }
            self.pool[v].active = false;
            L::apply(state, self.pool[c].act, ctx);
            v = c;
        }
        self.pool[v].active = false;
        v
    }

    /// `undo` しながら遡り、右に active な兄弟が見つかったらそこへ移る。
    /// 見つからないまま探索根に戻ったら根を返す（走査終了）。
    fn move_to_ancestor(&mut self, mut v: Idx, state: &mut L::State, ctx: &L::Context) -> Idx {
        while v != self.root {
            L::undo(state, self.pool[v].act, ctx);

            let mut u = self.pool[v].next;
            while u != NONE {
                if self.pool[u].active {
                    L::apply(state, self.pool[u].act, ctx);
                    return u;
                }
                u = self.pool[u].next;
            }
            v = self.pool[v].parent;
        }
        self.root
    }

    /// 展開を終えた葉を、いつ消してよいかの待ち行列に登録する。
    fn retire(&mut self, v: Idx) {
        let k = self.cur_step as usize;
        self.pool[v].expire = self.turn + k;
        while self.remove_queue.len() < k {
            self.remove_queue.push_back(Vec::new());
        }
        self.remove_queue[k - 1].push(v);
    }

    /// 前ターンまでに不要になった頂点を回収する。
    fn collect_garbage(&mut self) {
        let Some(mut queue) = self.remove_queue.pop_front() else {
            return;
        };
        for v in queue.drain(..) {
            // 同じ回の削除で既に巻き込まれていることがある。祖先と子孫が同じ待ち行列に
            // 並ぶのは step がばらつく場合だけなので、1ステップ限定なら起きない。
            if self.pool[v].dead {
                continue;
            }
            self.remove_leaf(v);
        }
        self.remove_queue.push_back(queue);
    }

    /// 子を持たない頂点を、条件を満たす限り根の方向へ再帰的に削除する。
    fn remove_leaf(&mut self, mut v: Idx) {
        loop {
            // 探索根は消さない。候補が尽きたことは `candidates()` が空になることで伝わる。
            if v == self.root || self.pool[v].child != NONE || self.pool[v].expire > self.turn {
                return;
            }
            let parent = self.pool[v].parent;
            let (prev, next) = (self.pool[v].prev, self.pool[v].next);
            if prev == NONE {
                self.pool[parent].child = next;
            } else {
                self.pool[prev].next = next;
            }
            if next != NONE {
                self.pool[next].prev = prev;
            }
            self.pool.release(v);
            v = parent;
        }
    }

    /// 採用された候補を葉として木に加え、根までの `active` を立て直す。
    fn add_leaf(&mut self, parent: Idx, act: L::Action, node: L::Node) {
        let sibling = self.pool[parent].child;
        let v = self.pool.alloc(TreeNode {
            act,
            node,
            parent,
            child: NONE,
            prev: NONE,
            next: sibling,
            active: true,
            dead: false,
            expire: 0,
        });
        self.pool[parent].child = v;
        if sibling != NONE {
            self.pool[sibling].prev = v;
        }

        // 既に active な頂点に当たったらそこから上は立っている。
        let mut u = parent;
        while !self.pool[u].active {
            self.pool[u].active = true;
            if u == self.root {
                break;
            }
            let p = self.pool[u].parent;
            // update_root が expire を見て止まるので、親は必ず探索根以下にいる。
            debug_assert_ne!(p, NONE, "候補の親が探索根より上にある");
            u = p;
        }
    }

    fn restore_path(&self, mut v: Idx) -> Vec<L::Action> {
        let mut path = Vec::new();
        while self.pool[v].parent != NONE {
            path.push(self.pool[v].act);
            v = self.pool[v].parent;
        }
        path.reverse();
        path
    }
}

impl<L: Logic> Cursor<L> for LinkedTree<L> {
    fn node(&self) -> &L::Node {
        &self.pool[self.cur].node
    }

    fn push_at(&mut self, step: u32, act: L::Action, node: L::Node) {
        assert!(step >= 1, "step は 1 以上でなければならない");
        let k = step as usize;
        while self.pending.len() < k {
            let cap = self.capacity;
            self.pending.push_back(Pending::with_capacity(cap));
        }
        self.pending[k - 1].nodes.push(node);
        self.pending[k - 1].edges.push((self.cur, act));
        self.cur_step = self.cur_step.max(step);
    }
}

impl<L: Logic> Tree<L> for LinkedTree<L>
where
    L::Action: Default,
{
    /// `protect` の時点で経路を確定させる。
    type Guard = Vec<L::Action>;

    fn new(root: L::Node, max_width: usize) -> Self {
        let capacity = max_width * 4;
        let mut pool = Pool::with_capacity(capacity);
        let r = pool.alloc(TreeNode {
            act: L::Action::default(),
            node: root,
            parent: NONE,
            child: NONE,
            prev: NONE,
            next: NONE,
            active: true,
            dead: false,
            expire: 0,
        });
        let mut pending = VecDeque::new();
        pending.push_back(Pending::with_capacity(capacity));

        Self {
            pool,
            root: r,
            cur: r,
            cur_step: 1,
            scanning: false,
            pending,
            remove_queue: VecDeque::new(),
            chosen_mask: Vec::with_capacity(capacity.div_ceil(64)),
            turn: 0,
            capacity,
        }
    }

    fn begin(&mut self) {
        self.collect_garbage();
        self.scanning = false;
    }

    /// 探索根の更新は `state` が要るのでここで行う（`begin` には渡らない）。
    fn walk(&mut self, state: &mut L::State, ctx: &L::Context) -> bool {
        if !self.scanning {
            self.scanning = true;
            self.update_root(state, ctx);
            if !self.pool[self.root].active {
                return false;
            }
            self.cur = self.move_to_leaf(self.root, state, ctx);
            self.cur_step = 1;
            return true;
        }

        // 直前の葉は展開し終わっている。ここで初めて cur_step が確定する。
        self.retire(self.cur);

        let v = self.move_to_ancestor(self.cur, state, ctx);
        if v == self.root {
            return false;
        }
        self.cur = self.move_to_leaf(v, state, ctx);
        self.cur_step = 1;
        true
    }

    fn candidates(&self) -> &[L::Node] {
        &self.pending[0].nodes
    }

    /// 採用集合を**添字の昇順**に辿って木へ加える。
    ///
    /// 候補は走査順に積まれているので、添字が近いものは親も近い。`Logic::select` が
    /// 返す評価値順のまま加えると `add_leaf` の親アクセスが毎回飛ぶ。昇順に均すと
    /// 同じ親が連続して当たる。昇順の配列を実体化せず、ビットを立てて舐めるだけで
    /// 済ませるのは `PostorderTour` と同じ手口。
    fn commit(&mut self, chosen: &[u32]) {
        let mut arrived = self.pending.pop_front().expect("pending は常に 1 要素以上");

        self.chosen_mask.clear();
        self.chosen_mask.resize(arrived.nodes.len().div_ceil(64), 0);
        for &i in chosen {
            self.chosen_mask[i as usize / 64] |= 1 << (i % 64);
        }
        for w in 0..self.chosen_mask.len() {
            let mut bits = self.chosen_mask[w];
            while bits != 0 {
                let i = w * 64 + bits.trailing_zeros() as usize;
                bits &= bits - 1;
                let (parent, act) = arrived.edges[i];
                self.add_leaf(parent, act, arrived.nodes[i]);
            }
        }

        arrived.clear();
        self.pending.push_back(arrived);
        self.turn += 1;
    }

    fn has_pending(&self) -> bool {
        self.pending.iter().skip(1).any(|p| !p.nodes.is_empty())
    }

    fn protect(&mut self, i: u32) -> Self::Guard {
        let (parent, act) = self.pending[0].edges[i as usize];
        let mut path = self.restore_path(parent);
        path.push(act);
        path
    }

    fn release(&mut self, _g: Self::Guard) {}

    fn path(&self, g: &Self::Guard) -> Vec<L::Action> {
        g.clone()
    }
}
