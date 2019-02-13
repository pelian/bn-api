/// Asserts whether two vectors are contain the same elements, ignoring sorting.
/// The types must implement PartialOrd
#[macro_export]
macro_rules! assert_equiv {
    ($left_vec:expr, $right_vec:expr) => {{
        let mut l = $left_vec.clone();
        l.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut r = $right_vec.clone();
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(l, r);
    }};
}

#[macro_export]
macro_rules! assert_nequiv {
    ($left_vec:expr, $right_vec:expr) => {{
        let mut l = $left_vec.clone();
        l.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut r = $right_vec.clone();
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_ne!(l, r);
    }};
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_assert_equivalent() {
        assert_equiv!(vec![1, 2, 3], vec![3, 2, 1]);
        assert_nequiv!(vec![1, 6, 3], vec![3, 2, 1]);
    }
}
