//! 世代スキップ（`push_at(step > 1)`）の正当性。
//!
//! # 問題
//!
//! 一列に並んだ `n` マスに得点 `a[i]` がある。`start` から出発し、**距離 `d`
//! (`min_jump..=max_jump`) だけ左右いずれかへ跳ぶ**。跳ぶと予算を `d` 消費し、
//! **着地したマスの得点だけ**を得る（初回のみ）。予算 `budget` の範囲で合計得点を
//! 最大化する。
//!
//! ターン＝消費した予算である。よって `d` マス跳ぶ手は `push_at(d, ..)` になり、
//! 同じターンに到達する候補は複数のターンから流れ込む。
//!
//! # なぜこの形か
//!
//! 「途中のマスを飛ばす」ので、**多段移動が単段移動の繰り返しと別物**になる。
//! ここが崩れると世代スキップを使う意味が消え、1ステップ限定の実装で書き直せてしまう。
//!
//! いつ止めてもよい問題なので `Terminal::KeepBest` を使う。予算を使い切った葉は
//! 子を生成せず消えるため、`Guard` で最良解を保持する経路も同時に検査できる。
//!
//! 未来が `(位置, 訪問済み集合)` と残予算だけで決まり、残予算はターンそのものなので、
//! 同一ターン内のハッシュによる重複除去は正当。幅を十分取れば全探索に一致する。

use beam::{BeamSearch, Cursor, FixedWidth, LinkedTree, Logic, Selector, Terminal};

/// `splitmix64`。
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

struct Skip;

struct Context {
    n: usize,
    budget: usize,
    /// 跳べる距離は `min_jump..=max_jump`。`min_jump > 1` にすると、到達する候補が
    /// 1つも無いターンが生まれる（`Tree::has_pending` の経路）。
    min_jump: usize,
    max_jump: usize,
    start: usize,
    a: Vec<i64>,
    zpos: Vec<u64>,
    zvis: Vec<u64>,
}

impl Context {
    fn generate(seed: u64, n: usize, budget: usize, max_jump: usize) -> Self {
        Self::with_min_jump(seed, n, budget, 1, max_jump)
    }

    fn with_min_jump(seed: u64, n: usize, budget: usize, min_jump: usize, max_jump: usize) -> Self {
        let mut rng = Rng::new(seed);
        let a = (0..n).map(|_| rng.below(100) as i64).collect();
        let zpos = (0..n).map(|_| rng.next_u64()).collect();
        let zvis = (0..n).map(|_| rng.next_u64()).collect();
        let start = rng.below(n as u64) as usize;
        Self {
            n,
            budget,
            min_jump,
            max_jump,
            start,
            a,
            zpos,
            zvis,
        }
    }

    /// 行き先と消費予算の組。
    fn moves(&self, from: usize, rest: usize) -> impl Iterator<Item = (usize, usize)> + '_ {
        let hi = self.max_jump.min(rest);
        (self.min_jump..=hi).flat_map(move |d| {
            let left = from.checked_sub(d);
            let right = (from + d < self.n).then_some(from + d);
            [left, right].into_iter().flatten().map(move |to| (to, d))
        })
    }

    /// 手順を根から再生して得点を求める。ライブラリを一切通さない検算用。
    fn replay(&self, path: &[u16]) -> i64 {
        let mut pos = self.start;
        let mut visited = vec![false; self.n];
        visited[pos] = true;
        let mut score = self.a[pos];
        let mut used = 0;
        for &act in path {
            let (from, to) = unpack(act);
            assert_eq!(from, pos, "経路が繋がっていない");
            let d = to.abs_diff(from);
            assert!(
                self.min_jump <= d && d <= self.max_jump,
                "移動幅 {d} が不正"
            );
            used += d;
            assert!(used <= self.budget, "予算超過");
            if !visited[to] {
                visited[to] = true;
                score += self.a[to];
            }
            pos = to;
        }
        score
    }

    /// 全探索。
    fn brute_force(&self) -> i64 {
        let mut visited = vec![false; self.n];
        visited[self.start] = true;
        self.dfs(self.start, &mut visited, self.a[self.start], self.budget)
    }

    fn dfs(&self, pos: usize, visited: &mut [bool], score: i64, rest: usize) -> i64 {
        // いつ止めてもよいので、今の得点も候補。
        let mut best = score;
        for (to, d) in self.moves(pos, rest).collect::<Vec<_>>() {
            let fresh = !visited[to];
            visited[to] = true;
            let gain = if fresh { self.a[to] } else { 0 };
            best = best.max(self.dfs(to, visited, score + gain, rest - d));
            if fresh {
                visited[to] = false;
            }
        }
        best
    }
}

struct State {
    pos: usize,
    /// 訪問回数。`undo` のためにカウンタにしている。
    cnt: Vec<u32>,
}

#[derive(Clone, Copy)]
struct Node {
    score: i64,
    hash: u64,
    /// 消費済み予算。ターンと一致するが、`expand` にターンが渡らないので持たせる。
    used: u32,
}

fn pack(from: usize, to: usize) -> u16 {
    ((from as u16) << 8) | to as u16
}

fn unpack(act: u16) -> (usize, usize) {
    ((act >> 8) as usize, (act & 0xff) as usize)
}

impl Logic for Skip {
    type State = State;
    type Context = Context;
    type Action = u16;
    type Node = Node;
    type Value = i64;
    type Hash = u64;

    const MINIMIZE: bool = false;
    /// 予算を使い切る前でも解になる。予算切れの葉は子を作らず消えるので、
    /// 最良解はここで拾わないと失われる。
    const TERMINAL: Terminal = Terminal::KeepBest;

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
            used: 0,
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
        let rest = ctx.budget - node.used as usize;
        for (to, d) in ctx.moves(from, rest) {
            let fresh = state.cnt[to] == 0;
            let child = Node {
                score: node.score + if fresh { ctx.a[to] } else { 0 },
                hash: node.hash
                    ^ ctx.zpos[from]
                    ^ ctx.zpos[to]
                    ^ if fresh { ctx.zvis[to] } else { 0 },
                used: node.used + d as u32,
            };
            cur.push_at(d as u32, pack(from, to), child);
        }
    }

    fn value(node: &Node, _ctx: &Context) -> i64 {
        node.score
    }

    fn hash(node: &Node, _ctx: &Context) -> u64 {
        node.hash
    }

    fn is_terminal(_node: &Node, _ctx: &Context) -> bool {
        true
    }

    /// 同値をハッシュで割って全順序にする（`common` のトイ問題と同じ理由）。
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
            y.score.cmp(&x.score).then(x.hash.cmp(&y.hash))
        });
        order.truncate(width);
        out.extend(order);
    }
}

fn run(ctx: Context, width: usize) -> (Vec<u16>, i64) {
    let budget = ctx.budget;
    let mut s = BeamSearch::<Skip, LinkedTree<Skip>, _>::new(ctx, FixedWidth::new(width));
    let path = s.run(budget);
    let score = s.ctx().replay(&path);
    (path, score)
}

/// 幅を十分取れば全探索に一致する。世代スキップの本丸。
#[test]
fn matches_brute_force() {
    for seed in 0..64 {
        for max_jump in [2, 3] {
            let n = 5 + (seed as usize % 5);
            let budget = 4 + (seed as usize % 6);
            let ctx = Context::generate(seed, n, budget, max_jump);
            let expected = ctx.brute_force();
            let (_, score) = run(ctx, 100_000);
            assert_eq!(
                score, expected,
                "seed={seed} n={n} budget={budget} jump={max_jump}"
            );
        }
    }
}

/// 幅を絞っても解は実行可能で、全探索を超えない。
#[test]
fn narrow_width_stays_feasible() {
    for seed in 0..64 {
        for width in [1, 2, 3, 7, 32] {
            let n = 6 + (seed as usize % 8);
            let budget = 5 + (seed as usize % 8);
            let ctx = Context::generate(seed, n, budget, 3);
            let expected = ctx.brute_force();
            // replay が経路の連結性・移動幅・予算超過をすべて assert する。
            let (_, score) = run(ctx, width);
            assert!(
                score <= expected,
                "seed={seed} width={width} 全探索 {expected} を超えた {score}"
            );
        }
    }
}

/// `max_jump = 1` なら 1 世代ずつの探索と同じ問題になる。
/// このとき `PostorderTour` と結果が一致しなければならない。
#[test]
fn unit_jump_matches_postorder() {
    use beam::PostorderTour;

    for seed in 0..64 {
        for width in [1, 3, 8, 64] {
            let n = 6 + (seed as usize % 8);
            let budget = 4 + (seed as usize % 8);

            let ctx = Context::generate(seed, n, budget, 1);
            let mut s = BeamSearch::<Skip, LinkedTree<Skip>, _>::new(ctx, FixedWidth::new(width));
            let a = s.run(budget);
            let sa = s.ctx().replay(&a);

            let ctx = Context::generate(seed, n, budget, 1);
            let mut s =
                BeamSearch::<Skip, PostorderTour<Skip>, _>::new(ctx, FixedWidth::new(width));
            let b = s.run(budget);
            let sb = s.ctx().replay(&b);

            assert_eq!(sa, sb, "seed={seed} width={width} 得点が違う");
            assert_eq!(a.len(), b.len(), "seed={seed} width={width} 手順長が違う");
        }
    }
}

/// コーナーケース。
#[test]
fn corner_cases() {
    // 予算が最大跳躍より小さい（step > rest の手が生成されない）
    let ctx = Context::generate(0, 8, 1, 3);
    let (path, _) = run(ctx, 8);
    assert_eq!(path.len(), 1);

    // 幅1
    let ctx = Context::generate(1, 8, 7, 3);
    let (path, _) = run(ctx, 1);
    assert!(!path.is_empty());

    // マスが2つで、跳べるのは距離1だけ
    let mut ctx = Context::generate(2, 2, 5, 3);
    ctx.start = 0;
    let expected = ctx.brute_force();
    let (_, score) = run(ctx, 8);
    assert_eq!(score, expected);

    // 予算 0。根がそのまま答え
    let ctx = Context::generate(3, 6, 0, 3);
    let expected = ctx.brute_force();
    let mut s = BeamSearch::<Skip, LinkedTree<Skip>, _>::new(ctx, FixedWidth::new(4));
    let path = s.run(0);
    assert!(path.is_empty());
    assert_eq!(s.ctx().replay(&path), expected);
}

/// **到達する候補が1つも無いターン**を跨げる。
///
/// 最短の手が2ターン消費するので、奇数ターンに何も届かない状況が必ず生まれる。
/// `BeamSearch` が空の `chosen` で世代だけ進める経路（`Tree::has_pending`）を通す。
#[test]
fn empty_generations() {
    for seed in 0..64 {
        for width in [1, 4, 100_000] {
            let n = 7 + (seed as usize % 7);
            let budget = 5 + (seed as usize % 7);
            let ctx = Context::with_min_jump(seed, n, budget, 2, 3);
            let expected = ctx.brute_force();
            let (_, score) = run(ctx, width);
            if width == 100_000 {
                assert_eq!(score, expected, "seed={seed} n={n} budget={budget}");
            } else {
                assert!(score <= expected, "seed={seed} width={width}");
            }
        }
    }
}

/// 全ての手が同じ予算を食う場合。到達ターンが 2 の倍数だけになる。
#[test]
fn uniform_step_two() {
    for seed in 0..32 {
        let n = 8 + (seed as usize % 6);
        let budget = 6 + (seed as usize % 6);
        let ctx = Context::with_min_jump(seed, n, budget, 2, 2);
        let expected = ctx.brute_force();
        let (_, score) = run(ctx, 100_000);
        assert_eq!(score, expected, "seed={seed} n={n} budget={budget}");
    }
}

/// 深い探索でも `update_root` と削除が壊れない。
#[test]
fn deep() {
    for seed in 0..16 {
        let ctx = Context::generate(seed, 14, 80, 3);
        let (_, score) = run(ctx, 8);
        assert!(score > 0, "seed={seed}");
    }
}
