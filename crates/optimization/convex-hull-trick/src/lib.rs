//! Convex Hull Trick。直線集合 `{a*x+b}` を管理し、ある `x` での
//! 最小値 (または最大値) を高速に求める。
//!
//! - [`MonotoneCht`]: 追加する直線の傾きと問い合わせる `x` がどちらも
//!   単調であることを要求する代わりに、各操作が償却 `O(1)`。
//! - [`LiChaoTree`][]: 傾き・問い合わせの順序に制約がない代わりに、
//!   あらかじめ決めた `x` の範囲 `[x_lo, x_hi]` 上で各操作 `O(log(x_hi-x_lo))`。

mod li_chao;
mod monotone;

pub use li_chao::LiChaoTree;
pub use monotone::MonotoneCht;
