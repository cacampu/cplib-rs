//! ビーム幅の決定。

pub trait WidthPolicy {
    /// バッファ確保に使う上限。実際の幅がこれを超えても動くが、再確保が起きる。
    fn max_width(&self) -> usize;

    /// このターンの幅を決める。`max_turn` は `BeamSearch::run` に渡された総ターン数。
    fn suggest(&mut self, turn: usize, max_turn: usize) -> usize;

    /// ターンの終わりに呼ばれる。`elapsed` は**探索開始からの累計秒数**。
    ///
    /// 時計はライブラリ側に1つだけ置く。残り時間から幅を決める政策は、この累計を
    /// 締切と突き合わせればよく、自前の `Instant` を持つ必要がない。
    fn observe(&mut self, _turn: usize, _width: usize, _elapsed: f64) {}
}

/// 固定幅。
pub struct FixedWidth {
    width: usize,
}

impl FixedWidth {
    pub fn new(width: usize) -> Self {
        assert!(width > 0);
        Self { width }
    }
}

impl WidthPolicy for FixedWidth {
    fn max_width(&self) -> usize {
        self.width
    }
    fn suggest(&mut self, _turn: usize, _max_turn: usize) -> usize {
        self.width
    }
}

/// 直近の実測から幅を調整する。`interval` ターンごとに更新する。
///
/// ```text
/// M ← clamp(M * sqrt(interval * r / (a * e)), M_min, M_max)
/// ```
///
/// `a` は残りターン数、`r` は残り時間、`e` は直近 `interval` ターンの所要時間。
/// 平方根と clamp は急激な変化を抑えるため。
///
pub struct SqrtFeedback {
    width: usize,
    min_width: usize,
    max_width: usize,
    interval: usize,
    /// `BeamSearch::new` から数えた打ち切り秒数。
    time_limit: f64,
    /// `observe` が渡してくる累計経過秒。
    elapsed: f64,
    /// 前回幅を更新した時点の `elapsed`。
    marked: f64,
}

impl SqrtFeedback {
    /// `time_limit` は `BeamSearch::new` からの秒数。プロセス開始からの猶予で
    /// 考えたい場合は [`with_elapsed`](Self::with_elapsed) で起動分を差し引く。
    pub fn new(initial: usize, min_width: usize, max_width: usize, time_limit: f64) -> Self {
        assert!(0 < min_width && min_width <= max_width);
        Self {
            width: initial.clamp(min_width, max_width),
            min_width,
            max_width,
            interval: 256,
            time_limit,
            elapsed: 0.0,
            marked: 0.0,
        }
    }

    /// 既に時間を消費している場合に、その分を差し引いた残り時間で開始する。
    pub fn with_elapsed(mut self, elapsed: f64) -> Self {
        self.time_limit -= elapsed;
        self
    }

    pub fn with_interval(mut self, interval: usize) -> Self {
        assert!(interval > 0);
        self.interval = interval;
        self
    }

    pub fn width(&self) -> usize {
        self.width
    }
}

impl WidthPolicy for SqrtFeedback {
    fn max_width(&self) -> usize {
        self.max_width
    }

    fn suggest(&mut self, turn: usize, max_turn: usize) -> usize {
        if turn > 0 && turn % self.interval == 0 {
            let spent = self.elapsed - self.marked;
            let remaining = (self.time_limit - self.elapsed).max(0.0);
            let turns_left = max_turn.saturating_sub(turn);
            if spent > 0.0 && turns_left > 0 {
                let ratio = (remaining * self.interval as f64 / (spent * turns_left as f64)).sqrt();
                let next = (self.width as f64 * ratio) as usize;
                self.width = next.clamp(self.min_width, self.max_width);
            }
            self.marked = self.elapsed;
        }
        self.width
    }

    fn observe(&mut self, _turn: usize, _width: usize, elapsed: f64) {
        self.elapsed = elapsed;
    }
}

/// 1 ターンの所要時間を推定しながら、残り時間を使い切るように幅を決める。
///
/// [`SqrtFeedback`] が直近の実測をそのまま比に使うのに対し、こちらは**「幅 1・
/// 1 ターンあたりの所要時間」の対数**をカルマンフィルタで推定し、その事後分布の
/// **+3σ（悲観側）**で幅を割り出す。ノイズの大きいターンに引きずられにくい。
///
/// - 毎ターン更新するので `interval` に相当するものは無い
/// - 最初の 1 ターンは推定値が無いので `initial` をそのまま使う
/// - 使い残した時間は残りターンに配り直されるので、+3σ を取っても**時間は使い切る**。
///   代わりに幅が「序盤は控えめ、終盤で膨らむ」形になる。均したいときは
///   [`with_noise`](Self::with_noise) でノイズを小さくする
pub struct BayesianWidth {
    width: usize,
    min_width: usize,
    max_width: usize,
    /// `BeamSearch::new` から数えた打ち切り秒数。
    time_limit: f64,
    /// `observe` が渡してくる累計経過秒。
    elapsed: f64,
    /// 直前のターン終了時点の `elapsed`。1 ターンの所要時間を出すのに使う。
    marked: f64,
    /// 「幅 1・1 ターンあたりの所要時間」の対数。その事後平均。
    log_unit: f64,
    /// 上の事後分散。
    variance: f64,
    /// 観測ノイズの分散。大きいほど 1 回の観測を信用しない。
    observation_variance: f64,
    /// 毎ターン事後分散へ加える量。1 ターンの重さが探索の進行で変わるのを追う。
    drift_variance: f64,
    observed: bool,
}

impl BayesianWidth {
    /// `time_limit` は `BeamSearch::new` からの秒数。プロセス開始からの猶予で
    /// 考えたい場合は [`with_elapsed`](Self::with_elapsed) で起動分を差し引く。
    pub fn new(initial: usize, min_width: usize, max_width: usize, time_limit: f64) -> Self {
        assert!(0 < min_width && min_width <= max_width);
        Self {
            width: initial.clamp(min_width, max_width),
            min_width,
            max_width,
            time_limit,
            elapsed: 0.0,
            marked: 0.0,
            log_unit: 0.0,
            variance: 0.0,
            observation_variance: 0.02,
            drift_variance: 0.002,
            observed: false,
        }
    }

    /// 既に時間を消費している場合に、その分を差し引いた残り時間で開始する。
    pub fn with_elapsed(mut self, elapsed: f64) -> Self {
        self.time_limit -= elapsed;
        self
    }

    /// 観測ノイズと過程ノイズの分散（対数時間での値）。既定は `(0.02, 0.002)`。
    ///
    /// 観測を大きくすると 1 回の実測を信用せず滑らかになり、過程を大きくすると
    /// 1 ターンの重さの変化に速く追従する。
    ///
    /// **どちらも小さくすると +3σ の余裕が縮み、幅の変動が平らになる。** 総時間は
    /// どちらでも使い切るので、効くのは「序盤を削って終盤に回す度合い」である。
    /// 幅が均一なほうが解の質は安定しやすいので、時間の見積もりが素直な問題では
    /// 小さめにする価値がある。
    pub fn with_noise(mut self, observation: f64, drift: f64) -> Self {
        assert!(observation > 0.0 && drift >= 0.0);
        self.observation_variance = observation;
        self.drift_variance = drift;
        self
    }

    pub fn width(&self) -> usize {
        self.width
    }
}

impl WidthPolicy for BayesianWidth {
    fn max_width(&self) -> usize {
        self.max_width
    }

    fn suggest(&mut self, turn: usize, max_turn: usize) -> usize {
        if !self.observed {
            return self.width;
        }
        let remaining = (self.time_limit - self.elapsed).max(0.0);
        let turns_left = max_turn.saturating_sub(turn).max(1);
        let budget = remaining / turns_left as f64;
        // 悲観側で見積もる。時間を余らせるより超過するほうが痛いため。
        let unit = (self.log_unit + 3.0 * self.variance.sqrt()).exp();
        // float -> int は飽和変換。NaN は 0 になり、下の clamp で拾われる。
        self.width = ((budget / unit) as usize).clamp(self.min_width, self.max_width);
        self.width
    }

    fn observe(&mut self, _turn: usize, width: usize, elapsed: f64) {
        let spent = elapsed - self.marked;
        self.marked = elapsed;
        self.elapsed = elapsed;
        if spent <= 0.0 || width == 0 {
            return;
        }
        let z = (spent / width as f64).ln();
        if !self.observed {
            self.log_unit = z;
            self.variance = self.observation_variance;
            self.observed = true;
            return;
        }
        self.variance += self.drift_variance;
        let gain = self.variance / (self.variance + self.observation_variance);
        self.log_unit += gain * (z - self.log_unit);
        self.variance *= 1.0 - gain;
    }
}
