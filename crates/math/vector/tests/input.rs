use io::{Scanner, input};
use vector::Vector;

#[test]
fn vectors_read_multiple_tokens_through_an_alias() {
    type P = Vector<i64, 2>;
    let mut source = Scanner::new("3\n1 2\n-3 4\n5 6 7");
    input! { from &mut source, n: usize, a: [P; n] }
    assert_eq!(a, vec![P::new(1, 2), P::new(-3, 4), P::new(5, 6)]);
    assert_eq!(input!(from &mut source, usize), 7);
    assert_eq!("1 2".parse::<P>().unwrap(), a[0]);
}
