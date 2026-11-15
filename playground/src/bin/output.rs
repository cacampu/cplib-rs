//! Printable・区切り指定・デバッグ出力と swap! の例。
use cplib_graph_grid::Grid;
use cplib_lang_io::{debug, display, fastout, lines, option_or, out, outln, separated, yes_no};
use cplib_lang_macros::swap;
use cplib_math_modint::ModInt998244353 as Mi;
use cplib_math_vector::Vector;

#[fastout]
fn main() {
    let (mut a, mut b) = (1, 2);
    swap!(a, b);
    outln!(a, b);

    let mut values = vec![Some(1), None, Some(3)];
    outln!(values);
    swap!(values; 0, 2);
    outln!(separated(&values, ","));
    outln!(lines(&values));
    outln!(None::<usize>);
    outln!(Some(9));
    outln!(yes_no(true));
    cplib_lang_io::print_yes_no(false);
    outln!(option_or(&None::<usize>, "Impossible"));

    outln!(42);
    outln!(true);
    let x = 1.25;
    outln!("{x:.2}");
    outln!("{} / {:?}", separated(&values, ":"), [1, 2]);
    outln!(display(std::num::NonZeroUsize::new(7).unwrap()));
    outln!(Vector::<_, 2>::new(4, 5));
    outln!(vec![Mi::new(6), Mi::new(7)]);
    outln!(Grid::new(2, 2, Some(8)));
    out!();
    outln!();
    out!("end ");
    outln!("{}", "ok");

    debug!(values);
    debug!(separated(&values, ";"));
    debug!("x={x:.1}");
    cplib_lang_io::debug(lines(&values));
    let mut evaluations = 0;
    debug!({
        evaluations += 1;
        Some(99)
    });
    outln!("evaluations={evaluations}");
}
