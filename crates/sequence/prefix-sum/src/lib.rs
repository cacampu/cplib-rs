use std::ops::Add;

pub trait CumulativeSum<T: Add<T, Output = T> + Clone + Default> {
    fn cumsum(&self) -> Vec<T>;
}

impl<T> CumulativeSum<T> for [T]
where
    T: Add<T, Output = T> + Clone + Default,
{
    fn cumsum(&self) -> Vec<T> {
        let mut sum = T::default();
        let mut ret = Vec::with_capacity(self.len() + 1);
        ret.push(sum.clone());
        for item in self {
            sum = sum + item.clone();
            ret.push(sum.clone())
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cumsum() {
        let a = vec![1, 2, 3, 4, 5];
        let s = a.cumsum();
        assert_eq!(s, vec![0, 1, 3, 6, 10, 15]);
    }
}
