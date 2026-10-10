//! グリッド探索で使う4方向。`[T; 2]` / `(T, T)` / `vector::Vector<T, 2>` と
//! 加算できる (結果は常に `isize` 側、座標の正当性は呼び出し側で確認する)。

use std::ops::Add;

use prim::{Int, IntN};

/// 上下左右。`delta()` は `[Δ行, Δ列]` (下 = 行が増える方向)。
///
/// ```
/// use dir::Dir;
/// let pos = [3usize, 3];
/// assert_eq!(pos + Dir::U, [2, 3]);
/// assert_eq!(Dir::U.rotate_cw(), Dir::R);
/// assert_eq!(Dir::U.opposite(), Dir::D);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    U,
    D,
    L,
    R,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::U, Dir::D, Dir::L, Dir::R];

    #[inline]
    pub fn delta(self) -> [isize; 2] {
        match self {
            Dir::U => [-1, 0],
            Dir::D => [1, 0],
            Dir::L => [0, -1],
            Dir::R => [0, 1],
        }
    }

    /// 時計回りに90度: `U -> R -> D -> L -> U`。
    pub fn rotate_cw(self) -> Self {
        match self {
            Dir::U => Dir::R,
            Dir::R => Dir::D,
            Dir::D => Dir::L,
            Dir::L => Dir::U,
        }
    }

    /// 反時計回りに90度。
    pub fn rotate_ccw(self) -> Self {
        match self {
            Dir::U => Dir::L,
            Dir::L => Dir::D,
            Dir::D => Dir::R,
            Dir::R => Dir::U,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Dir::U => Dir::D,
            Dir::D => Dir::U,
            Dir::L => Dir::R,
            Dir::R => Dir::L,
        }
    }
}

impl IntN<2> for Dir {
    #[inline]
    fn to_isizes(self) -> [isize; 2] {
        self.delta()
    }
}

impl<T: Int> Add<Dir> for [T; 2] {
    type Output = [isize; 2];
    fn add(self, rhs: Dir) -> [isize; 2] {
        let [dr, dc] = rhs.delta();
        [self[0].to_isize() + dr, self[1].to_isize() + dc]
    }
}

impl<T: Int> Add<Dir> for (T, T) {
    type Output = (isize, isize);
    fn add(self, rhs: Dir) -> (isize, isize) {
        let [dr, dc] = rhs.delta();
        (self.0.to_isize() + dr, self.1.to_isize() + dc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_cycles_through_all_four() {
        let mut d = Dir::U;
        for expected in [Dir::R, Dir::D, Dir::L, Dir::U] {
            d = d.rotate_cw();
            assert_eq!(d, expected);
        }
        for expected in [Dir::L, Dir::D, Dir::R, Dir::U] {
            d = d.rotate_ccw();
            assert_eq!(d, expected);
        }
    }

    #[test]
    fn rotate_cw_and_ccw_are_inverses() {
        for d in Dir::ALL {
            assert_eq!(d.rotate_cw().rotate_ccw(), d);
            assert_eq!(d.rotate_ccw().rotate_cw(), d);
        }
    }

    #[test]
    fn opposite_is_involutive() {
        for d in Dir::ALL {
            assert_eq!(d.opposite().opposite(), d);
            assert_ne!(d.opposite(), d);
        }
    }

    #[test]
    fn add_matches_delta() {
        let p = [5i64, 5i64];
        for d in Dir::ALL {
            let [dr, dc] = d.delta();
            let expected = [p[0] as isize + dr, p[1] as isize + dc];
            assert_eq!(p + d, expected);
            assert_eq!((p[0], p[1]) + d, (expected[0], expected[1]));
        }
    }

    #[test]
    fn add_on_usize_point() {
        let p = [0usize, 0usize];
        assert_eq!(p + Dir::U, [-1, 0]);
        assert_eq!(p + Dir::D, [1, 0]);
    }
}
