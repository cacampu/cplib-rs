use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

fn run_capture(program: &str, input: &str, local: bool, debug: bool) -> (String, String) {
    let mut child = Command::new(program)
        .env("CP_LOCAL", if local { "1" } else { "0" })
        .env("CPLIB_DEBUG", if debug { "1" } else { "0" })
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
    run_capture(program, input, false, false).0
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
    assert_eq!(
        run(
            env!("CARGO_BIN_EXE_multi-testcase"),
            "3\n2 ab\n2 ba\n4 aaaa\n"
        ),
        "Yes\nNo\nYes\n"
    );
    assert_eq!(run(env!("CARGO_BIN_EXE_multi-testcase"), "0"), "");
}

#[test]
fn nested_run_and_threads_share_one_output_buffer() {
    assert_eq!(
        run(env!("CARGO_BIN_EXE_io-runtime"), ""),
        "main helper closure\nthread\n1 2 3\n"
    );
}

#[test]
fn interactive_input_flushes_queries_before_waiting_for_responses() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_interactive-io"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in lines {
            if send.send(line).is_err() {
                break;
            }
        }
    });

    for (reply, expected) in [("7\n", "? 7"), ("\n5\n", "? 12"), ("8\n", "! 20")] {
        input.write_all(reply.as_bytes()).unwrap();
        let actual = receive.recv_timeout(Duration::from_secs(5));
        if !matches!(&actual, Ok(Ok(line)) if line == expected) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("expected {expected:?}, got {actual:?}");
        }
    }

    drop(input);
    assert!(child.wait().unwrap().success());
    reader.join().unwrap();
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
        "6\n6\n105\n7\n10\n2\n6 0 1 2 3 4 5\n"
    );
}

#[test]
fn printable_output_and_debug_share_formats_without_moving_values() {
    for (local, debug) in [(false, false), (true, false), (false, true)] {
        let (stdout, stderr) = run_capture(env!("CARGO_BIN_EXE_output"), "", local, debug);
        let debug_enabled = local || debug;
        let evaluations = usize::from(debug_enabled);
        assert_eq!(
            stdout,
            format!(
                "2 1\n1 -1 3\n3,-1,1\n3\n-1\n1\n-1\n9\nYes\nNo\nImpossible\n\
                 42\ntrue\n1.25\n3:-1:1 / [1, 2]\n7\n4 5\n6 7\n8 8\n8 8\n\n\
                 end ok\nevaluations={evaluations}\n"
            )
        );
        let mut expected_stderr = String::new();
        if debug_enabled {
            expected_stderr.push_str("3 -1 1\n3;-1;1\nx=1.2\n[Left, Right]\n3\n-1\n1\n99\n");
        }
        if local {
            expected_stderr.push_str("local check\n");
        }
        assert_eq!(stderr, expected_stderr);
    }
}
