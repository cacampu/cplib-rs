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
        let mut idx = vec![0; self.len()];
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

        let mut p = lis.len();
        let mut ret = vec![0; p];
        p -= 1;
        for i in (0..self.len()).rev() {
            if idx[i] == p {
                ret[p] = i;
                p -= 1;
            }
        }
        ret
    }
}
