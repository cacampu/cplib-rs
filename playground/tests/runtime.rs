use std::io::Write;
use std::process::{Command, Stdio};

fn run(program: &str, input: &str) -> String {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

#[test]
fn global_input_expression_and_flush_on_return() {
    assert_eq!(
        run(env!("CARGO_BIN_EXE_cplib-playground"), "3 10 20 30"),
        "60\n"
    );
}

#[test]
fn testcase_count_can_share_a_line_and_early_return_is_per_case() {
    assert_eq!(
        run(env!("CARGO_BIN_EXE_multi-testcase"), "3 2 ab 2 ba 4 aaaa"),
        "Yes\nNo\nYes\n"
    );
    assert_eq!(run(env!("CARGO_BIN_EXE_multi-testcase"), "0"), "");
}

#[test]
fn all_output_paths_share_one_buffer_in_order() {
    assert_eq!(
        run(env!("CARGO_BIN_EXE_io-runtime"), ""),
        "main helper closure\nthread\n1 2 3\n"
    );
}

#[test]
fn global_grid_input_can_cross_functions_and_testcases() {
    assert_eq!(
        run(
            env!("CARGO_BIN_EXE_shared-input"),
            "2 1 2 10 20 3 2 1 4 5 6"
        ),
        "33\n15\n"
    );
}

#[test]
fn closure_macros_compose_with_io_and_binary_search() {
    assert_eq!(
        run(env!("CARGO_BIN_EXE_closure-macros"), ""),
        "6\n6\n105\n7\n10\n2\n"
    );
}
