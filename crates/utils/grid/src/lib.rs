use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid<T> {
    pub h: usize,
    pub w: usize,
    pub inner: Vec<T>,
}

pub trait Point {
    fn xy(&self) -> [usize; 2];
}
macro_rules! impl_point_for_tuple_and_array {
    ($($t:ty),*) => {
        $(
            impl Point for ($t,$t){
                #[inline]
                fn xy(&self) -> [usize; 2] {
                    [self.0 as usize, self.1 as usize]
                }
            }
            impl Point for [$t; 2]{
                #[inline]
                fn xy(&self) -> [usize; 2] {
                    self.map(|x| x as usize)
                }
            }

        )*
    };
}
impl_point_for_tuple_and_array!(
    usize, isize, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128
);

impl<T> Grid<T> {
    pub fn new(h: usize, w: usize, default: T) -> Self
    where
        T: Clone,
    {
        Self {
            h,
            w,
            inner: vec![default; h * w],
        }
    }
    #[inline]
    pub fn in_bounds(&self, point: impl Point) -> bool {
        let [x, y] = point.xy();
        x < self.h && y < self.w
    }
    pub fn neighbors_custom<'a, P: Point>(
        &'a self,
        point: P,
        d: &'a [[isize; 2]],
    ) -> impl Iterator<Item = [usize; 2]> + 'a {
        let [x, y] = point.xy();
        d.into_iter()
            .map(move |&[dx, dy]| [x.wrapping_add_signed(dx), y.wrapping_add_signed(dy)])
            .filter(|p| self.in_bounds(*p))
    }
    pub fn neighbors<'a, P: Point>(&'a self, point: P) -> impl Iterator<Item = [usize; 2]> + 'a {
        static D: [[isize; 2]; 4] = [[1, 0], [0, 1], [-1, 0], [0, -1]];
        self.neighbors_custom(point, &D)
    }
    pub fn neighbors_8<'a, P: Point>(&'a self, point: P) -> impl Iterator<Item = [usize; 2]> + 'a {
        #[rustfmt::skip]
        static D: [[isize; 2]; 8] = [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]];
        self.neighbors_custom(point, &D)
    }
}

impl<P: Point, T> Index<P> for Grid<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: P) -> &Self::Output {
        let [x, y] = index.xy();
        assert!(x < self.h && y < self.w);
        &self.inner[x * self.w + y]
    }
}

impl<P: Point, T> IndexMut<P> for Grid<T> {
    #[inline]
    fn index_mut(&mut self, index: P) -> &mut Self::Output {
        let [x, y] = index.xy();
        assert!(x < self.h && y < self.w);
        &mut self.inner[x * self.w + y]
    }
}

impl<T> TryFrom<Vec<Vec<T>>> for Grid<T> {
    type Error = ();

    fn try_from(value: Vec<Vec<T>>) -> Result<Self, Self::Error> {
        let h = value.len();
        let w = value[0].len();
        if !value.iter().all(|row| row.len() == w) {
            return Err(());
        }
        let mut inner = Vec::with_capacity(h * w);
        for row in value {
            inner.extend(row);
        }
        Ok(Grid { inner, h, w })
    }
}

impl<T: Display> Display for Grid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.h {
            for j in 0..self.w {
                let idx = i * self.h + j;
                write!(f, "{}", &self.inner[idx])?;
                if j <= self.w - 1 {
                    write!(f, " ")?;
                }
            }
            if i <= self.h - 1 {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}
