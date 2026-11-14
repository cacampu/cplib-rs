use cplib_lang_io::{fastout, input, outln};
use cplib_math_vector::Vector;

type P = Vector<i64, 2>;

#[fastout]
fn main() {
    input! {
        n: usize,
        a: [P; n],
    }
    let mut squared = 0;
    for i in 0..n {
        for j in 0..i {
            let difference = a[i] - a[j];
            squared = squared.max(difference.dot(difference));
        }
    }
    outln!("{:.10}", (squared as f64).sqrt());
}
