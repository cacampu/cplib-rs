//! 差分更新型ビームサーチ。使い方と各バックエンドの説明は [`beam`] を参照。
//! 開発時は `profile` feature でフェーズごとの計測を有効にできる。

pub mod beam;
#[cfg(feature = "profile")]
pub mod profile;

pub use beam::*;
