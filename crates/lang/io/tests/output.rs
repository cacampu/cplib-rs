use std::fmt;

use io::{Printable, display, lines, option_or, printable, separated};

fn text(value: impl Printable) -> String {
    format!("{}", printable(value))
}

#[test]
fn vectors_slices_arrays_tuples_and_options_compose() {
    let values = vec![Some(2usize), None, Some(5)];
    assert_eq!(text(&values), "2 -1 5");
    assert_eq!(text(values.as_slice()), "2 -1 5");
    assert_eq!(text([1, 2, 3]), "1 2 3");
    assert_eq!(text((7, None::<i32>, "ok")), "7 -1 ok");
    assert_eq!(text(Some(vec![1, 2])), "1 2");
    assert_eq!(text(None::<Vec<i32>>), "-1");
    assert_eq!(text(Vec::<i32>::new()), "");
    assert_eq!(text(()), "");
    assert_eq!(text(true), "true");
}

#[test]
fn separators_and_option_fallbacks_are_explicit() {
    let values = [Some(1), None, Some(3)];
    assert_eq!(text(separated(&values, ",")), "1,-1,3");
    assert_eq!(text(separated(&values, '・')), "1・-1・3");
    assert_eq!(text(lines(&values)), "1\n-1\n3");
    assert_eq!(text(separated(&['a', 'b', 'c'], "")), "abc");
    assert_eq!(text(separated(&[] as &[i32], ",")), "");
    assert_eq!(text(option_or(&None::<i32>, "Impossible")), "Impossible");
    assert_eq!(text(option_or(&Some(42), "Impossible")), "42");
    assert_eq!(io::yes_no(true), "Yes");
    assert_eq!(text(io::yes_no_upper(false)), "NO");
}

struct Point(i64, i64);
impl Printable for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}

struct Label(&'static str);
impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[test]
fn custom_printable_and_display_types_work_in_collections() {
    let points = [Point(1, 2), Point(3, 4)];
    assert_eq!(text(lines(&points)), "1 2\n3 4");
    assert_eq!(text([display(Label("a")), display(Label("b"))]), "a b");
    assert_eq!(format!("{:.2}", printable(1.234)), "1.23");
    assert_eq!(io::line(&[Some(Point(1, 2)), None]), "1 2 -1");
}

#[test]
fn writer_helpers_use_the_same_printable_rules() {
    let mut writer = Vec::new();
    io::write_line(&mut writer, [Some(1), None, Some(3)]).unwrap();
    io::write_joined(&mut writer, [Some(4), None], ",").unwrap();
    writer.push(b'\n');
    io::write_matrix(&mut writer, [vec![Some(5), None], vec![Some(6), Some(7)]]).unwrap();
    assert_eq!(
        String::from_utf8(writer).unwrap(),
        "1 -1 3\n4,-1\n5 -1\n6 7"
    );
    assert_eq!(
        io::matrix([vec![Some(1), None], vec![Some(2), Some(3)]]),
        "1 -1\n2 3"
    );
}
