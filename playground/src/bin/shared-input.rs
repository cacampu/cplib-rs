//! 複数ケース・solve・ヘルパー関数から同じ入力バッファを使う例。
use cplib_graph_grid::Grid;
use cplib_lang_io::{INPUT, input, multi_testcase, outln};

#[multi_testcase]
fn main() {
    solve();
}

fn solve() {
    let (h, w) = input!((usize, usize));
    let grid = read_grid(h, w);
    input! { from &INPUT, extra: i64 }
    outln!("{}", grid.inner.iter().sum::<i64>() + extra);
}

fn read_grid(h: usize, w: usize) -> Grid<i64> {
    Grid::read(h, w, &INPUT)
}
