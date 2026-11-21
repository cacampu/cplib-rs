//! 選択の作業領域と、標準の部品。
//!
//! ユーザーが上書きするのは [`Logic::select`] で、ここにあるのはそこへ渡ってくる
//! 道具箱。毎ターン使い回すバッファを持つので `BeamSearch` が所有する。
//!
//! 何もしなければ [`Selector::dedup_top_k`]（ハッシュで重複除去して `value` 上位
//! `width`）が使われる。多様性制約などが要る問題だけ `Logic::select` を上書きし、
//! [`dedup`](Selector::dedup) や [`dedup_sorted`](Selector::dedup_sorted) を
//! 部品として使う。

use std::cmp::Ordering;
use std::hash::{BuildHasher, BuildHasherDefault, Hasher};

use super::logic::Logic;

/// `L::Hash` を、ハッシュ表がそのまま使える 64bit のキーに直す。
///
/// [`FlatHashSet`] / [`FlatHashMap`] に `L::Hash` を入れたいときはこれを通す。
/// `L::Hash = u64` なら恒等だが、それより狭い型では**必ず通すこと**（理由は下の
/// [`PreHashed`] を参照）。
pub fn hash_key<L: Logic>(node: &L::Node, ctx: &L::Context) -> u64 {
    let bh: BuildHasherDefault<PreHashed> = Default::default();
    bh.hash_one(L::hash(node, ctx))
}

/// 既にハッシュ済みのキーを受け取る前提の `Hasher`。整数以外は `write` で panic する。
///
/// **64bit 未満のキーだけ** splitmix64 の finalizer で全ビットに散らす。狭いキーを
/// そのまま広げると上位ビットが常に 0 になり、ハッシュ表がタグに使う上位ビットが
/// 効かなくなるため。`u64` はそのまま通すので、`L::Hash = u64` のときは**下位ビット
/// まで一様であること**が前提になる（Zobrist なら満たす）。
#[derive(Default, Clone, Copy)]
pub struct PreHashed {
    hash: u64,
}

impl Hasher for PreHashed {
    fn write(&mut self, _: &[u8]) {
        unimplemented!("PreHashed は整数のキーしか受け付けない")
    }
    #[inline]
    fn write_u8(&mut self, n: u8) {
        self.hash = spread(n as u64);
    }
    #[inline]
    fn write_u16(&mut self, n: u16) {
        self.hash = spread(n as u64);
    }
    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.hash = spread(n as u64);
    }
    /// 64bit は Zobrist が一様である前提でそのまま通す。
    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.hash = n;
    }
    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.hash = n as u64;
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

/// splitmix64 の finalizer。狭いキーを 64bit 全体に散らす。
#[inline]
fn spread(x: u64) -> u64 {
    let mut z = x;
    z = (z ^ (z >> 33)).wrapping_mul(0xff51_afd7_ed55_8ccd);
    z = (z ^ (z >> 33)).wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    z ^ (z >> 33)
}

/// ハッシュ済みのキーから `u32` への写像。開番地法（線形探索）。
pub struct FlatHashMap {
    /// 探索中に舐める列。値と分けてあるのは、線形探索が触るバイト数を減らすため
    /// （`(u64, u32)` を1本にすると詰め物で 16 バイトになり、表の大きさが倍になる）。
    keys: Vec<u64>,
    /// 当たったときだけ読む列。
    vals: Vec<u32>,
    /// 使用中フラグ。探索の大半はここで完結する。
    occupied: Vec<u64>,
    mask: usize,
}

impl FlatHashMap {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            vals: Vec::new(),
            occupied: Vec::new(),
            mask: 0,
        }
    }

    /// 最大 `n` 要素を入れられるように確保する。既に足りていれば何もしない。
    ///
    /// 倍率 2 は実測で決めた。大きくすると表がキャッシュから溢れて逆効果になる。
    pub fn reserve(&mut self, n: usize) {
        // ビットマップが 1 語に満たないと困るので 64 を下限にする。
        let cap = (n * 2).next_power_of_two().max(64);
        if cap <= self.keys.len() {
            return;
        }
        self.keys = vec![0; cap];
        self.vals = vec![0; cap];
        self.occupied = vec![0; cap / 64];
        self.mask = cap - 1;
    }

    /// 空にする。確保済みの領域は保つ。
    pub fn clear(&mut self) {
        self.occupied.fill(0);
    }

    /// `h` が既にあればその値を返す。無ければ `v` を入れて `None` を返す。
    #[inline]
    pub fn get_or_insert(&mut self, h: u64, v: u32) -> Option<u32> {
        let mut i = (h as usize) & self.mask;
        loop {
            let (w, b) = (i >> 6, 1u64 << (i & 63));
            if self.occupied[w] & b == 0 {
                self.occupied[w] |= b;
                self.keys[i] = h;
                self.vals[i] = v;
                return None;
            }
            if self.keys[i] == h {
                return Some(self.vals[i]);
            }
            i = (i + 1) & self.mask;
        }
    }
}

impl Default for FlatHashMap {
    fn default() -> Self {
        Self::new()
    }
}

/// ハッシュ済みのキーの集合。[`FlatHashMap`] の値を持たない版。
///
/// 「もう見たか」だけを問う用途（重複除去）はこちらの方が軽い。走査中に自前で
/// 重複除去したい場合に `Context` に持たせて使う。
pub struct FlatHashSet {
    keys: Vec<u64>,
    occupied: Vec<u64>,
    mask: usize,
}

impl FlatHashSet {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            occupied: Vec::new(),
            mask: 0,
        }
    }

    /// 最大 `n` 要素を入れられるように確保する。既に足りていれば何もしない。
    pub fn reserve(&mut self, n: usize) {
        let cap = (n * 2).next_power_of_two().max(64);
        if cap <= self.keys.len() {
            return;
        }
        self.keys = vec![0; cap];
        self.occupied = vec![0; cap / 64];
        self.mask = cap - 1;
    }

    /// 空にする。確保済みの領域は保つ。
    pub fn clear(&mut self) {
        self.occupied.fill(0);
    }

    /// 新規なら `true`。既にあれば `false`。
    #[inline]
    pub fn insert(&mut self, h: u64) -> bool {
        let mut i = (h as usize) & self.mask;
        loop {
            let (w, b) = (i >> 6, 1u64 << (i & 63));
            if self.occupied[w] & b == 0 {
                self.occupied[w] |= b;
                self.keys[i] = h;
                return true;
            }
            if self.keys[i] == h {
                return false;
            }
            i = (i + 1) & self.mask;
        }
    }
}

impl Default for FlatHashSet {
    fn default() -> Self {
        Self::new()
    }
}

/// 選択の作業領域と標準の部品。[`Logic::select`] に渡ってくる。
pub struct Selector<L: Logic> {
    table: FlatHashMap,
    /// 生存者の `(value, 候補番号)`。ハッシュごとに最良の1つ。
    survivors: Vec<(L::Value, u32)>,
    /// [`dedup`](Self::dedup) 系が返す添字列。
    idx_buf: Vec<u32>,
    /// 直近の重複除去で残った件数。`Logic::LOG` の表示に使う。0 は「測っていない」。
    unique: usize,
}

impl<L: Logic> Selector<L> {
    pub fn new() -> Self {
        Self {
            table: FlatHashMap::new(),
            survivors: Vec::new(),
            idx_buf: Vec::new(),
            unique: 0,
        }
    }

    #[inline]
    fn is_better(a: L::Value, b: L::Value) -> bool {
        if L::MINIMIZE { a < b } else { a > b }
    }

    /// 良い順の比較。
    #[inline]
    fn cmp(a: &(L::Value, u32), b: &(L::Value, u32)) -> Ordering {
        if L::MINIMIZE {
            a.0.cmp(&b.0)
        } else {
            b.0.cmp(&a.0)
        }
    }

    /// ハッシュごとに `value` 最良の1つを `self.survivors` に集める。
    fn collect(&mut self, cands: &[L::Node], ctx: &L::Context) {
        self.survivors.clear();
        self.table.reserve(cands.len());
        self.table.clear();
        for (i, node) in cands.iter().enumerate() {
            let v = L::value(node, ctx);
            let h = hash_key::<L>(node, ctx);
            let slot = self.survivors.len() as u32;
            match self.table.get_or_insert(h, slot) {
                None => self.survivors.push((v, i as u32)),
                Some(s) => {
                    let e = &mut self.survivors[s as usize];
                    if Self::is_better(v, e.0) {
                        *e = (v, i as u32);
                    }
                }
            }
        }
        self.unique = self.survivors.len();
    }

    /// 直近の重複除去で残った件数。重複除去を通らない選択をした場合は 0。
    pub fn unique(&self) -> usize {
        self.unique
    }

    pub(super) fn reset_unique(&mut self) {
        self.unique = 0;
    }

    fn take_idx(&mut self) -> &[u32] {
        self.idx_buf.clear();
        self.idx_buf.extend(self.survivors.iter().map(|&(_, i)| i));
        &self.idx_buf
    }

    /// ハッシュで重複除去し、生存者の候補番号を返す。並びは候補の出現順。
    ///
    /// 同じハッシュが複数あれば `value` が最良のものを残す。
    pub fn dedup(&mut self, cands: &[L::Node], ctx: &L::Context) -> &[u32] {
        self.collect(cands, ctx);
        self.take_idx()
    }

    /// [`dedup`](Self::dedup) と同じだが、`value` の良い順に並べて返す。
    ///
    /// 良い順に1つずつ判定したい場合（多様性制約など）に使う。
    pub fn dedup_sorted(&mut self, cands: &[L::Node], ctx: &L::Context) -> &[u32] {
        self.collect(cands, ctx);
        self.survivors.sort_unstable_by(Self::cmp);
        self.take_idx()
    }

    /// **最も基本的な選択。** ハッシュで重複除去し、`value` 上位 `width` を選ぶ。
    ///
    /// 全体を並べ替えず `select_nth_unstable` で切るので O(候補数)。
    /// [`Logic::select`] の既定実装がこれ。`out` の並びは不定。
    pub fn dedup_top_k(
        &mut self,
        cands: &[L::Node],
        ctx: &L::Context,
        width: usize,
        out: &mut Vec<u32>,
    ) {
        if width == 0 {
            return;
        }
        self.collect(cands, ctx);
        if self.survivors.len() > width {
            self.survivors.select_nth_unstable_by(width - 1, Self::cmp);
            self.survivors.truncate(width);
        }
        out.extend(self.survivors.iter().map(|&(_, i)| i));
    }
}

impl<L: Logic> Selector<L> {
    /// 全候補を `value` の良い順に走査し、`keep` が true を返したものを `out` に
    /// 書く。`width` 個に達したら打ち切る。
    ///
    /// **重複除去はしない。** 必要なら `keep` の中に書く（[`FlatHashSet`] を
    /// `Context` に持たせる）。走査が1回で済むので、`hash` と判定に使う情報を同じ
    /// タッチで読める。
    ///
    /// [`dedup_top_k`](Self::dedup_top_k) との使い分けは重複率で決まる。重複が多い
    /// なら先に潰す `dedup_top_k`、少なくて走査中に状態も更新したいならこちら。
    pub fn sorted_filter(
        &mut self,
        cands: &[L::Node],
        ctx: &mut L::Context,
        width: usize,
        out: &mut Vec<u32>,
        mut keep: impl FnMut(&L::Node, &mut L::Context) -> bool,
    ) {
        if width == 0 {
            return;
        }
        // 重複除去を通らないので件数は測れない。
        self.unique = 0;
        self.survivors.clear();
        self.survivors.reserve(cands.len());
        for (i, node) in cands.iter().enumerate() {
            self.survivors.push((L::value(node, ctx), i as u32));
        }
        self.survivors.sort_unstable_by(Self::cmp);
        for k in 0..self.survivors.len() {
            let i = self.survivors[k].1;
            if keep(&cands[i as usize], ctx) {
                out.push(i);
                if out.len() == width {
                    break;
                }
            }
        }
    }
}

impl<L: Logic> Default for Selector<L> {
    fn default() -> Self {
        Self::new()
    }
}
