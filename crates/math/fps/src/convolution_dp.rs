use convolution::convolution;
use modint::ModInt;

/// 分割統治 FFT で次の DP を `O(n log^2(n+1))` 時間で求める。
///
/// `dp[i] = finalize(i, initial[i] + sum_{j<i} dp[j] * kernel[i-j])`。
/// `n = initial.len()`、`kernel` の範囲外は 0。`kernel[0]` は使わない。
/// `finalize` は `i = 0, 1, ...` の順に1回ずつ呼ぶ。落とし穴で 0 を返すなど、
/// 各項を確定するときの補正に使える。終端では到達量を集計して 0 を返せば伝播が止まる。
///
/// ```
/// use fps::convolution_dp;
/// use modint::ModInt998244353 as Mi;
/// // 1歩・2歩ずつ進む方法の個数。ただし位置 3 は通れない。
/// let initial = [1, 0, 0, 0, 0, 0].map(Mi::new);
/// let kernel = [0, 1, 1].map(Mi::new);
/// let dp = convolution_dp(&initial, &kernel, |i, value| {
///     if i == 3 { Mi::new(0) } else { value }
/// });
/// assert_eq!(dp, [1, 1, 2, 0, 2, 2].map(Mi::new));
/// ```
pub fn convolution_dp<const M: u32>(
    initial: &[ModInt<M>],
    kernel: &[ModInt<M>],
    mut finalize: impl FnMut(usize, ModInt<M>) -> ModInt<M>,
) -> Vec<ModInt<M>> {
    fn solve<const M: u32>(
        dp: &mut [ModInt<M>],
        kernel: &[ModInt<M>],
        offset: usize,
        finalize: &mut impl FnMut(usize, ModInt<M>) -> ModInt<M>,
    ) {
        let n = dp.len();
        if n <= 32 {
            for i in 0..n {
                dp[i] = finalize(offset + i, dp[i]);
                for j in i + 1..n.min(i + kernel.len()) {
                    let contribution = dp[i] * kernel[j - i];
                    dp[j] += contribution;
                }
            }
            return;
        }
        let mid = n / 2;
        solve(&mut dp[..mid], kernel, offset, finalize);
        let cross = convolution(&dp[..mid], &kernel[..kernel.len().min(n)]);
        for i in mid..n.min(cross.len()) {
            dp[i] += cross[i];
        }
        solve(&mut dp[mid..], kernel, offset + mid, finalize);
    }

    let mut dp = initial.to_vec();
    solve(&mut dp, kernel, 0, &mut finalize);
    dp
}
