use std::io::Write;
use std::process::{Command, Stdio};

fn run_capture(program: &str, input: &str) -> (String, String) {
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
    (
        String::from_utf8(result.stdout).unwrap(),
        String::from_utf8(result.stderr).unwrap(),
    )
}

fn run(program: &str, input: &str) -> String {
    run_capture(program, input).0
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

#[test]
fn printable_output_and_debug_share_formats_without_moving_values() {
    let (stdout, stderr) = run_capture(env!("CARGO_BIN_EXE_output"), "");
    let evaluations = usize::from(cfg!(debug_assertions));
    assert_eq!(
        stdout,
        format!(
            "2 1\n1 -1 3\n3,-1,1\n3\n-1\n1\n-1\n9\nYes\nNo\nImpossible\n\
             42\ntrue\n1.25\n3:-1:1 / [1, 2]\n7\n4 5\n6 7\n8 8\n8 8\n\n\
             end ok\nevaluations={evaluations}\n"
        )
    );
    assert_eq!(
        stderr,
        if cfg!(debug_assertions) {
            "3 -1 1\n3;-1;1\nx=1.2\n3\n-1\n1\n99\n"
        } else {
            ""
        }
    );
}
