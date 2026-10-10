/// 順序に制約のない Convex Hull Trick (Li Chao Tree)。
///
/// あらかじめ決めた整数区間 `[lo, hi]` 上で、直線 `y = a*x + b` の追加
/// (`add_line`) と、ある `x` での最小値 (または `max` モードなら最大値)
/// の問い合わせ (`query`) をそれぞれ `O(log(hi-lo))` で行う。
/// `MonotoneCht` と異なり追加・問い合わせの順序は自由。
///
/// ノードは必要になった時点で確保する (動的セグメント木) ので、
/// `hi - lo` が大きくてもメモリは挿入した直線の本数 `O(log(hi-lo))` 倍で済む。
pub struct LiChaoTree {
    maximize: bool,
    lo: i64,
    hi: i64,
    nodes: Vec<Node>,
}

struct Node {
    line: Option<(i64, i64)>,
    left: Option<u32>,
    right: Option<u32>,
}

impl Node {
    fn empty() -> Self {
        Self {
            line: None,
            left: None,
            right: None,
        }
    }
}

impl LiChaoTree {
    /// 最小値を問い合わせるための Li Chao Tree。`x` は常に `[lo, hi]` の範囲。
    pub fn new_min(lo: i64, hi: i64) -> Self {
        assert!(lo <= hi);
        Self {
            maximize: false,
            lo,
            hi,
            nodes: vec![Node::empty()],
        }
    }

    /// 最大値を問い合わせるための Li Chao Tree。
    pub fn new_max(lo: i64, hi: i64) -> Self {
        assert!(lo <= hi);
        Self {
            maximize: true,
            lo,
            hi,
            nodes: vec![Node::empty()],
        }
    }

    #[inline]
    fn eval(line: (i64, i64), x: i64) -> i64 {
        line.0 * x + line.1
    }

    fn child_left(&mut self, idx: usize) -> usize {
        if let Some(c) = self.nodes[idx].left {
            c as usize
        } else {
            self.nodes.push(Node::empty());
            let c = self.nodes.len() - 1;
            self.nodes[idx].left = Some(c as u32);
            c
        }
    }

    fn child_right(&mut self, idx: usize) -> usize {
        if let Some(c) = self.nodes[idx].right {
            c as usize
        } else {
            self.nodes.push(Node::empty());
            let c = self.nodes.len() - 1;
            self.nodes[idx].right = Some(c as u32);
            c
        }
    }

    /// 直線 `y = a*x + b` を追加する。`O(log(hi-lo))`。
    pub fn add_line(&mut self, a: i64, b: i64) {
        let line = if self.maximize { (-a, -b) } else { (a, b) };
        self.add_line_rec(0, self.lo, self.hi, line);
    }

    fn add_line_rec(&mut self, idx: usize, l: i64, r: i64, line: (i64, i64)) {
        let cur = match self.nodes[idx].line {
            None => {
                self.nodes[idx].line = Some(line);
                return;
            }
            Some(cur) => cur,
        };

        let mid = l + (r - l) / 2;
        let (mut keep, mut rest) = (cur, line);
        if Self::eval(keep, mid) > Self::eval(rest, mid) {
            std::mem::swap(&mut keep, &mut rest);
        }
        self.nodes[idx].line = Some(keep);
        if l == r {
            return;
        }

        if Self::eval(keep, l) > Self::eval(rest, l) {
            let child = self.child_left(idx);
            self.add_line_rec(child, l, mid, rest);
        } else if Self::eval(keep, r) > Self::eval(rest, r) {
            let child = self.child_right(idx);
            self.add_line_rec(child, mid + 1, r, rest);
        }
        // それ以外は `rest` がこの区間全体で `keep` に劣るので捨てる。
    }

    /// `x` での最小値 (または `max` モードなら最大値)。直線が1本もなければ
    /// `min` モードは `i64::MAX`、`max` モードは `-i64::MAX` を返す。
    /// `O(log(hi-lo))`。
    pub fn query(&self, x: i64) -> i64 {
        assert!(self.lo <= x && x <= self.hi, "x out of range");
        let mut idx = 0usize;
        let mut l = self.lo;
        let mut r = self.hi;
        let mut best = i64::MAX;
        loop {
            if let Some(line) = self.nodes[idx].line {
                best = best.min(Self::eval(line, x));
            }
            if l == r {
                break;
            }
            let mid = l + (r - l) / 2;
            if x <= mid {
                match self.nodes[idx].left {
                    Some(c) => {
                        idx = c as usize;
                        r = mid;
                    }
                    None => break,
                }
            } else {
                match self.nodes[idx].right {
                    Some(c) => {
                        idx = c as usize;
                        l = mid + 1;
                    }
                    None => break,
                }
            }
        }
        if self.maximize { -best } else { best }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xorshift() -> impl FnMut() -> u64 {
        let mut state = 88172645463325252u64;
        move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        }
    }

    #[test]
    fn min_matches_naive() {
        let mut next = xorshift();
        let mut tree = LiChaoTree::new_min(-500, 500);
        let mut lines: Vec<(i64, i64)> = Vec::new();
        for _ in 0..200 {
            let a = (next() % 21) as i64 - 10;
            let b = (next() % 2001) as i64 - 1000;
            tree.add_line(a, b);
            lines.push((a, b));
            let x = (next() % 1001) as i64 - 500;
            let expected = lines.iter().map(|&(a, b)| a * x + b).min().unwrap();
            assert_eq!(tree.query(x), expected, "x={x}");
        }
    }

    #[test]
    fn max_matches_naive() {
        let mut next = xorshift();
        let mut tree = LiChaoTree::new_max(-500, 500);
        let mut lines: Vec<(i64, i64)> = Vec::new();
        for _ in 0..200 {
            let a = (next() % 21) as i64 - 10;
            let b = (next() % 2001) as i64 - 1000;
            tree.add_line(a, b);
            lines.push((a, b));
            let x = (next() % 1001) as i64 - 500;
            let expected = lines.iter().map(|&(a, b)| a * x + b).max().unwrap();
            assert_eq!(tree.query(x), expected, "x={x}");
        }
    }

    #[test]
    fn single_point_domain() {
        let mut tree = LiChaoTree::new_min(7, 7);
        tree.add_line(3, 1); // 3*7+1 = 22
        tree.add_line(-1, 100); // -1*7+100 = 93
        assert_eq!(tree.query(7), 22);
    }

    #[test]
    fn empty_tree_returns_sentinel() {
        let tree_min = LiChaoTree::new_min(0, 10);
        assert_eq!(tree_min.query(5), i64::MAX);
        let tree_max = LiChaoTree::new_max(0, 10);
        assert_eq!(tree_max.query(5), -i64::MAX);
    }
}
