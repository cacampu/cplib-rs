use cplib_graph_grid::Grid;
use cplib_lang_io::{fastout, input, outln};

#[fastout]
fn main() {
    let mut source = cplib_lang_io::stdin();
    input! { from &mut source,
        n: usize, m: usize,
        required: [i64; m],
    }
    let nutrients = Grid::<i64>::read(n, m, &mut source);
    let mut total = vec![0; m];
    for [row, col] in nutrients.all_pos() {
        total[col] += nutrients[[row, col]];
    }
    outln!(
        "{}",
        cplib_lang_io::yes_no((0..m).all(|i| total[i] >= required[i]))
    );
}
