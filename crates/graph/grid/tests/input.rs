use grid::Grid;
use io::{LineSource, Scanner, input};

#[test]
fn numeric_grid_consumes_exactly_its_cells() {
    let mut source = Scanner::new("2 3\n1 2\n3 4 5\n6 99");
    let (h, w) = input!(from &mut source, (usize, usize));
    let g = Grid::<i64>::read(h, w, &mut source);
    assert_eq!(g.inner, [1, 2, 3, 4, 5, 6]);
    assert_eq!(g[[1, 2]], 6);
    assert_eq!(source.read::<usize>(), 99);
    assert_eq!(Grid::<i64>::read(0, 3, &mut source).size, [0, 3]);
}

#[test]
fn character_and_byte_grids_work_with_both_sources() {
    let mut source = LineSource::new(std::io::Cursor::new("あい\nうえ\n.#\n#. 42"));
    let chars = Grid::read_chars(2, 2, &mut source);
    assert_eq!(chars.inner, ['あ', 'い', 'う', 'え']);
    let bytes = Grid::read_bytes(2, 2, &mut source);
    assert_eq!(bytes.inner, b".##.");
    assert_eq!(source.read::<usize>(), 42);
    assert_eq!(Grid::read_chars(2, 0, &mut source).size, [2, 0]);
    assert_eq!(Grid::read_bytes(2, 0, &mut source).size, [2, 0]);
}

#[test]
#[should_panic(expected = "grid row width mismatch")]
fn wrong_row_width_is_rejected() {
    Grid::read_chars(1, 2, &mut Scanner::new("abc"));
}
