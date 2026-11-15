// Fn の共有借用で再帰する。再帰先へ渡す可変状態は call の引数として毎回再借用する。
#[doc(hidden)]
pub struct Recursive<F>(pub F);

#[doc(hidden)]
pub trait Call<Args, Output> {
    fn call(&self, args: Args) -> Output;
}

impl<Args, Output, F> Call<Args, Output> for Recursive<F>
where
    F: Fn(&dyn Call<Args, Output>, Args) -> Output,
{
    fn call(&self, args: Args) -> Output {
        (self.0)(self, args)
    }
}

#[doc(hidden)]
pub trait CallState<State: ?Sized, Args, Output> {
    fn call(&self, state: &mut State, args: Args) -> Output;
}

impl<State: ?Sized, Args, Output, F> CallState<State, Args, Output> for Recursive<F>
where
    F: Fn(&dyn CallState<State, Args, Output>, &mut State, Args) -> Output,
{
    fn call(&self, state: &mut State, args: Args) -> Output {
        (self.0)(self, state, args)
    }
}

/// 同名の関数呼び出しで再帰できるクロージャを定義する。`unsafe` は使わない。
/// 引数の型は必須。戻り値を省略すると `()`。
///
/// ```
/// use macros::recurfn;
/// let weights = [3u64, 5, 7];
/// recurfn!(sum = |n: usize| -> u64 {
///     if n == 0 { 0 } else { weights[n - 1] + sum(n - 1) }
/// });
/// assert_eq!(sum(3), 15);
/// ```
///
/// 可変状態は最初の引数 `state: &mut T` で渡す。キャプチャは読み取り専用。
///
/// ```
/// use macros::recurfn;
/// let tree = [vec![1, 2], vec![], vec![]];
/// recurfn!(dfs = |sub: &mut [usize], u: usize| -> usize {
///     sub[u] = 1;
///     for &v in &tree[u] { sub[u] += dfs(sub, v); }
///     sub[u]
/// });
/// let mut sub = [0; 3];
/// assert_eq!(dfs(&mut sub, 0), 3);
/// assert_eq!(sub, [3, 1, 1]);
/// ```
///
/// 再帰をまたいで状態の要素を借用すると、通常の関数と同じくコンパイルエラーになる。
///
/// ```compile_fail
/// use macros::recurfn;
/// recurfn!(walk = |xs: &mut Vec<i32>, n: usize| {
///     let first = &xs[0];
///     if n > 0 { walk(xs, n - 1); }
///     xs.push(*first);
/// });
/// ```
///
/// 再帰クロージャで外部変数を直接可変キャプチャすることはできない。
///
/// ```compile_fail
/// use macros::recurfn;
/// let mut total = 0;
/// recurfn!(sum = |n: usize| {
///     total += n;
///     if n > 0 { sum(n - 1); }
/// });
/// sum(3);
/// ```
#[macro_export]
macro_rules! recurfn {
    ($name:ident = || $(-> $r:ty)? $body:block) => {
        $crate::recurfn!(@plain $name [] [$($r)?] $body);
    };
    ($name:ident = |$state:ident: &mut $s:ty $(, $a:ident: $t:ty)* $(,)?| $(-> $r:ty)? $body:block) => {
        $crate::recurfn!(@state $name [$state: $s] [$($a: $t),*] [$($r)?] $body);
    };
    ($name:ident = |$($a:ident: $t:ty),* $(,)?| $(-> $r:ty)? $body:block) => {
        $crate::recurfn!(@plain $name [$($a: $t),*] [$($r)?] $body);
    };
    (@plain $name:ident [$($a:ident: $t:ty),*] [] $body:block) => {
        $crate::recurfn!(@plain $name [$($a: $t),*] [()] $body);
    };
    (@state $name:ident [$state:ident: $s:ty] [$($a:ident: $t:ty),*] [] $body:block) => {
        $crate::recurfn!(@state $name [$state: $s] [$($a: $t),*] [()] $body);
    };
    (@plain $name:ident [$($a:ident: $t:ty),*] [$r:ty] $body:block) => {
        let __recursive = $crate::__Recursive(
            |__this: &dyn $crate::__Call<($($t,)*), $r>, ($($a,)*): ($($t,)*)| -> $r {
                #[allow(unused_variables)]
                let $name = |$($a: $t),*| -> $r { __this.call(($($a,)*)) };
                $body
            }
        );
        let $name = move |$($a: $t),*| -> $r {
            $crate::__Call::call(&__recursive, ($($a,)*))
        };
    };
    (@state $name:ident [$state:ident: $s:ty] [$($a:ident: $t:ty),*] [$r:ty] $body:block) => {
        let __recursive = $crate::__Recursive(
            |__this: &dyn $crate::__CallState<$s, ($($t,)*), $r>, $state: &mut $s, ($($a,)*): ($($t,)*)| -> $r {
                #[allow(unused_variables)]
                let $name = |$state: &mut $s, $($a: $t),*| -> $r {
                    __this.call($state, ($($a,)*))
                };
                $body
            }
        );
        let $name = move |$state: &mut $s, $($a: $t),*| -> $r {
            $crate::__CallState::call(&__recursive, $state, ($($a,)*))
        };
    };
}

/// 呼び出した式の間だけキャプチャを借用するローカルマクロを定義する。
/// `name!(args)` で呼び、`name!(fn)` で通常のクロージャ値を得る。
///
/// ```
/// use macros::lazyfn;
/// let mut limit = 10;
/// let mut calls = 0;
/// lazyfn!(check = |x: i32| -> bool { calls += 1; x <= limit });
/// assert!(check!(7));
/// limit = 5;
/// assert!(!check!(7));
/// assert_eq!((0..10).filter(|&x| check!(x)).count(), 6);
/// let count = (0..10).map(check!(fn)).filter(|&ok| ok).count();
/// assert_eq!(count, 6);
/// ```
///
/// 再帰版は `lazyfn!(recur name = |...| { ... })`。可変状態は `recurfn!` と同じく第1引数。
/// `return` / `?` はクロージャから抜ける。`name!(fn)` を変数に保持すると、その間は借用が続く。
#[macro_export]
macro_rules! lazyfn {
    (recur $name:ident = || $(-> $r:ty)? $body:block) => {
        $crate::lazyfn!(@ ($) [recur] $name [] [] [] [$($r)?] $body);
    };
    (recur $name:ident = |$state:ident: &mut $s:ty $(, $a:ident: $t:ty)* $(,)?| $(-> $r:ty)? $body:block) => {
        $crate::lazyfn!(@ ($) [recur] $name [$state: &mut $s $(, $a: $t)*] [$state $(, $a)*] [&mut $s $(, $t)*] [$($r)?] $body);
    };
    (recur $name:ident = |$($a:ident: $t:ty),* $(,)?| $(-> $r:ty)? $body:block) => {
        $crate::lazyfn!(@ ($) [recur] $name [$($a: $t),*] [$($a),*] [$($t),*] [$($r)?] $body);
    };
    ($name:ident = || $(-> $r:ty)? $body:block) => {
        $crate::lazyfn!(@ ($) [plain] $name [] [] [] [$($r)?] $body);
    };
    ($name:ident = |$($a:ident: $t:ty),* $(,)?| $(-> $r:ty)? $body:block) => {
        $crate::lazyfn!(@ ($) [plain] $name [$($a: $t),*] [$($a),*] [$($t),*] [$($r)?] $body);
    };
    (@ ($d:tt) [$mode:ident] $name:ident [$($sig:tt)*] [$($a:ident),*] [$($t:ty),*] [] $body:block) => {
        $crate::lazyfn!(@ ($d) [$mode] $name [$($sig)*] [$($a),*] [$($t),*] [()] $body);
    };
    (@ ($d:tt) [$mode:ident] $name:ident [$($sig:tt)*] [$($a:ident),*] [$($t:ty),*] [$r:ty] $body:block) => {
        #[allow(unused_macros)]
        macro_rules! $name {
            (fn) => {
                |$($sig)*| -> $r {
                    $crate::lazyfn!(@body [$mode] $name [$($sig)*] [$($a),*] [$r] $body)
                }
            };
            ($($d $a:expr),* $d(,)?) => {{
                let ($($a,)*): ($($t,)*) = ($($d $a,)*);
                (|$($sig)*| -> $r {
                    $crate::lazyfn!(@body [$mode] $name [$($sig)*] [$($a),*] [$r] $body)
                })($($a),*)
            }};
        }
    };
    (@body [plain] $name:ident [$($sig:tt)*] [$($a:ident),*] [$r:ty] $body:block) => {
        $body
    };
    (@body [recur] $name:ident [$($sig:tt)*] [$($a:ident),*] [$r:ty] $body:block) => {{
        $crate::recurfn!($name = |$($sig)*| -> $r $body);
        $name($($a),*)
    }};
}

/// 呼び出す位置へ本体を展開する非再帰のローカルマクロ。引数の型は省略可能。
/// `return` / `?` / `break` / `continue` は呼び出し元に作用し、値としては渡せない。
///
/// ```
/// use macros::lazymac;
/// let mut values = [0, 0];
/// lazymac!(add = |i, x| { values[i] += x; });
/// add!(0, 3);
/// values[1] = 5;
/// add!(1, 2);
/// assert_eq!(values, [3, 7]);
/// ```
#[macro_export]
macro_rules! lazymac {
    ($name:ident = || $body:expr) => {
        $crate::lazymac!(@ ($) $name [] $body);
    };
    ($name:ident = |$($a:ident $(: $t:ty)?),* $(,)?| $body:expr) => {
        $crate::lazymac!(@ ($) $name [$($a $(: $t)?),*] $body);
    };
    (@ ($d:tt) $name:ident [$($a:ident $(: $t:ty)?),*] $body:expr) => {
        #[allow(unused_macros)]
        macro_rules! $name {
            ($($d $a:expr),* $d(,)?) => {{
                let ($($a,)*): ($($crate::lazymac!(@type $($t)?),)*) = ($($d $a,)*);
                $body
            }};
        }
    };
    (@type $t:ty) => { $t };
    (@type) => { _ };
}
