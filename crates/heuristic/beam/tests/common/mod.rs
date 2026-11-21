//! テスト用のトイ問題。
//!
//! # 問題
//!
//! 一列に並んだ `n` マスに得点 `a[i]` がある。`start` から出発し、毎ターン
//! 「左・その場・右」のいずれかへ動く（両端でクランプ）。マスに**初めて**入ったとき
//! だけ `a[i]` を得る。`t` ターン後の合計得点を最大化する。
//!
//! # なぜこの問題か
//!
//! - `State` に訪問回数カウンタを持つので `apply` / `undo` が自明でなく、
//!   巻き戻しのバグが得点のズレとして表面化する。
//! - 「初回訪問時のみ加点」は巻き戻しが面倒な量なので `Node` 側に置く。
//!   `State` と `Node` の分担をそのまま踏む。
//! - 未来が `(位置, 訪問済み集合)` だけで決まるので Zobrist ハッシュによる重複除去が
//!   正当。したがって幅を十分大きくすれば**必ず最適解に一致する**。全探索と
//!   突き合わせられる。

use beam::{Cursor, Logic, Selector};

/// `splitmix64`。テストの再現性のために手書きする（依存を増やさない）。
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

pub struct Toy;

pub struct Context {
    pub n: usize,
    pub t: usize,
    pub start: usize,
    pub a: Vec<i64>,
    /// 位置の Zobrist。
    pub zpos: Vec<u64>,
    /// 訪問済みマスの Zobrist。
    pub zvis: Vec<u64>,
}

impl Context {
    pub fn generate(seed: u64, n: usize, t: usize) -> Self {
        let mut rng = Rng::new(seed);
        let a = (0..n).map(|_| rng.below(100) as i64).collect();
        let zpos = (0..n).map(|_| rng.next_u64()).collect();
        let zvis = (0..n).map(|_| rng.next_u64()).collect();
        let start = rng.below(n as u64) as usize;
        Self {
            n,
            t,
            start,
            a,
            zpos,
            zvis,
        }
    }

    /// 手順を根から再生して得点を求める。ライブラリを一切通さない検算用。
    pub fn replay(&self, path: &[u16]) -> i64 {
        let mut pos = self.start;
        let mut visited = vec![false; self.n];
        visited[pos] = true;
        let mut score = self.a[pos];
        for &act in path {
            let (from, to) = unpack(act);
            assert_eq!(from, pos, "経路が繋がっていない");
            assert!(to.abs_diff(from) <= 1, "移動幅が不正");
            if !visited[to] {
                visited[to] = true;
                score += self.a[to];
            }
            pos = to;
        }
        score
    }

    /// 全探索。`3^t` 通りを DFS する。
    pub fn brute_force(&self) -> i64 {
        let mut visited = vec![false; self.n];
        visited[self.start] = true;
        self.dfs(self.start, &mut visited, self.a[self.start], self.t)
    }

    fn dfs(&self, pos: usize, visited: &mut [bool], score: i64, rest: usize) -> i64 {
        if rest == 0 {
            return score;
        }
        let mut best = i64::MIN;
        for to in neighbors(pos, self.n) {
            let fresh = !visited[to];
            visited[to] = true;
            let gain = if fresh { self.a[to] } else { 0 };
            best = best.max(self.dfs(to, visited, score + gain, rest - 1));
            if fresh {
                visited[to] = false;
            }
        }
        best
    }
}

#[derive(Clone)]
pub struct State {
    pub pos: usize,
    /// 訪問回数。0 なら未訪問。`undo` のためにカウンタにしている。
    pub cnt: Vec<u32>,
}

#[derive(Clone, Copy)]
pub struct Node {
    pub score: i64,
    pub hash: u64,
}

/// 辺は `(移動元, 移動先)` を u16 に詰める。`undo` に移動元が要るため。
pub fn pack(from: usize, to: usize) -> u16 {
    ((from as u16) << 8) | to as u16
}

pub fn unpack(act: u16) -> (usize, usize) {
    ((act >> 8) as usize, (act & 0xff) as usize)
}

pub fn neighbors(pos: usize, n: usize) -> impl Iterator<Item = usize> {
    let lo = pos.saturating_sub(1);
    let hi = (pos + 1).min(n - 1);
    (lo..=hi).collect::<Vec<_>>().into_iter()
}

impl Logic for Toy {
    type State = State;
    type Context = Context;
    type Action = u16;
    type Node = Node;
    type Value = i64;
    type Hash = u64;

    const MINIMIZE: bool = false;

    fn root(ctx: &Context) -> (State, Node) {
        let mut cnt = vec![0; ctx.n];
        cnt[ctx.start] = 1;
        let state = State {
            pos: ctx.start,
            cnt,
        };
        let node = Node {
            score: ctx.a[ctx.start],
            hash: ctx.zpos[ctx.start] ^ ctx.zvis[ctx.start],
        };
        (state, node)
    }

    fn apply(state: &mut State, act: u16, _ctx: &Context) {
        let (_, to) = unpack(act);
        state.cnt[to] += 1;
        state.pos = to;
    }

    fn undo(state: &mut State, act: u16, _ctx: &Context) {
        let (from, to) = unpack(act);
        state.cnt[to] -= 1;
        state.pos = from;
    }

    fn expand(state: &mut State, ctx: &Context, cur: &mut impl Cursor<Self>) {
        let node = *cur.node();
        let from = state.pos;
        for to in neighbors(from, ctx.n) {
            let fresh = state.cnt[to] == 0;
            let child = Node {
                score: node.score + if fresh { ctx.a[to] } else { 0 },
                hash: node.hash
                    ^ ctx.zpos[from]
                    ^ ctx.zpos[to]
                    ^ if fresh { ctx.zvis[to] } else { 0 },
            };
            cur.push(pack(from, to), child);
        }
    }

    fn value(node: &Node, _ctx: &Context) -> i64 {
        node.score
    }

    fn hash(node: &Node, _ctx: &Context) -> u64 {
        node.hash
    }

    /// 同値をハッシュで割って**全順序**にする。
    ///
    /// 既定の `dedup_top_k` は `select_nth_unstable` なので、同値が境界にいると
    /// どれが残るかが候補の並び順で決まる。バックエンドは候補を逆順に構築するので、
    /// そのままではバックエンド間の結果一致テストが成立しない。
    ///
    /// `Logic::select` を上書きする実例も兼ねている。
    fn select(
        cands: &[Node],
        ctx: &mut Context,
        width: usize,
        sel: &mut Selector<Self>,
        out: &mut Vec<u32>,
    ) {
        let mut order: Vec<u32> = sel.dedup(cands, ctx).to_vec();
        order.sort_unstable_by(|&a, &b| {
            let (x, y) = (&cands[a as usize], &cands[b as usize]);
            // 最大化なので score は降順、同値は hash 昇順。
            y.score.cmp(&x.score).then(x.hash.cmp(&y.hash))
        });
        order.truncate(width);
        out.extend(order);
    }
}
