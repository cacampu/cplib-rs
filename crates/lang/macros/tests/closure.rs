#![forbid(unsafe_code)]

use macros::{chmax, chmin, lazyfn, lazymac, recurfn};

#[test]
fn recursive_closure_supports_captures_generics_and_moves() {
    fn apply<T: Copy + std::ops::Add<Output = T>>(zero: T, values: &[T]) -> T {
        recurfn!(
            sum = |n: usize| -> T {
                if n == 0 {
                    zero
                } else {
                    values[n - 1] + sum(n - 1)
                }
            }
        );
        let moved = sum;
        moved(values.len())
    }
    assert_eq!(apply(0, &[1, 2, 3, 4]), 10);
    assert_eq!(apply(0.0, &[1.5, 2.5]), 4.0);

    fn make_factorial() -> impl Fn(u64) -> u64 {
        recurfn!(factorial = |n: u64| -> u64 { if n == 0 { 1 } else { n * factorial(n - 1) } });
        factorial
    }
    assert_eq!(make_factorial()(5), 120);

    let calls = std::cell::Cell::new(0);
    recurfn!(
        again = || {
            calls.set(calls.get() + 1);
            if calls.get() < 4 {
                again();
            }
        }
    );
    again();
    assert_eq!(calls.get(), 4);
}

#[test]
fn recursive_mutable_state_is_reborrowed() {
    let g = [vec![1, 4], vec![2, 3], vec![], vec![], vec![5], vec![]];
    recurfn!(
        dfs = |dist: &mut [usize], u: usize| {
            for &v in &g[u] {
                if chmin!(dist[v], dist[u] + 1) {
                    dfs(dist, v);
                }
            }
        }
    );
    let mut distance = vec![usize::MAX; 6];
    distance[0] = 0;
    dfs(&mut distance, 0);
    assert_eq!(distance, [0, 1, 2, 2, 1, 2]);

    recurfn!(
        fill = |values: &mut Vec<String>| {
            if values.len() == 4 {
                return;
            }
            values.push(values.len().to_string());
            fill(values);
        }
    );
    let mut values = Vec::new();
    fill(&mut values);
    assert_eq!(values, ["0", "1", "2", "3"]);
}

#[test]
fn lazy_recursive_calls_allow_updates_between_invocations() {
    let g = [
        vec![1, 4],
        vec![0, 2, 3],
        vec![1],
        vec![1],
        vec![0, 5],
        vec![4],
    ];
    let mut weights = [1i64; 6];
    let mut sub = [0i64; 6];
    lazyfn!(recur tree = |sub: &mut [i64], u: usize, p: usize| -> i64 {
        let mut sum = weights[u];
        for &v in &g[u] {
            if v != p { sum += tree(sub, v, u); }
        }
        sub[u] = sum;
        sum
    });
    assert_eq!(tree!(&mut sub, 0, usize::MAX), 6);
    weights[3] = 100;
    assert_eq!(tree!(&mut sub, 0, usize::MAX,), 105);
    assert_eq!(sub, [105, 102, 1, 100, 2, 1]);

    fn twice(mut f: impl FnMut(&mut [i64], usize, usize) -> i64, sub: &mut [i64]) -> i64 {
        f(sub, 0, usize::MAX) + f(sub, 1, usize::MAX)
    }
    assert_eq!(twice(tree!(fn), &mut sub), 210);
    weights[0] = 0;
    assert_eq!(tree!(&mut sub, 0, usize::MAX), 104);

    let multiplier = 3;
    lazyfn!(recur factorial = |n: u64| -> u64 {
        if n == 0 { multiplier } else { n * factorial(n - 1) }
    });
    assert_eq!(factorial!(5), 360);
    let callable = factorial!(fn);
    assert_eq!(callable(3), 18);

    let visits = std::cell::Cell::new(0);
    lazyfn!(recur repeat = || {
        visits.set(visits.get() + 1);
        if visits.get() < 3 { repeat(); }
    });
    repeat!();
    assert_eq!(visits.get(), 3);
}

#[test]
fn lazy_non_recursive_callbacks_support_mutable_captures() {
    fn bsearch(mut ok: i64, mut ng: i64, mut check: impl FnMut(i64) -> bool) -> i64 {
        while (ok - ng).abs() > 1 {
            let middle = (ok + ng) / 2;
            if check(middle) {
                ok = middle;
            } else {
                ng = middle;
            }
        }
        ok
    }
    let mut limit = 1_000_000;
    let mut calls = 0;
    lazyfn!(
        check = |m: i64| -> bool {
            calls += 1;
            m * m <= limit
        }
    );
    assert_eq!(bsearch(0, 2_000_000, check!(fn)), 1000);
    limit = 50;
    assert_eq!(bsearch(0, 2_000_000, check!(fn)), 7);
    assert!(calls > 0);
    assert!(check!(7));
    assert!(!check!(8));

    lazyfn!(
        increment = || {
            calls += 1;
        }
    );
    let before = calls;
    increment!();
    assert_eq!(calls, before + 1);
}

#[test]
fn lazy_calls_evaluate_arguments_once_before_binding_and_borrowing() {
    let x = 100usize;
    lazyfn!(sum = |x: usize, y: usize| -> usize { x + y });
    lazymac!(also_sum = |x, y| x + y);
    assert_eq!(sum!(1, x), 101);
    assert_eq!(also_sum!(1, x,), 101);

    let mut log = Vec::new();
    lazyfn!(
        append = |a: usize, b: usize| {
            log.push(a + b);
        }
    );
    append!(
        {
            log.push(1);
            10
        },
        {
            log.push(2);
            20
        }
    );
    assert_eq!(log, [1, 2, 30]);
    lazymac!(
        append_again = |a: usize, b| {
            log.push(a + b);
        }
    );
    append_again!(
        {
            log.push(3);
            30
        },
        {
            log.push(4);
            40
        }
    );
    assert_eq!(log, [1, 2, 30, 3, 4, 70]);
}

#[test]
fn local_macros_can_share_mutable_state_and_infer_types() {
    let xs = [3usize, 1, 3, 2, 1, 3];
    let mut count = [0; 4];
    let mut kinds = 0;
    lazymac!(
        add = |i: usize| {
            if count[xs[i]] == 0 {
                kinds += 1;
            }
            count[xs[i]] += 1;
        }
    );
    lazymac!(
        remove = |i| {
            count[xs[i]] -= 1;
            if count[xs[i]] == 0 {
                kinds -= 1;
            }
        }
    );
    lazymac!(key = |l, r| (r - l) * 100 + kinds);
    for i in 0..4 {
        add!(i);
    }
    assert_eq!(key!(0, 4), 403);
    remove!(0);
    add!(4);
    add!(5);
    assert_eq!(kinds, 3);
}

#[test]
fn lazy_functions_and_inline_macros_have_distinct_control_flow() {
    fn function_return() -> i32 {
        lazyfn!(
            local = |n: i32| -> i32 {
                if n == 0 {
                    return 7;
                }
                n
            }
        );
        local!(0) + 1
    }
    fn caller_return() -> i32 {
        lazymac!(
            local = |n| {
                if n == 0 {
                    return 7;
                }
                n
            }
        );
        local!(0) + 1
    }
    assert_eq!(function_return(), 8);
    assert_eq!(caller_return(), 7);

    let mut sum = 0;
    lazymac!(
        skip = || {
            continue;
        }
    );
    for i in 0..4 {
        if i == 2 {
            skip!();
        }
        sum += i;
    }
    assert_eq!(sum, 4);
}

#[test]
fn update_macros_evaluate_each_expression_once_and_allow_shared_container() {
    let mut values = [9, 2];
    let mut indices = 0;
    let mut candidates = 0;
    assert!(chmin!(
        values[{
            indices += 1;
            0
        }],
        {
            candidates += 1;
            values[1]
        }
    ));
    assert_eq!((values, indices, candidates), ([2, 2], 1, 1));
    assert!(!chmin!(values[0], {
        candidates += 1;
        4
    }));
    assert_eq!(candidates, 2);
    assert!(chmax!(values[1], 5,));
    let mut text = String::from("z");
    assert!(chmin!(text, String::from("a")));
    assert_eq!(text, "a");
}

#[test]
fn multidimensional_array_only_needs_the_public_macro_import() {
    let values = macros::mat![0; 2, 3, 4];
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].len(), 3);
    assert_eq!(values[0][0], [0; 4]);
}
