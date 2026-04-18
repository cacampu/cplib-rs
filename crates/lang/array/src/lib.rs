pub trait ArrayExt<T, const N: usize> {
    fn zip_with<U, S>(self, other: [U; N], f: impl Fn(T, U) -> S) -> [S; N];
    fn arr_min(self) -> T
    where
        T: Ord;
    fn arr_max(self) -> T
    where
        T: Ord;
}

impl<T, const N: usize> ArrayExt<T, N> for [T; N] {
    fn zip_with<U, S>(self, other: [U; N], f: impl Fn(T, U) -> S) -> [S; N] {
        let mut a = self.into_iter();
        let mut b = other.into_iter();
        std::array::from_fn(|_| f(a.next().unwrap(), b.next().unwrap()))
    }

    fn arr_min(self) -> T
    where
        T: Ord,
    {
        self.into_iter().min().unwrap()
    }

    fn arr_max(self) -> T
    where
        T: Ord,
    {
        self.into_iter().max().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_with() {
        assert_eq!([1, 2, 3].zip_with([4, 5, 6], |a, b| a + b), [5, 7, 9]);
    }

    #[test]
    fn arr_min_max() {
        assert_eq!([3, 1, 2].arr_min(), 1);
        assert_eq!([3, 1, 2].arr_max(), 3);
    }
}
