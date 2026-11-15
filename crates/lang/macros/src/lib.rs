//! 競プロ用の小さなマクロ。再帰・呼び出し時借用は `recurfn!` / `lazyfn!` / `lazymac!`。

#![forbid(unsafe_code)]

mod closure;
#[doc(hidden)]
pub use closure::{Call as __Call, CallState as __CallState, Recursive as __Recursive};

/// 変数・フィールドを交換する。同じ配列内では `swap!(a; i, j)` を使う。
///
/// ```
/// use macros::swap;
/// let (mut a, mut b) = (1, 2);
/// swap!(a, b);
/// assert_eq!((a, b), (2, 1));
/// let mut values = vec![String::from("a"), String::from("b")];
/// swap!(values; 0, 1);
/// assert_eq!(values, ["b", "a"]);
/// ```
#[macro_export]
macro_rules! swap {
    ($values:expr; $i:expr, $j:expr $(,)?) => {
        ($values).swap($i, $j)
    };
    ($a:expr, $b:expr $(,)?) => {
        ::std::mem::swap(&mut $a, &mut $b)
    };
}

#[macro_export]
macro_rules! chmin {
    ($a:expr,$b:expr $(,)?) => {{
        let candidate = $b;
        let target = &mut $a;
        if candidate < *target {
            *target = candidate;
            true
        } else {
            false
        }
    }};
}
#[macro_export]
macro_rules! chmax {
    ($a:expr,$b:expr $(,)?) => {{
        let candidate = $b;
        let target = &mut $a;
        if candidate > *target {
            *target = candidate;
            true
        } else {
            false
        }
    }};
}

/// 多次元配列を作成
/// # Examples
/// ```
/// use macros::mat;
/// let a = mat![0; 3, 4, 5];
/// assert_eq!(a, vec![vec![vec![0; 5]; 4]; 3]);
/// ```
#[macro_export]
macro_rules! mat {
	($e:expr; $d:expr) => { ::std::vec![$e; $d] };
	($e:expr; $d:expr, $($ds:expr),+) => { ::std::vec![$crate::mat![$e; $($ds),+]; $d] };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_chminmax() {
        let mut a = [0, 1, 2, 3, 4];
        assert!(chmin!(a[3], a[0]));
        assert!(chmax!(a[0], a[4]));
        assert_eq!(a[3], 0);
        assert_eq!(a[0], 4);
    }
    #[test]
    fn test_mat() {
        let a = mat![0; 3, 4, 5];
        assert_eq!(a, vec![vec![vec![0; 5]; 4]; 3]);
    }
}
