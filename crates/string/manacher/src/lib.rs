/// ```
/// use manacher::manacher;
/// let s   = [1, 2, 1, 1, 1, 2, 1, 2, 1];
/// let ret = manacher(&s);
/// assert_eq!(ret,vec![1, 2, 1, 4, 1, 2, 3, 2, 1]);
/// ```
pub fn manacher<T: Eq>(s: &[T]) -> Vec<usize> {
    let n = s.len();
    let mut ret = vec![0; n];
    let [mut i, mut j] = [0, 0];
    while i < n {
        while i >= j && i + j < n && s[i - j] == s[i + j] {
            j += 1;
        }
        ret[i] = j;
        let mut k = 1;
        while i >= k && k + ret[i - k] < j {
            ret[i + k] = ret[i - k];
            k += 1;
        }
        i += k;
        j -= k;
    }
    ret
}

#[cfg(test)]
mod tests {

    use rand::{Rng, SeedableRng, rngs::StdRng};

    use super::*;
    #[test]
    fn test_manacher() {
        let mut rng = StdRng::seed_from_u64(10);
        for _ in 0..10 {
            let n = rng.random_range(10..20);
            let s = (0..n).map(|_| rng.random_range(0..3)).collect::<Vec<_>>();
            let naive = {
                let mut ret = vec![0; n];
                for i in 0..n {
                    let mut j = 0;
                    while j <= i && i + j < n && s[i - j] == s[i + j] {
                        j += 1;
                    }
                    ret[i] = j;
                }
                ret
            };
            let ret = manacher(&s);
            assert_eq!(naive, ret);
        }
    }
}
