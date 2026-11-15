use macros::swap;

#[test]
fn swaps_owned_values_fields_and_collection_elements() {
    let (mut a, mut b) = (String::from("first"), String::from("second"));
    swap!(a, b);
    assert_eq!((a.as_str(), b.as_str()), ("second", "first"));
    let mut pair = (a, b);
    swap!(pair.0, pair.1,);
    assert_eq!(pair, (String::from("first"), String::from("second")));
    let mut values = vec![String::from("a"), String::from("b")];
    swap!(values; 0, 1);
    swap!(values; 0, 0,);
    assert_eq!(values, ["b", "a"]);
}

#[test]
fn evaluates_places_and_indices_once() {
    let (mut a, mut b) = ([1, 2], [3, 4]);
    let mut calls = 0;
    swap!(
        a[{
            calls += 1;
            0
        }],
        b[{
            calls += 1;
            1
        }]
    );
    assert_eq!((a, b, calls), ([4, 2], [3, 1], 2));
    swap!(a; { calls += 1; 0 }, { calls += 1; 1 });
    assert_eq!((a, calls), ([2, 4], 4));
}
