//! 制御フロー。
//!
//! 「展開 → 終了判定 → 選択 → 反映」の順。選択対象は展開の結果生まれた候補なので、
//! 選択を先に置く設計は成立しない。終了判定を選択の**前**に置くのは、幅や重複除去で
//! 落ちた実行可能解を見逃さないため。

use std::time::Instant;

use super::logic::{Logic, Terminal};
use super::selector::Selector;
use super::tree::Tree;
use super::width::WidthPolicy;

/// `Logic::LOG` が出す 1 ターンの内訳。
///
/// `commit` は**前のターンの**反映。ターンの区切りが選択の直後なので、そこだけ
/// 1 つ前のぶんが乗る。
#[derive(Default, Clone, Copy)]
struct Phases {
    commit: f64,
    expand: f64,
    judge: f64,
    select: f64,
}

impl Phases {
    fn total(&self) -> f64 {
        self.commit + self.expand + self.judge + self.select
    }
}

pub struct BeamSearch<L: Logic, T: Tree<L>, W> {
    /// 探索中の唯一の時計。`WidthPolicy` に累計経過を渡すのに使う。
    clock: Instant,
    state: L::State,
    ctx: L::Context,
    tree: T,
    selector: Selector<L>,
    policy: W,
    best: Option<(L::Value, L::Node, T::Guard)>,
}

impl<L, T, W> BeamSearch<L, T, W>
where
    L: Logic,
    T: Tree<L>,
    W: WidthPolicy,
{
    pub fn new(ctx: L::Context, policy: W) -> Self {
        let (state, root) = L::root(&ctx);
        let tree = T::new(root, policy.max_width());
        Self {
            clock: Instant::now(),
            state,
            ctx,
            tree,
            selector: Selector::new(),
            policy,
            best: None,
        }
    }

    pub fn ctx(&self) -> &L::Context {
        &self.ctx
    }

    pub fn state(&self) -> &L::State {
        &self.state
    }

    pub fn policy(&self) -> &W {
        &self.policy
    }

    /// 探索を終えて `Context` を取り出す。出力の組み立てに使う。
    pub fn into_ctx(self) -> L::Context {
        self.ctx
    }

    pub fn run(&mut self, max_turn: usize) -> Vec<L::Action> {
        let mut chosen: Vec<u32> = Vec::with_capacity(self.policy.max_width());
        // 直前のターンまでの累計。ターンごとの所要時間を出すのに使う。
        let mut prev_elapsed = 0.0;

        for turn in 0..max_turn {
            // 幅は展開より先に決める。advance が幅から作る量（多様性の閾値など）を
            // ctx に置けるようにするため。suggest は ctx を見ないので順序を入れ替えて
            // 問題ない。
            let width = self.policy.suggest(turn, max_turn);
            L::advance(&mut self.ctx, turn, width);

            // --- 展開 ---
            let t_begin = self.stamp();
            self.tree.begin();
            while self.tree.walk(&mut self.state, &self.ctx) {
                L::expand(&mut self.state, &self.ctx, &mut self.tree);
            }
            let t_expanded = self.stamp();
            if self.tree.candidates().is_empty() {
                if self.tree.has_pending() {
                    // 世代スキップ中。このターンに到達する候補が無いだけで、後の
                    // ターンに届く候補は残っている。世代だけ進める。
                    if L::LOG {
                        eprintln!("turn={turn:<6} 到達する候補なし（世代スキップ）");
                    }
                    self.tree.commit(&[]);
                    continue;
                }
                // 候補が尽きた。終端解は直前の commit で確定した葉だが、それらは
                // commit 済みで添字からは触れない（`Tree` に手段が無い）。
                // TERMINAL を設定してある問題なら best を拾えているので、そちらを返す。
                assert!(
                    self.best.is_some(),
                    "turn {turn}: 候補が尽きました。expand がどの葉からも子を生成せず、\
                     TERMINAL も設定されていないため解を取り出せません。\
                     途中で打ち切りうる問題では Terminal::KeepBest を設定してください"
                );
                break;
            }

            // --- 終了判定（選択の前。候補全体が対象） ---
            if L::TERMINAL != Terminal::Ignore
                && let Some(i) = self.best_terminal()
            {
                self.update_best(i);
                if L::TERMINAL == Terminal::HaltOnFirst {
                    return self.take_best();
                }
            }

            // --- 選択 ---
            let t_judged = self.stamp();
            chosen.clear();
            if L::LOG {
                self.selector.reset_unique();
            }
            L::select(
                self.tree.candidates(),
                &mut self.ctx,
                width,
                &mut self.selector,
                &mut chosen,
            );
            assert!(!chosen.is_empty(), "候補が1つも採用されませんでした");

            if L::LOG {
                let end = self.stamp();
                self.log_turn(
                    turn,
                    width,
                    chosen.len(),
                    Phases {
                        commit: t_begin - prev_elapsed,
                        expand: t_expanded - t_begin,
                        judge: t_judged - t_expanded,
                        select: end - t_judged,
                    },
                    end,
                );
                prev_elapsed = end;
            }

            // --- 最終ターンの確定 ---
            //
            // commit すると添字が無効になるので、最終ターンだけは commit せずに
            // ここで確定させる。毎ターン protect すれば済む話ではない。protect は
            // 経路を実体化するので 1 回 O(深さ)、毎ターンやると O(ターン数^2) になる。
            if turn + 1 == max_turn {
                if self.best.is_none()
                    && let Some(i) = self.best_candidate()
                {
                    self.update_best(i);
                }
                break;
            }

            // --- 反映 ---
            self.tree.commit(&chosen);
            self.policy
                .observe(turn, chosen.len(), self.clock.elapsed().as_secs_f64());
        }

        self.take_best()
    }

    /// `L::LOG` のときだけ時計を読む。`false` なら定数畳み込みで消える。
    #[inline]
    fn stamp(&self) -> f64 {
        if L::LOG {
            self.clock.elapsed().as_secs_f64()
        } else {
            0.0
        }
    }

    /// `L::LOG` のときだけ呼ばれる 1 ターン分の内訳。
    ///
    /// `uniq` は重複除去後の件数。`Logic::select` が重複除去を通らない選択をした
    /// 場合は測れないので `-` になる。
    fn log_turn(&self, turn: usize, width: usize, chosen: usize, ph: Phases, elapsed: f64) {
        let cand = self.tree.candidates().len();
        let uniq = self.selector.unique();
        let dedup = if uniq == 0 {
            "     -".to_string()
        } else {
            format!("{:5.1}%", 100.0 * (1.0 - uniq as f64 / cand as f64))
        };
        let uniq = if uniq == 0 {
            "-".to_string()
        } else {
            uniq.to_string()
        };
        let ms = |x: f64| x * 1000.0;
        eprintln!(
            "turn={turn:<5} width={width:<6} cand={cand:<7} uniq={uniq:<7} 除去={dedup} \
             chosen={chosen:<6} | 反映{:6.2} 展開{:6.2} 判定{:6.2} 選択{:6.2} = {:7.2}ms \
             累計{elapsed:7.3}s",
            ms(ph.commit),
            ms(ph.expand),
            ms(ph.judge),
            ms(ph.select),
            ms(ph.total()),
        );
    }

    fn update_best(&mut self, i: u32) {
        let node = self.tree.candidates()[i as usize];
        let v = L::final_value(&node, &self.ctx);
        let better = match &self.best {
            None => true,
            Some((bv, _, _)) => Self::is_better(v, *bv),
        };
        if better {
            if let Some((_, _, g)) = self.best.take() {
                self.tree.release(g);
            }
            let g = self.tree.protect(i);
            self.best = Some((v, node, g));
        }
    }

    /// 実行可能解のうち `final_value` 最良のもの。無ければ `None`。
    fn best_terminal(&self) -> Option<u32> {
        self.pick(true)
    }

    /// 候補全体のうち `final_value` 最良のもの。空なら `None`。
    fn best_candidate(&self) -> Option<u32> {
        self.pick(false)
    }

    /// 同点のときは `hash` の小さい方を採る。単に「最初に見つかった最良」にすると
    /// 候補の構築順に依存し、バックエンドを差し替えただけで別の手順が返る。
    fn pick(&self, terminal_only: bool) -> Option<u32> {
        let mut best: Option<(L::Value, L::Hash, u32)> = None;
        for (i, node) in self.tree.candidates().iter().enumerate() {
            if terminal_only && !L::is_terminal(node, &self.ctx) {
                continue;
            }
            let v = L::final_value(node, &self.ctx);
            let h = L::hash(node, &self.ctx);
            let better = match best {
                None => true,
                Some((bv, bh, _)) => Self::is_better(v, bv) || (v == bv && h < bh),
            };
            if better {
                best = Some((v, h, i as u32));
            }
        }
        best.map(|(_, _, i)| i)
    }

    fn take_best(&mut self) -> Vec<L::Action> {
        match self.best.take() {
            Some((_, _, g)) => {
                let path = self.tree.path(&g);
                self.tree.release(g);
                path
            }
            None => Vec::new(),
        }
    }

    #[inline]
    fn is_better(v: L::Value, bv: L::Value) -> bool {
        if L::MINIMIZE { v < bv } else { v > bv }
    }
}
