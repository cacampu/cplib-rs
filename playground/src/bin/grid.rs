use std::collections::VecDeque;

use cplib_graph_grid::Grid;
use cplib_lang_io::marker::Usize1;
use cplib_lang_io::{INPUT, fastout, input, outln};

#[fastout]
fn main() {
    input! {
        h: usize, w: usize,
        start: (Usize1, Usize1),
        goal: (Usize1, Usize1),
    }
    let map = Grid::read_bytes(h, w, &INPUT);
    let mut distance = Grid::new(h, w, usize::MAX);
    distance[start] = 0;
    let mut queue = VecDeque::from([start]);
    while let Some(p) = queue.pop_front() {
        for q in distance.neighbors(p) {
            if map[q] != b'#' && distance[q] == usize::MAX {
                distance[q] = distance[p] + 1;
                queue.push_back((q[0], q[1]));
            }
        }
    }
    outln!("{}", distance[goal]);
}
