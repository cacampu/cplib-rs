use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use prim::IntN;

fn zip_with<T, U, S, const N: usize>(
    a: [T; N], b: [U; N], f: impl Fn(T, U) -> S,
) -> [S; N] {
    let mut a = a.into_iter();
    let mut b = b.into_iter();
    std::array::from_fn(|_| f(a.next().unwrap(), b.next().unwrap()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid<T> {
    pub size: [usize; 2],
    pub inner: Vec<T>,
}

#[inline]
fn in_bounds<const N: usize>(coord: [isize; N], size: [usize; N]) -> bool {
    coord.into_iter().zip(size).all(|(c, s)| c >= 0 && c < s as isize)
}

impl<T> Grid<T> {
    pub fn new(size: [usize; 2], default: T) -> Self
    where
        T: Clone,
    {
        Self {
            size,
            inner: vec![default; size[0] * size[1]],
        }
    }

    #[inline]
    pub fn in_bounds(&self, point: impl IntN<2>) -> bool {
        in_bounds(point.to_ints(), self.size)
    }

    pub fn neighbors_custom<'d, P, D>(
        &self,
        point: P,
        d: &'d [D],
    ) -> impl Iterator<Item = [usize; 2]> + 'd
    where
        P: IntN<2>,
        D: IntN<2> + Copy + 'd,
    {
        let p = point.to_ints();
        let size = self.size;
        d.iter().filter_map(move |&dir| {
            let nb = zip_with(p, dir.to_ints(), |pi, di| pi + di);
            in_bounds(nb, size).then(|| nb.to_index())
        })
    }

    pub fn neighbors_custom_indexed<'d, P, D>(
        &self,
        point: P,
        d: &'d [D],
    ) -> impl Iterator<Item = (usize, [usize; 2])> + 'd
    where
        P: IntN<2>,
        D: IntN<2> + Copy + 'd,
    {
        let p = point.to_ints();
        let size = self.size;
        d.iter().enumerate().filter_map(move |(idx, &dir)| {
            let nb = zip_with(p, dir.to_ints(), |pi, di| pi + di);
            in_bounds(nb, size).then(|| (idx, nb.to_index()))
        })
    }

    /// 点 `point` から方向 `d` に1歩移動した座標を返す。範囲外なら `None`。
    pub fn step(&self, point: impl IntN<2>, d: impl IntN<2>) -> Option<[usize; 2]> {
        let nb = zip_with(point.to_ints(), d.to_ints(), |pi, di| pi + di);
        in_bounds(nb, self.size).then(|| nb.to_index())
    }

    pub const D4: [[isize; 2]; 4] = [[1, 0], [0, 1], [-1, 0], [0, -1]];
    pub fn neighbors<P: IntN<2>>(&self, point: P) -> impl Iterator<Item = [usize; 2]> {
        self.neighbors_custom(point, &Self::D4)
    }
    /// `(D4のインデックス, 隣接座標)` を返す。`Grid::D4[idx]` で方向ベクトルを参照できる。
    pub fn neighbors_indexed<P: IntN<2>>(
        &self,
        point: P,
    ) -> impl Iterator<Item = (usize, [usize; 2])> {
        self.neighbors_custom_indexed(point, &Self::D4)
    }

    #[rustfmt::skip]
    pub const D8: [[isize; 2]; 8] = [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]];
    pub fn neighbors_8<P: IntN<2>>(&self, point: P) -> impl Iterator<Item = [usize; 2]> {
        self.neighbors_custom(point, &Self::D8)
    }
    /// `(D8のインデックス, 隣接座標)` を返す。`Grid::D8[idx]` で方向ベクトルを参照できる。
    pub fn neighbors_8_indexed<P: IntN<2>>(
        &self,
        point: P,
    ) -> impl Iterator<Item = (usize, [usize; 2])> {
        self.neighbors_custom_indexed(point, &Self::D8)
    }
}

impl<P: IntN<2>, T> Index<P> for Grid<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: P) -> &Self::Output {
        let [x, y] = index.to_index();
        assert!(x < self.size[0] && y < self.size[1]);
        &self.inner[x * self.size[1] + y]
    }
}

impl<P: IntN<2>, T> IndexMut<P> for Grid<T> {
    #[inline]
    fn index_mut(&mut self, index: P) -> &mut Self::Output {
        let [x, y] = index.to_index();
        assert!(x < self.size[0] && y < self.size[1]);
        &mut self.inner[x * self.size[1] + y]
    }
}

impl<T> TryFrom<Vec<Vec<T>>> for Grid<T> {
    type Error = ();

    fn try_from(value: Vec<Vec<T>>) -> Result<Self, Self::Error> {
        let h = value.len();
        let w = value.first().map_or(0, |r| r.len());
        if !value.iter().all(|row| row.len() == w) {
            return Err(());
        }
        let mut inner = Vec::with_capacity(h * w);
        for row in value {
            inner.extend(row);
        }
        Ok(Grid { inner, size: [h, w] })
    }
}

impl<T: Display> Display for Grid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [h, w] = self.size;
        for i in 0..h {
            for j in 0..w {
                write!(f, "{}", &self.inner[i * w + j])?;
                if j < w - 1 {
                    write!(f, " ")?;
                }
            }
            if i < h - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
