pub trait Shakutori {
    type State: Copy;
    type Ans;
    fn push(&self, state: Self::State, r: usize) -> Self::State;
    fn pop(&self, state: Self::State, l: usize) -> Self::State;
    fn check(&self, state: Self::State, r: usize) -> bool;
    fn upd(&self, ans: &mut Self::Ans, state: Self::State, l: usize, r: usize);
    fn solve(&self, n: usize, init_state: Self::State, init_ans: Self::Ans) -> Self::Ans {
        let mut l = 0;
        let mut r = 0;
        let mut ans = init_ans;
        let mut state = init_state;

        while l < n {
            while r < n && self.check(state, r) {
                state = self.push(state, r);
                r += 1;
            }
            self.upd(&mut ans, state, l, r);
            if l == r {
                state = self.push(state, r);
                r += 1;
            }
            state = self.pop(state, l);
            l += 1;
        }
        ans
    }
}
