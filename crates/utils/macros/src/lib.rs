#[macro_export]
macro_rules! chmin {
    ($a:expr,$b:expr) => {
        if $a > $b {
            $a = $b;
            true
        } else {
            false
        }
    };
}
#[macro_export]
macro_rules! chmax {
    ($a:expr,$b:expr) => {
        if $a < $b {
            $a = $b;
            true
        } else {
            false
        }
    };
}

/// 多次元配列を作成
/// # Examples
/// ```
/// use macros::mat;
/// let a = mat![3, 4, 5; 0];
/// assert_eq!(a, vec![vec![vec![0; 5]; 4]; 3]);
/// ```
#[macro_export]
macro_rules! mat {
	($d:expr; $e:expr) => { vec![$e; $d] };
	($d:expr, $($ds:expr),+; $e:expr) => { vec![mat![$($ds),+; $e]; $d] };
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
        let a = mat![3, 4, 5 ;0];
        assert_eq!(a, vec![vec![vec![0; 5]; 4]; 3]);
    }
}
