//! 再帰・呼び出し時借用・インライン展開の使い分け。
use cplib_lang_io::{fastout, outln};
use cplib_lang_macros::{lazyfn, lazymac, recurfn};
use cplib_search_binary_search::binary_search;

#[fastout]
fn main() {
    let tree = [vec![1, 4], vec![2, 3], vec![], vec![], vec![5], vec![]];
    recurfn!(
        size = |sizes: &mut [usize], u: usize| -> usize {
            let children: usize = tree[u].iter().map(|&v| size(sizes, v)).sum();
            sizes[u] = children + 1;
            sizes[u]
        }
    );
    let mut sizes = [0; 6];
    outln!("{}", size(&mut sizes, 0));

    let mut weights = [1i64; 6];
    lazyfn!(recur sum = |sub: &mut [i64], u: usize| -> i64 {
        let children: i64 = tree[u].iter().map(|&v| sum(sub, v)).sum();
        sub[u] = weights[u] + children;
        sub[u]
    });
    let mut sub = [0; 6];
    outln!("{}", sum!(&mut sub, 0));
    weights[3] = 100; // 呼び出し間ではキャプチャ対象も更新できる。
    outln!("{}", sum!(&mut sub, 0));

    let mut limit = 50;
    let mut calls = 0;
    lazyfn!(
        check = |m: i64| -> bool {
            calls += 1;
            m * m <= limit
        }
    );
    outln!("{}", binary_search(0i64, 2_000_000, check!(fn)));
    limit = 100;
    outln!("{}", binary_search(0i64, 2_000_000, check!(fn)));

    let xs = [3usize, 1, 3, 2];
    let mut count = [0; 4];
    let mut kinds = 0;
    lazymac!(
        add = |i| {
            let x = xs[i];
            if count[x] == 0 {
                kinds += 1;
            }
            count[x] += 1;
        }
    );
    lazymac!(
        remove = |i| {
            let x = xs[i];
            count[x] -= 1;
            if count[x] == 0 {
                kinds -= 1;
            }
        }
    );
    add!(0);
    add!(1);
    add!(2);
    remove!(0);
    outln!("{kinds}");
}
