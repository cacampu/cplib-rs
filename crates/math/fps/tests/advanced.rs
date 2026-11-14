use convolution::convolution_naive;
use fps::{
    Factorials, Fps, berlekamp_massey, bostan_mori, convolution_dp, interpolate, linear_recurrence,
    power_sums, sum_rationals,
};
use modint::ModInt998244353 as Mi;

type F = Fps<998244353>;

fn random() -> impl FnMut() -> Mi {
    let mut state = 88172645463325252u64;
    move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        Mi::new(state)
    }
}

fn naive_div(p: &F, q: &F, n: usize) -> F {
    let mut result = F::zeros(n);
    let inv = q.at(0).inv();
    for i in 0..n {
        let mut value = p.at(i);
        for j in 1..=i {
            value -= q.at(j) * result[i - j];
        }
        result[i] = value * inv;
    }
    result
}

#[test]
fn sparse_operations_match_dense_arithmetic() {
    let mut next = random();
    let terms = [
        (4, Mi::new(2)),
        (0, Mi::new(3)),
        (2, Mi::new(4)),
        (0, Mi::new(5)),
        (2, Mi::new(-1)),
        (usize::MAX, Mi::new(9)),
    ];
    let q = F::from_ints(&[8, 0, 3, 0, 2]);
    for n in [0, 1, 3, 61, 129] {
        let f = F::new((0..n + 5).map(|_| next()).collect());
        let expected_product = F::new(convolution_naive(f.coef(), q.coef())).pre(n);
        assert_eq!(f.mul_sparse(&terms, n), expected_product);
        assert_eq!(f.mul_truncated(&q, n), expected_product);
        assert_eq!(f.div_sparse(&terms, n), naive_div(&f, &q, n));
        assert_eq!(f.div_series(&q, n), naive_div(&f, &q, n));
        for degree in [0, 1, 2, n, usize::MAX] {
            let c = Mi::new(7);
            let mut g = f.pre(n);
            let original = g.clone();
            g.mul_binomial_assign(degree, c);
            assert_eq!(g, original.mul_sparse(&[(0, Mi::new(1)), (degree, c)], n));
            g.div_binomial_assign(degree, c);
            assert_eq!(g, original);
        }
    }
    assert_eq!(F::zeros(0).mul_truncated(&q, 4), F::zeros(4));
    assert_eq!(q.mul_sparse(&[], 4), F::zeros(4));
}

#[test]
#[should_panic(expected = "non-zero constant term")]
fn sparse_division_rejects_cancelled_constant() {
    F::zeros(4).div_sparse(&[(0, Mi::new(1)), (0, Mi::new(-1)), (2, Mi::new(1))], 4);
}

#[test]
fn product_matches_naive_with_unbalanced_degrees() {
    let mut next = random();
    let mut factors = Vec::new();
    let mut expected = vec![Mi::new(1)];
    for n in [1, 129, 2, 3, 67, 1, 4, 2] {
        let coef: Vec<_> = (0..n).map(|_| next()).collect();
        expected = convolution_naive(&expected, &coef);
        factors.push(F::new(coef));
    }
    assert_eq!(factors.clone().into_iter().product::<F>(), F::new(expected));
    factors.push(F::zeros(0));
    assert!(factors.into_iter().product::<F>().is_empty());
    assert_eq!(std::iter::empty::<F>().product::<F>(), F::from_ints(&[1]));
    assert_eq!(
        [F::from_ints(&[1, 0]), F::from_ints(&[1, 0])]
            .into_iter()
            .product::<F>(),
        F::from_ints(&[1, 0, 0])
    );
}

#[test]
fn power_sums_match_direct_powers() {
    let mut next = random();
    for m in [0, 1, 3, 129] {
        let mut values: Vec<_> = (0..m).map(|_| next()).collect();
        if m > 0 {
            values[0] = Mi::new(0);
        }
        for n in [0, 1, 2, 70, 129] {
            let mut expected = F::zeros(n);
            for &value in &values {
                let mut power = Mi::new(1);
                for c in expected.coef_mut() {
                    *c += power;
                    power *= value;
                }
            }
            assert_eq!(power_sums(&values, n), expected, "m={m}, n={n}");
        }
    }
}

#[test]
fn rational_sum_matches_individual_series() {
    let mut next = random();
    for count in [0, 1, 3, 129] {
        let mut terms = Vec::new();
        let mut expected = F::zeros(37);
        for i in 0..count {
            let p = F::new((0..i % 4 + 1).map(|_| next()).collect());
            let q = F::new(vec![Mi::new(1), next()]);
            expected += naive_div(&p, &q, 37);
            terms.push((p, q));
        }
        let (p, q) = sum_rationals(terms);
        assert_eq!(p.div_series(&q, 37), expected);
    }
}

#[test]
fn multipoint_evaluation_matches_horner_with_duplicates() {
    let mut next = random();
    for n in [0, 1, 4, 100, 400] {
        let f = F::new((0..n).map(|_| next()).collect());
        for m in [0, 1, 2, 5, 129] {
            let points: Vec<_> = (0..m)
                .map(|i| if i % 3 == 0 { Mi::new(0) } else { next() })
                .collect();
            let expected: Vec<_> = points.iter().map(|&x| f.eval(x)).collect();
            assert_eq!(f.multipoint_eval(&points), expected, "n={n}, m={m}");
        }
    }
}

#[test]
fn interpolation_reconstructs_polynomial() {
    let mut next = random();
    for n in [0, 1, 3, 67, 129] {
        let points: Vec<_> = (0..n).map(|i| Mi::new(i * 17 + 2)).collect();
        let f = F::new((0..n).map(|_| next()).collect());
        let values: Vec<_> = points.iter().map(|&x| f.eval(x)).collect();
        assert_eq!(interpolate(&points, &values), f);
        assert_eq!(interpolate(&points, &vec![Mi::new(0); n]), F::zeros(n));
    }
}

#[test]
#[should_panic(expected = "must be distinct")]
fn interpolation_rejects_duplicate_points() {
    interpolate(&[Mi::new(1); 2], &[Mi::new(2); 2]);
}

#[test]
fn bostan_mori_matches_long_division() {
    let mut next = random();
    for p_len in [0, 1, 5, 80] {
        for q_len in [1, 2, 5, 65] {
            let p = F::new((0..p_len).map(|_| next()).collect());
            let mut q = F::new((0..q_len).map(|_| next()).collect());
            q[0] = Mi::new(7);
            let expected = naive_div(&p, &q, 140);
            for n in [0, 1, 2, 7, 79, 139] {
                assert_eq!(bostan_mori(&p, &q, n as u64), expected[n]);
            }
        }
    }
    let one = F::from_ints(&[1]);
    let square = F::from_ints(&[1, -2, 1]);
    for n in [1_000_000_000, 1_000_000_000_000_000_000, u64::MAX] {
        assert_eq!(bostan_mori(&one, &square, n), Mi::new(n) + Mi::new(1));
        assert_eq!(
            linear_recurrence(&[Mi::new(3)], &[Mi::new(5)], n),
            Mi::new(3) * Mi::new(5).pow(n)
        );
    }
    assert_eq!(bostan_mori(&F::zeros(0), &one, u64::MAX), Mi::new(0));
    assert_eq!(
        linear_recurrence::<998244353>(&[], &[], u64::MAX),
        Mi::new(0)
    );
}

#[test]
fn recovered_recurrences_predict_unseen_terms() {
    let mut next = random();
    for degree in [1, 2, 3, 8, 65] {
        for trailing_zero in [false, true] {
            let mut coefficients: Vec<_> = (0..degree).map(|_| next()).collect();
            if trailing_zero {
                coefficients[degree - 1] = Mi::new(0);
            }
            let mut sequence: Vec<_> = (0..degree).map(|_| next()).collect();
            for i in degree..220 {
                let value = (0..degree)
                    .map(|j| coefficients[j] * sequence[i - 1 - j])
                    .sum();
                sequence.push(value);
            }
            let recovered = berlekamp_massey(&sequence[..degree * 2]);
            assert!(recovered.len() <= degree);
            for n in [0, degree, degree * 2, 219] {
                assert_eq!(
                    linear_recurrence(&sequence[..degree], &coefficients, n as u64),
                    sequence[n]
                );
                assert_eq!(
                    linear_recurrence(&sequence[..recovered.len()], &recovered, n as u64),
                    sequence[n]
                );
            }
        }
    }
    for sequence in [
        vec![],
        vec![Mi::new(0); 10],
        vec![Mi::new(7); 10],
        [0, 0, 0, 1, 0, 0, 0, 0].map(Mi::new).to_vec(),
    ] {
        let recovered = berlekamp_massey(&sequence);
        for n in recovered.len()..sequence.len() {
            assert_eq!(
                linear_recurrence(&sequence[..recovered.len()], &recovered, n as u64),
                sequence[n]
            );
        }
    }
}

#[test]
fn convolution_dp_matches_sequential_updates() {
    let mut next = random();
    for n in [0, 1, 31, 32, 33, 63, 129, 400] {
        for kernel_len in [0, 1, 2, 6, n + 5] {
            let initial: Vec<_> = (0..n).map(|_| next()).collect();
            let kernel: Vec<_> = (0..kernel_len).map(|_| next()).collect();
            let correct = |i: usize, value| {
                if i % 7 == 3 {
                    Mi::new(0)
                } else {
                    value * Mi::new(i + 2) + Mi::new(i)
                }
            };
            let mut expected = initial.clone();
            for i in 0..n {
                let mut value = initial[i];
                for (j, &previous) in expected.iter().take(i).enumerate() {
                    value += previous * kernel.get(i - j).copied().unwrap_or(Mi::new(0));
                }
                expected[i] = correct(i, value);
            }
            let mut calls = 0;
            let got = convolution_dp(&initial, &kernel, |i, value| {
                assert_eq!(i, calls);
                calls += 1;
                correct(i, value)
            });
            assert_eq!(calls, n);
            assert_eq!(got, expected, "n={n}, kernel_len={kernel_len}");
        }
    }
}

#[test]
fn cyclic_power_matches_repeated_cyclic_convolution() {
    let mut next = random();
    for n in [1, 2, 4, 64, 128] {
        let f = F::new((0..n + 3).map(|_| next()).collect());
        for exponent in [0, 1, 2, 7] {
            let mut expected = F::zeros(n);
            expected[0] = Mi::new(1);
            for _ in 0..exponent {
                let mut product = F::zeros(n);
                for (i, &a) in expected.coef().iter().enumerate() {
                    for (j, &b) in f.coef().iter().enumerate() {
                        product[(i + j) % n] += a * b;
                    }
                }
                expected = product;
            }
            assert_eq!(f.pow_cyclic(exponent, n), expected);
        }
        let mut expected = F::zeros(n);
        expected[(11u128 * u64::MAX as u128 % n as u128) as usize] = Mi::new(1);
        assert_eq!(
            F::from_ints(&[1]).shift_up(11).pow_cyclic(u64::MAX, n),
            expected
        );
    }
    assert_eq!(F::zeros(0).pow_cyclic(0, 4), F::from_ints(&[1, 0, 0, 0]));
    assert_eq!(F::zeros(0).pow_cyclic(10, 4), F::zeros(4));
}

#[test]
fn factorials_match_pascal_triangle() {
    let table = Factorials::<998244353>::new(256);
    let mut row = vec![Mi::new(1)];
    for n in 0..=256 {
        assert_eq!(table.fact[n] * table.inv_fact[n], Mi::new(1));
        for (k, &expected) in row.iter().enumerate() {
            assert_eq!(table.binom(n, k), expected);
        }
        assert_eq!(table.binom(n, n + 1), Mi::new(0));
        let mut next = vec![Mi::new(1); n + 2];
        for k in 1..=n {
            next[k] = row[k - 1] + row[k];
        }
        row = next;
    }
    assert_eq!(Factorials::<7>::new(6).binom(6, 3).val(), 6);
    assert_eq!(Factorials::<7>::new(0).fact[0].val(), 1);
}
