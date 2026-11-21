//! 探索木のバックエンド。
//!
//! | 実装 | 世代スキップ | 用途 |
//! |---|---|---|
//! | [`PostorderTour`] | 非対応 | 主力。帰りがけ順配列版 |
//! | [`LinkedTree`] | 対応 | 二重連鎖木版。定数倍は劣る |
//! | [`NaiveCopy`] | 非対応 | `State` をコピーする素朴版。正当性のリファレンス |

pub mod linked;
pub mod naive;
pub mod postorder;

pub use linked::LinkedTree;
pub use naive::NaiveCopy;
pub use postorder::PostorderTour;

use super::logic::{Cursor, Logic};

/// 探索木。
///
/// # 契約
///
/// - `candidates()` の添字は `commit` を呼ぶまで有効。`commit` 後は無効。
/// - `commit(chosen)` の `chosen` の**順序は問わない**。走査順の都合はバックエンドが
///   内部で引き受ける（`PostorderTour` は降順に走るが、それを `Selector` に要求しない）。
/// - `walk` が提示する頂点は、前ターンの `commit` で採用されたものだけ。
/// - `protect` で得た `Guard` は `commit` を跨いで有効。不要になったら `release` する。
pub trait Tree<L: Logic>: Cursor<L> {
    /// 経路を保持するハンドル。
    type Guard;

    fn new(root: L::Node, max_width: usize) -> Self;

    /// 前ターンの残骸を回収し、走査を開始する。
    fn begin(&mut self);

    /// 次の展開対象へ進み、`state` を同期させる。
    /// `true` を返したとき `node()` が展開対象を指す。
    fn walk(&mut self, state: &mut L::State, ctx: &L::Context) -> bool;

    /// 次のターンに到達する候補。選択の対象になる。
    ///
    /// 世代スキップ対応のバックエンドでは、**今ターンの展開で生まれたものとは限らない**。
    /// 過去のターンから `push_at(step > 1)` で飛んできた候補も、到達ターンが揃った
    /// ここで初めて姿を現す。選択は同じターンに到達する候補どうしでのみ行われる。
    fn candidates(&self) -> &[L::Node];

    /// `chosen`（`candidates()` の添字。順不同）を木に反映し、世代を進める。
    ///
    /// `chosen` は空でもよい。世代スキップ中で今ターンに到達する候補が
    /// 1 つも無い場合、`BeamSearch` は空の `chosen` で世代だけ進める。
    fn commit(&mut self, chosen: &[u32]);

    /// **後の**ターンに到達する候補を抱えているか。
    ///
    /// `candidates()` が空でも、これが `true` なら探索は続く。世代スキップ非対応の
    /// バックエンドでは常に `false`。
    fn has_pending(&self) -> bool {
        false
    }

    /// `candidates()[i]` への経路を保存するよう指示する。`commit` を跨いで有効。
    fn protect(&mut self, i: u32) -> Self::Guard;
    fn release(&mut self, g: Self::Guard);
    fn path(&self, g: &Self::Guard) -> Vec<L::Action>;
}
