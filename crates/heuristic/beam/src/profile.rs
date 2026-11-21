//! 探索のフェーズ別の所要時間を測る。開発用。
//!
//! `profile` feature を有効にすると利用できる。
//!
//! [`BeamSearch::run`](crate::beam::BeamSearch::run) と同じ順序でループを回すが、
//! 各フェーズを個別に計測する。`Terminal` と `Guard` は扱わないので、打ち切りのある
//! 問題でも最後まで回る（性能を見るには十分）。
//!
//! ```ignore
//! let phases = profile::<MyLogic, PostorderTour<MyLogic>>(ctx, 1000, 500);
//! eprintln!("{}", phases.report());
//! ```

use std::fmt::Write as _;
use std::time::Instant;

use crate::beam::{Logic, Selector, Tree};

#[derive(Default, Clone, Copy)]
pub struct Phases {
    pub advance: f64,
    /// 木の走査（`walk`）と `Logic::expand` の合計。
    pub expand: f64,
    pub select: f64,
    pub commit: f64,
    pub turns: usize,
    /// 全ターンで生成された候補の総数。
    pub candidates: usize,
    /// 全ターンで実際に採用された数の総数。
    ///
    /// 幅に届いていなければ、重複除去や `Logic::select` の制約で落ちている。
    /// 公称の幅と実効的な幅を取り違えないために出している。
    pub chosen: usize,
}

impl Phases {
    pub fn total(&self) -> f64 {
        self.advance + self.expand + self.select + self.commit
    }

    pub fn report(&self) -> String {
        let total = self.total();
        let mut s = String::new();
        let _ = writeln!(
            s,
            "turns={} 候補数/ターン={} 採用数/ターン={}",
            self.turns,
            self.candidates.checked_div(self.turns).unwrap_or(0),
            self.chosen.checked_div(self.turns).unwrap_or(0)
        );
        for (name, t) in [
            ("advance", self.advance),
            ("expand ", self.expand),
            ("select ", self.select),
            ("commit ", self.commit),
        ] {
            let pct = if total > 0.0 { t / total * 100.0 } else { 0.0 };
            let _ = writeln!(s, "  {name} {t:7.3}s  {pct:5.1}%");
        }
        let _ = write!(s, "  total   {total:7.3}s");
        s
    }
}

/// 幅を固定して `turns` ターン回し、フェーズ別の所要時間を返す。
///
/// 幅を固定するのは、時間制限つきで回すと計測が自分自身に影響してしまうため。
pub fn profile<L, T>(mut ctx: L::Context, width: usize, turns: usize) -> Phases
where
    L: Logic,
    T: Tree<L>,
{
    let (mut state, root) = L::root(&ctx);
    let mut tree = T::new(root, width);
    let mut selector = Selector::<L>::new();
    let mut chosen: Vec<u32> = Vec::with_capacity(width);
    let mut p = Phases::default();

    for turn in 0..turns {
        let t0 = Instant::now();
        L::advance(&mut ctx, turn, width);

        let t1 = Instant::now();
        tree.begin();
        while tree.walk(&mut state, &ctx) {
            L::expand(&mut state, &ctx, &mut tree);
        }

        let t2 = Instant::now();
        if tree.candidates().is_empty() {
            p.advance += (t1 - t0).as_secs_f64();
            p.expand += (t2 - t1).as_secs_f64();
            if !tree.has_pending() {
                break;
            }
            // 世代スキップ中で、このターンに到達する候補が無い。世代だけ進める。
            // ここを break にすると、根が step > 1 しか押さない問題で 0 ターンになる。
            let t3 = Instant::now();
            tree.commit(&[]);
            p.commit += t3.elapsed().as_secs_f64();
            p.turns += 1;
            continue;
        }
        p.candidates += tree.candidates().len();
        chosen.clear();
        L::select(
            tree.candidates(),
            &mut ctx,
            width,
            &mut selector,
            &mut chosen,
        );

        let t3 = Instant::now();
        p.chosen += chosen.len();
        tree.commit(&chosen);
        let t4 = Instant::now();

        p.advance += (t1 - t0).as_secs_f64();
        p.expand += (t2 - t1).as_secs_f64();
        p.select += (t3 - t2).as_secs_f64();
        p.commit += (t4 - t3).as_secs_f64();
        p.turns += 1;
    }
    p
}
