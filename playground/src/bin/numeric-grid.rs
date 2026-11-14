use cplib_graph_grid::Grid;
use cplib_lang_io::{INPUT, fastout, input, outln};

#[fastout]
fn main() {
    solve();
}

fn solve() {
    input! {
        n: usize, m: usize,
        required: [i64; m],
    }
    let nutrients = read_nutrients(n, m);
    let mut total = vec![0; m];
    for [row, col] in nutrients.all_pos() {
        total[col] += nutrients[[row, col]];
    }
    outln!(
        "{}",
        cplib_lang_io::yes_no((0..m).all(|i| total[i] >= required[i]))
    );
}

fn read_nutrients(h: usize, w: usize) -> Grid<i64> {
    // 入力元を引数に渡し回さず、同じ pub static バッファから続きを読む。
    Grid::read(h, w, &INPUT)
}
