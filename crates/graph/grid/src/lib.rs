use std::{
    fmt::Display,
    ops::{Index, IndexMut, Range},
};

use dir::Dir;
use prim::IntN;

fn zip_with<T, U, S, const N: usize>(a: [T; N], b: [U; N], f: impl Fn(T, U) -> S) -> [S; N] {
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
    coord
        .into_iter()
        .zip(size)
        .all(|(c, s)| c >= 0 && c < s as isize)
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
        in_bounds(point.to_isizes(), self.size)
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
        let p = point.to_isizes();
        let size = self.size;
        d.iter().filter_map(move |&dir| {
            let nb = zip_with(p, dir.to_isizes(), |pi, di| pi + di);
            in_bounds(nb, size).then(|| nb.to_usizes())
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
        let p = point.to_isizes();
        let size = self.size;
        d.iter().enumerate().filter_map(move |(idx, &dir)| {
            let nb = zip_with(p, dir.to_isizes(), |pi, di| pi + di);
            in_bounds(nb, size).then(|| (idx, nb.to_usizes()))
        })
    }

    /// 点 `point` から方向 `d` に1歩移動した座標を返す。範囲外なら `None`。
    pub fn step(&self, point: impl IntN<2>, d: impl IntN<2>) -> Option<[usize; 2]> {
        let nb = zip_with(point.to_isizes(), d.to_isizes(), |pi, di| pi + di);
        in_bounds(nb, self.size).then(|| nb.to_usizes())
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

    /// `(方向, 隣接座標)` を返す。
    pub fn neighbors_dir<P: IntN<2>>(&self, point: P) -> impl Iterator<Item = (Dir, [usize; 2])> {
        let p = point.to_isizes();
        let size = self.size;
        Dir::ALL.into_iter().filter_map(move |d| {
            let nb = zip_with(p, d.to_isizes(), |pi, di| pi + di);
            in_bounds(nb, size).then(|| (d, nb.to_usizes()))
        })
    }

    /// 90度時計回りに回転した `size[1] x size[0]` のグリッドを返す。
    pub fn rotate_cw(&self) -> Self
    where
        T: Clone,
    {
        let [h, w] = self.size;
        Self {
            size: [w, h],
            inner: (0..w)
                .flat_map(|i| (0..h).map(move |j| self[[h - 1 - j, i]].clone()))
                .collect(),
        }
    }

    /// 90度反時計回りに回転した `size[1] x size[0]` のグリッドを返す。
    pub fn rotate_ccw(&self) -> Self
    where
        T: Clone,
    {
        let [h, w] = self.size;
        Self {
            size: [w, h],
            inner: (0..w)
                .flat_map(|i| (0..h).map(move |j| self[[j, w - 1 - i]].clone()))
                .collect(),
        }
    }

    /// `rows x cols` の部分グリッドを切り出す。
    pub fn sub_grid(&self, rows: Range<usize>, cols: Range<usize>) -> Self
    where
        T: Clone,
    {
        Self {
            size: [rows.len(), cols.len()],
            inner: rows
                .flat_map(|i| cols.clone().map(move |j| self[[i, j]].clone()))
                .collect(),
        }
    }

    /// 外周が `pad` だけの行・列を、なくなるまで取り除く (文字列の `trim` の2次元版)。
    ///
    /// ```
    /// use grid::Grid;
    /// let rows: Vec<Vec<char>> = ["....", ".#..", "...."]
    ///     .into_iter()
    ///     .map(|s| s.chars().collect())
    ///     .collect();
    /// let g: Grid<char> = rows.try_into().unwrap();
    /// let trimmed = g.trim(&'.');
    /// assert_eq!(trimmed.size, [1, 1]);
    /// assert_eq!(trimmed[[0usize, 0usize]], '#');
    /// ```
    pub fn trim(&self, pad: &T) -> Self
    where
        T: Clone + PartialEq,
    {
        let [h, w] = self.size;
        let (mut top, mut bottom) = (0, h);
        let (mut left, mut right) = (0, w);
        while top < bottom && (left..right).all(|j| self[[top, j]] == *pad) {
            top += 1;
        }
        while top < bottom && (left..right).all(|j| self[[bottom - 1, j]] == *pad) {
            bottom -= 1;
        }
        while left < right && (top..bottom).all(|i| self[[i, left]] == *pad) {
            left += 1;
        }
        while left < right && (top..bottom).all(|i| self[[i, right - 1]] == *pad) {
            right -= 1;
        }
        self.sub_grid(top..bottom, left..right)
    }
}

impl<P: IntN<2>, T> Index<P> for Grid<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: P) -> &Self::Output {
        let [x, y] = index.to_usizes();
        assert!(x < self.size[0] && y < self.size[1]);
        &self.inner[x * self.size[1] + y]
    }
}

impl<P: IntN<2>, T> IndexMut<P> for Grid<T> {
    #[inline]
    fn index_mut(&mut self, index: P) -> &mut Self::Output {
        let [x, y] = index.to_usizes();
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
        Ok(Grid {
            inner,
            size: [h, w],
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn from_rows(rows: &[&str]) -> Grid<char> {
        rows.iter()
            .map(|s| s.chars().collect())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    fn to_rows(g: &Grid<char>) -> Vec<String> {
        let [h, w] = g.size;
        (0..h)
            .map(|i| (0..w).map(|j| g[[i, j]]).collect())
            .collect()
    }

    #[test]
    fn rotate_cw_matches_expected() {
        let g = from_rows(&["012", "345"]);
        let rotated = g.rotate_cw();
        assert_eq!(rotated.size, [3, 2]);
        assert_eq!(to_rows(&rotated), vec!["30", "41", "52"]);
    }

    #[test]
    fn rotate_ccw_matches_expected() {
        let g = from_rows(&["012", "345"]);
        let rotated = g.rotate_ccw();
        assert_eq!(rotated.size, [3, 2]);
        assert_eq!(to_rows(&rotated), vec!["25", "14", "03"]);
    }

    #[test]
    fn rotate_cw_then_ccw_is_identity() {
        let g = from_rows(&["abc", "def", "ghi"]);
        assert_eq!(g.rotate_cw().rotate_ccw(), g);
        assert_eq!(g.rotate_ccw().rotate_cw(), g);
    }

    #[test]
    fn rotate_four_times_is_identity() {
        let g = from_rows(&["012", "345"]);
        let r = g.rotate_cw().rotate_cw().rotate_cw().rotate_cw();
        assert_eq!(r, g);
    }

    #[test]
    fn sub_grid_extracts_rectangle() {
        let g = from_rows(&["0123", "4567", "89ab"]);
        let sub = g.sub_grid(0..2, 1..3);
        assert_eq!(sub.size, [2, 2]);
        assert_eq!(to_rows(&sub), vec!["12", "56"]);
    }

    #[test]
    fn trim_removes_padding_border() {
        let g = from_rows(&["....", ".#..", "...."]);
        let trimmed = g.trim(&'.');
        assert_eq!(trimmed.size, [1, 1]);
        assert_eq!(to_rows(&trimmed), vec!["#"]);
    }

    #[test]
    fn trim_keeps_grid_without_padding() {
        let g = from_rows(&["ab", "cd"]);
        assert_eq!(g.trim(&'.'), g);
    }

    #[test]
    fn trim_all_padding_collapses_to_empty() {
        let g = from_rows(&["...", "..."]);
        let trimmed = g.trim(&'.');
        assert_eq!(trimmed.size, [0, 0]);
    }

    #[test]
    fn trim_handles_asymmetric_padding() {
        let g = from_rows(&["...", "..#", "..."]);
        let trimmed = g.trim(&'.');
        assert_eq!(trimmed.size, [1, 1]);
        assert_eq!(to_rows(&trimmed), vec!["#"]);
    }

    #[test]
    fn neighbors_dir_reports_direction_and_position() {
        let g = Grid::new([3, 3], 0);
        let found: Vec<_> = g.neighbors_dir([0usize, 0]).collect();
        assert_eq!(found.len(), 2);
        assert!(found.contains(&(Dir::D, [1, 0])));
        assert!(found.contains(&(Dir::R, [0, 1])));

        let center: Vec<_> = g.neighbors_dir([1usize, 1]).collect();
        assert_eq!(center.len(), 4);
        for d in Dir::ALL {
            let [dr, dc] = d.delta();
            assert!(center.contains(&(d, [(1 + dr) as usize, (1 + dc) as usize])));
        }
    }
}
