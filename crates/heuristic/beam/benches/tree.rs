//! バックエンドのマイクロベンチ。問題に依存しないので、ライブラリ単体の回帰ガードになる。
//!
//! # 測っていること
//!
//! 1. **`Action` のサイズが効くか**。`Action` 以外を完全に同じにしたまま
//!    u16 / u32 / u64 と振る。`tour` と `trace` のコピー量だけが変わるので、
//!    「木上のビームサーチはこのコピーが律速」という主張を数字で確かめられる。
//! 2. **深さの効き方**。`PostorderTour` の走査コストはビームに残った頂点たちの
//!    Steiner 木の大きさに比例するので、系統が合流しないと深さに対して超線形になる。
//! 3. **`PostorderTour` と `NaiveCopy` の分かれ目**。`NaiveCopy` の1ターンのコストは
//!    O(幅 × `State` のサイズ) なので、`State` のサイズを振らないと比較にならない。
//!    深さと `State` サイズの2軸で見る。
//!
//! ```sh
//! cargo bench -p beam
//! ```

use std::hint::black_box;
use std::marker::PhantomData;

use beam::{BeamSearch, Cursor, FixedWidth, Logic, NaiveCopy, PostorderTour, Tree};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

/// 一列のマスを歩き、初めて入ったマスの得点を得る。テストのトイ問題と同じ規則。
///
/// `Action` の型だけを差し替えられるようにしてある。
pub trait Edge: Copy + Default {
    fn pack(from: u16, to: u16) -> Self;
    fn unpack(self) -> (u16, u16);
}

macro_rules! impl_edge {
    ($t:ty) => {
        impl Edge for $t {
            #[inline]
            fn pack(from: u16, to: u16) -> Self {
                ((from as $t) << 8) | to as $t
            }
            #[inline]
            fn unpack(self) -> (u16, u16) {
                ((self >> 8) as u16, (self & 0xff) as u16)
            }
        }
    };
}
impl_edge!(u16);
impl_edge!(u32);
impl_edge!(u64);

pub struct Toy<A>(PhantomData<A>);

pub struct Context {
    n: usize,
    a: Vec<i64>,
    zpos: Vec<u64>,
    zvis: Vec<u64>,
    start: usize,
    /// `State` に載せる余分な重り（u64 いくつ分か）。
    ///
    /// `NaiveCopy` は毎ターン `State` を clone するのでここが効くが、
    /// `PostorderTour` は `State` を持ち回るだけなので効かない。
    /// 両者の分かれ目を見るための軸。
    pad: usize,
}

impl Context {
    fn new(seed: u64, n: usize, pad: usize) -> Self {
        assert!(n <= 256, "辺に 8 bit で詰めているので 256 マスまで");
        let mut s = seed;
        let mut next = || {
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let a = (0..n).map(|_| (next() % 100) as i64).collect();
        let zpos = (0..n).map(|_| next()).collect();
        let zvis = (0..n).map(|_| next()).collect();
        let start = (next() % n as u64) as usize;
        Self {
            n,
            a,
            zpos,
            zvis,
            start,
            pad,
        }
    }
}

#[derive(Clone)]
pub struct State {
    pos: u16,
    cnt: Vec<u32>,
    /// 重り。`apply` / `undo` では触らないので、clone のコストとしてだけ効く。
    #[allow(dead_code)]
    pad: Vec<u64>,
}

#[derive(Clone, Copy)]
pub struct Node {
    score: i64,
    hash: u64,
}

impl<A: Edge> Logic for Toy<A> {
    type State = State;
    type Context = Context;
    type Action = A;
    type Node = Node;
    type Value = i64;
    type Hash = u64;

    const MINIMIZE: bool = false;

    fn root(ctx: &Context) -> (State, Node) {
        let mut cnt = vec![0; ctx.n];
        cnt[ctx.start] = 1;
        (
            State {
                pos: ctx.start as u16,
                cnt,
                pad: vec![0; ctx.pad],
            },
            Node {
                score: ctx.a[ctx.start],
                hash: ctx.zpos[ctx.start] ^ ctx.zvis[ctx.start],
            },
        )
    }

    #[inline]
    fn apply(state: &mut State, act: A, _ctx: &Context) {
        let (_, to) = act.unpack();
        state.cnt[to as usize] += 1;
        state.pos = to;
    }

    #[inline]
    fn undo(state: &mut State, act: A, _ctx: &Context) {
        let (from, to) = act.unpack();
        state.cnt[to as usize] -= 1;
        state.pos = from;
    }

    fn expand(state: &mut State, ctx: &Context, cur: &mut impl Cursor<Self>) {
        let node = *cur.node();
        let from = state.pos as usize;
        let lo = from.saturating_sub(1);
        let hi = (from + 1).min(ctx.n - 1);
        for to in lo..=hi {
            let fresh = state.cnt[to] == 0;
            let child = Node {
                score: node.score + if fresh { ctx.a[to] } else { 0 },
                hash: node.hash
                    ^ ctx.zpos[from]
                    ^ ctx.zpos[to]
                    ^ if fresh { ctx.zvis[to] } else { 0 },
            };
            cur.push(A::pack(from as u16, to as u16), child);
        }
    }

    fn value(node: &Node, _ctx: &Context) -> i64 {
        node.score
    }

    fn hash(node: &Node, _ctx: &Context) -> u64 {
        node.hash
    }
}

fn search<A: Edge, T: Tree<Toy<A>>>(n: usize, turns: usize, width: usize, pad: usize) -> i64 {
    let ctx = Context::new(12345, n, pad);
    let mut s = BeamSearch::<Toy<A>, T, _>::new(ctx, FixedWidth::new(width));
    let path = s.run(turns);
    path.len() as i64
}

/// `Action` のサイズだけを振る。差が出れば `tour` のコピーが効いているということ。
fn action_size(c: &mut Criterion) {
    let (n, turns, width) = (64, 400, 256);
    let mut g = c.benchmark_group("action_size");
    g.throughput(Throughput::Elements((turns * width) as u64));
    g.bench_function("u16", |b| {
        b.iter(|| black_box(search::<u16, PostorderTour<Toy<u16>>>(n, turns, width, 0)))
    });
    g.bench_function("u32", |b| {
        b.iter(|| black_box(search::<u32, PostorderTour<Toy<u32>>>(n, turns, width, 0)))
    });
    g.bench_function("u64", |b| {
        b.iter(|| black_box(search::<u64, PostorderTour<Toy<u64>>>(n, turns, width, 0)))
    });
    g.finish();
}

/// `State` のサイズを振ってバックエンドの分かれ目を見る。
///
/// `NaiveCopy` は葉ごとに毎ターン `State` を clone するのでサイズに比例して重くなる。
/// `PostorderTour` は `State` を持ち回るだけなので効かない。したがって
/// **`State` が小さいときだけ `NaiveCopy` に勝ち目がある**。
fn state_size(c: &mut Criterion) {
    let (n, turns, width) = (64, 200, 256);
    let mut g = c.benchmark_group("state_size");
    // pad は u64 の個数。0 なら State は 264 バイト程度。
    for pad in [0usize, 128, 1024] {
        let bytes = 264 + pad * 8;
        g.bench_with_input(BenchmarkId::new("postorder", bytes), &pad, |b, &p| {
            b.iter(|| black_box(search::<u16, PostorderTour<Toy<u16>>>(n, turns, width, p)))
        });
        g.bench_with_input(BenchmarkId::new("naive", bytes), &pad, |b, &p| {
            b.iter(|| black_box(search::<u16, NaiveCopy<Toy<u16>>>(n, turns, width, p)))
        });
    }
    g.finish();
}

/// 深さを振る。
///
/// `PostorderTour` の走査コストは「ビームに残った頂点たちの Steiner 木の大きさ」に
/// 比例する。系統が合流せずに根の近くで分かれたままだと、これは
/// O(幅 × 深さ) まで膨らむ。この問題は全マス訪問後に位置ごとの系統が独立して
/// 走り続けるので、その最悪ケースを踏む。
///
/// 1ターンあたりの時間が伸びていれば超線形。`Throughput` で正規化して見る。
/// `NaiveCopy` との比較は `state_size` の方でやる（`State` のサイズを固定したまま
/// 深さだけ振っても、片方に有利な条件を選んだだけになる）。
fn depth(c: &mut Criterion) {
    let (n, width) = (64, 256);
    let mut g = c.benchmark_group("depth");
    for turns in [100usize, 400, 1600] {
        g.throughput(Throughput::Elements((turns * width) as u64));
        g.bench_with_input(BenchmarkId::new("postorder", turns), &turns, |b, &t| {
            b.iter(|| black_box(search::<u16, PostorderTour<Toy<u16>>>(n, t, width, 0)))
        });
    }
    g.finish();
}

criterion_group!(benches, action_size, state_size, depth);
criterion_main!(benches);
