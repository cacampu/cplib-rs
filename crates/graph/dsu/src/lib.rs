pub struct Dsu {
    size: usize,
    parent_or_size: Vec<i32>,
}

impl Dsu {
    pub fn new(n: usize) -> Self {
        Self {
            size: n,
            parent_or_size: vec![-1; n],
        }
    }
    pub fn find(&mut self, x: usize) -> usize {
        if self.parent_or_size[x] < 0 {
            return x;
        }
        let root = self.find(self.parent_or_size[x] as usize);
        self.parent_or_size[x] = root as i32;
        root
    }
    pub fn union(&mut self, x: usize, y: usize) -> usize {
        let mut root_x = self.find(x);
        let mut root_y = self.find(y);
        if root_x == root_y {
            return root_x;
        }
        if -self.parent_or_size[root_x] < -self.parent_or_size[root_y] {
            std::mem::swap(&mut root_x, &mut root_y);
        }
        self.parent_or_size[root_x] += self.parent_or_size[root_y];
        self.parent_or_size[root_y] = root_x as i32;
        root_x
    }
    pub fn union_with(&mut self, x: usize, y: usize, mut f: impl FnMut(usize, usize)) -> usize {
        let mut root_x = self.find(x);
        let mut root_y = self.find(y);
        if root_x == root_y {
            return root_x;
        }
        if -self.parent_or_size[root_x] < -self.parent_or_size[root_y] {
            std::mem::swap(&mut root_x, &mut root_y);
        }
        self.parent_or_size[root_x] += self.parent_or_size[root_y];
        self.parent_or_size[root_y] = root_x as i32;
        f(root_x, root_y);
        root_x
    }

    pub fn size(&mut self, x: usize) -> usize {
        let root = self.find(x);
        -self.parent_or_size[root] as usize
    }
    pub fn same(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }
    pub fn groups(&mut self) -> Vec<Vec<usize>> {
        let mut group_size = vec![0; self.size];
        let mut roots_buf = vec![0; self.size];
        for i in 0..self.size {
            let root = self.find(i);
            roots_buf[i] = root;
            group_size[root] += 1;
        }
        let mut offsets = vec![0; self.size + 1];
        for i in 0..self.size {
            offsets[i + 1] = offsets[i] + group_size[i];
        }

        let mut groups_flat = vec![0; self.size];
        let mut pts = offsets.clone();
        for i in 0..self.size {
            let root = roots_buf[i];
            groups_flat[pts[root]] = i;
            pts[root] += 1;
        }
        let mut groups = Vec::with_capacity(self.size);
        for i in 0..self.size {
            if group_size[i] > 0 {
                let (l, r) = (offsets[i], offsets[i + 1]);
                groups.push(groups_flat[l..r].to_vec());
            }
        }
        groups
    }
}
