pub trait LIS<T: Ord> {
    fn lis(&self) -> usize;
    fn lis_restore(&self) -> Vec<usize>;
}
impl<T: Ord + Clone> LIS<T> for [T] {
    fn lis(&self) -> usize {
        let mut lis = vec![];
        for x in self {
            let pos = lis.partition_point(|&y| y < x);
            if pos == lis.len() {
                lis.push(x);
            } else {
                lis[pos] = x;
            }
        }
        lis.len()
    }
    fn lis_restore(&self) -> Vec<usize> {
        let mut idx = Vec::with_capacity(self.len());
        let mut lis = vec![];
        for x in self {
            let pos = lis.partition_point(|&y| y < x);
            if pos == lis.len() {
                lis.push(x);
            } else {
                lis[pos] = x;
            }
            idx.push(pos)
        }

        // 後ろから見て `idx` が len-1, len-2, ... となる要素を貪欲に拾う。
        // 同じ `idx` を持つ要素の値は添字順に非増加なので、右端を取れば必ず増加列になる。
        let mut rest = lis.len();
        let mut ret = vec![0; rest];
        for i in (0..self.len()).rev() {
            if rest > 0 && idx[i] == rest - 1 {
                rest -= 1;
                ret[rest] = i;
            }
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_increasing_subsequence<T: Ord>(a: &[T], idx: &[usize]) -> bool {
        idx.windows(2).all(|w| w[0] < w[1] && a[w[0]] < a[w[1]])
    }

    #[test]
    fn lis_len() {
        assert_eq!([2, 1, 3, 5, 4].lis(), 3);
        assert_eq!([1, 1, 1].lis(), 1);
        assert_eq!(([] as [i32; 0]).lis(), 0);
    }

    #[test]
    fn restore() {
        let a = [2, 1, 3, 5, 4];
        let idx = a.lis_restore();
        assert_eq!(idx.len(), a.lis());
        assert!(is_increasing_subsequence(&a, &idx));
    }

    #[test]
    fn restore_empty_and_flat() {
        assert_eq!(([] as [i32; 0]).lis_restore(), Vec::<usize>::new());
        assert_eq!([7, 7, 7].lis_restore().len(), 1);
    }

    #[test]
    fn restore_matches_len_on_random() {
        let mut state = 12345u64;
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state >> 33) % 20
        };
        for _ in 0..200 {
            let a: Vec<u64> = (0..30).map(|_| next()).collect();
            let idx = a.lis_restore();
            assert_eq!(idx.len(), a.lis(), "{a:?}");
            assert!(is_increasing_subsequence(&a, &idx), "{a:?} -> {idx:?}");
        }
    }
}
