pub trait Bisect: Clone {
    fn middle_point(&self, rhs: &Self) -> Option<Self>;
}

macro_rules! impl_bisect {
    ($($t:ty),*) => {
        $(
            impl Bisect for $t{
                fn middle_point(&self, rhs: &Self) -> Option<Self>{
                    if self.abs_diff(*rhs) > 1{
                        Some(self.midpoint(*rhs))
                    } else {
                        None
                    }
                }
            }
        )*
    };
}

impl_bisect!(
    usize, isize, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128
);

pub fn binary_search<T: Bisect>(mut ok: T, mut ng: T, mut checker: impl FnMut(T) -> bool) -> T {
    while let Some(m) = ok.middle_point(&ng) {
        if checker(m.clone()) {
            ok = m;
        } else {
            ng = m;
        }
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_binary_search() {
        assert_eq!(binary_search(0, 100, |i| i * i < 100), 9);
    }
}
