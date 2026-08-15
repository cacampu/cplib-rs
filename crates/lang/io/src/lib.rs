use std::fmt::{self, Display};
use std::io::{self, Write};

pub fn yes_no(ok: bool) -> &'static str {
    if ok { "Yes" } else { "No" }
}

pub fn yes_no_upper(ok: bool) -> &'static str {
    if ok { "YES" } else { "NO" }
}

pub fn possible_impossible(ok: bool) -> &'static str {
    if ok { "Possible" } else { "Impossible" }
}

pub fn line<I, T>(iter: I) -> String
where
    I: IntoIterator<Item = T>,
    T: Display,
{
    joined(iter, " ")
}

pub fn joined<I, T>(iter: I, sep: &str) -> String
where
    I: IntoIterator<Item = T>,
    T: Display,
{
    let mut buf = String::new();
    write_joined_fmt(&mut buf, iter, sep).expect("writing to String cannot fail");
    buf
}

pub fn matrix<I, J, T>(rows: I) -> String
where
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = T>,
    T: Display,
{
    let mut buf = String::new();
    write_matrix_fmt(&mut buf, rows).expect("writing to String cannot fail");
    buf
}

pub fn chars(chars: &[char]) -> String {
    chars.iter().collect()
}

pub fn write_yes_no<W>(writer: &mut W, ok: bool) -> io::Result<()>
where
    W: Write,
{
    writeln!(writer, "{}", yes_no(ok))
}

pub fn write_line<W, I, T>(writer: &mut W, iter: I) -> io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = T>,
    T: Display,
{
    write_joined(writer, iter, " ")?;
    writeln!(writer)
}

pub fn write_joined<W, I, T>(writer: &mut W, iter: I, sep: &str) -> io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = T>,
    T: Display,
{
    let mut first = true;
    for value in iter {
        if !first {
            write!(writer, "{sep}")?;
        }
        first = false;
        write!(writer, "{value}")?;
    }
    Ok(())
}

pub fn write_matrix<W, I, J, T>(writer: &mut W, rows: I) -> io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = T>,
    T: Display,
{
    let mut first_row = true;
    for row in rows {
        if !first_row {
            writeln!(writer)?;
        }
        first_row = false;
        write_joined(writer, row, " ")?;
    }
    Ok(())
}

pub fn debug<T>(value: T)
where
    T: Display,
{
    if cfg!(debug_assertions) {
        eprintln!("{value}");
    }
}

pub fn debug_line<I, T>(iter: I)
where
    I: IntoIterator<Item = T>,
    T: Display,
{
    if cfg!(debug_assertions) {
        eprintln!("{}", line(iter));
    }
}

pub fn debug_matrix<I, J, T>(rows: I)
where
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = T>,
    T: Display,
{
    if cfg!(debug_assertions) {
        eprintln!("{}", matrix(rows));
    }
}

fn write_joined_fmt<W, I, T>(writer: &mut W, iter: I, sep: &str) -> fmt::Result
where
    W: fmt::Write,
    I: IntoIterator<Item = T>,
    T: Display,
{
    let mut first = true;
    for value in iter {
        if !first {
            writer.write_str(sep)?;
        }
        first = false;
        write!(writer, "{value}")?;
    }
    Ok(())
}

fn write_matrix_fmt<W, I, J, T>(writer: &mut W, rows: I) -> fmt::Result
where
    W: fmt::Write,
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = T>,
    T: Display,
{
    let mut first_row = true;
    for row in rows {
        if !first_row {
            writer.write_char('\n')?;
        }
        first_row = false;
        write_joined_fmt(writer, row, " ")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yes_no_family() {
        assert_eq!(yes_no(true), "Yes");
        assert_eq!(yes_no_upper(false), "NO");
        assert_eq!(possible_impossible(true), "Possible");
    }

    #[test]
    fn format_line_and_joined() {
        assert_eq!(line([1, 2, 3]), "1 2 3");
        assert_eq!(joined([1, 2, 3], ","), "1,2,3");
        assert_eq!(line(std::iter::empty::<i32>()), "");
    }

    #[test]
    fn format_matrix_and_chars() {
        assert_eq!(matrix([vec![1, 2], vec![3, 4]]), "1 2\n3 4");
        assert_eq!(chars(&['a', 'b', 'c']), "abc");
    }

    #[test]
    fn write_helpers() {
        let mut out = Vec::new();
        write_yes_no(&mut out, true).unwrap();
        write_line(&mut out, [1, 2, 3]).unwrap();
        write_matrix(&mut out, [vec![4, 5], vec![6, 7]]).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "Yes\n1 2 3\n4 5\n6 7");
    }
}
