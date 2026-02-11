use dashu::float::Context;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// 内部計算に使用する2進数浮動小数点数型
type BinFloat = FBig<HalfEven, 2>;

/// Chudnovsky法の進捗状況を保持する構造体
#[derive(Debug, Clone)]
pub struct ChudnovskyProgress {
    /// 推定確定桁数
    pub estimated_digits: usize,
    /// 現在の計算範囲 [0, current_max)
    pub range: (usize, usize),
    /// 追加のカスタムメッセージ
    pub message: Option<String>,
}

/// Binary Splitting法で使用する中間項を保持する構造体
/// Chudnovskyの公式の各項を P, Q, T に分解して計算する
/// 文献（HAKMEM Item 14 等）に倣い、一般的な変数名定義に修正
/// P(a,b) = P(a,m) * P(m,b)
/// Q(a,b) = Q(a,m) * Q(m,b)
/// T(a,b) = T(a,m) * Q(m,b) + P(a,m) * T(m,b)
struct Pqt {
    p: IBig,
    q: IBig,
    t: IBig,
}

/// Binary Splitting法を並列で実行し、Chudnovskyの項を計算する
/// * a, b: 計算範囲 [a, b)
/// * c3: 定数 C^3
/// * counter: 葉ノード完了カウンタ
/// * max_k: 現在到達した最大のk（進捗表示用）
fn bs_parallel(a: i64, b: i64, c3: &IBig, counter: &AtomicUsize, max_k: &AtomicUsize) -> Pqt {
    // 基底ケース: 範囲が1の場合は直接計算する
    if b - a == 1 {
        let k = IBig::from(a);
        let a_val = IBig::from(13591409);
        let b_val = IBig::from(545140134);

        if a == 0 {
            counter.fetch_add(1, Ordering::Relaxed);
            max_k.fetch_max(b as usize, Ordering::Relaxed);
            return Pqt {
                p: IBig::from(1),
                q: IBig::from(1),
                t: a_val,
            };
        }

        // P(k, k+1) = -(6k-5)(2k-1)(6k-1)
        let p: IBig =
            (IBig::from(6) * &k - 5) * (IBig::from(2) * &k - 1) * (IBig::from(6) * &k - 1) * -1;

        // Q(k, k+1) = k^3 * C^3 / 24
        let q_num = k.pow(3) * c3;
        // 整数除算の割り切れ保証（デバッグ時のみチェック）
        debug_assert_eq!(&q_num % IBig::from(24), IBig::ZERO);
        let q = q_num / 24;

        // T(k, k+1) = P(k) * (A + Bk)
        let t = &p * (a_val + b_val * &k);

        counter.fetch_add(1, Ordering::Relaxed);
        max_k.fetch_max(b as usize, Ordering::Relaxed);

        Pqt { p, q, t }
    } else {
        let m = (a + b) / 2;

        // 並列化のしきい値。これより小さい範囲は逐次処理する
        if b - a < 3000 {
            let left = bs_parallel(a, m, c3, counter, max_k);
            let right = bs_parallel(m, b, c3, counter, max_k);

            // 項の結合（ムーブセマンティクスで内部バッファを再利用）
            // T(a,b) = T(a,m) * Q(m,b) + P(a,m) * T(m,b)
            // t は left の t, q と right の q, t を参照するため先に計算
            let new_t = &left.t * &right.q + &left.p * &right.t;
            let new_p = left.p * right.p;
            let new_q = left.q * right.q;
            return Pqt {
                p: new_p,
                q: new_q,
                t: new_t,
            };
        }

        // 左右の範囲を rayon を用いて並列に計算する
        let (left, right) = rayon::join(
            || bs_parallel(a, m, c3, counter, max_k),
            || bs_parallel(m, b, c3, counter, max_k),
        );

        // 項の結合（ムーブセマンティクスで内部バッファを再利用）
        let new_t = &left.t * &right.q + &left.p * &right.t;
        let new_p = left.p * right.p;
        let new_q = left.q * right.q;
        Pqt {
            p: new_p,
            q: new_q,
            t: new_t,
        }
    }
}

/// Chudnovsky法を用いて円周率を高精度に計算する
/// * digits: 求めたい10進数の桁数
/// * on_progress: 進捗状況を受け取るコールバック関数
pub fn calc_chudnovsky<F>(
    digits: usize,
    mut on_progress: F,
) -> crate::error::Result<(BinFloat, Duration)>
where
    F: FnMut(ChudnovskyProgress),
{
    let start = Instant::now();

    // 精度を自動計算: digits * log2(10) + 64
    let precision = (digits as f64 * std::f64::consts::LOG2_10).ceil() as usize + 64;

    // 初期化フェーズの通知
    on_progress(ChudnovskyProgress {
        estimated_digits: 0,
        range: (0, 0),
        message: Some("Initializing...".to_string()),
    });

    // 必要な項数を算出 (1項あたり約14.18桁)
    let n = digits / 14 + 1;

    let c = IBig::from(640320);
    let c3 = &c * &c * &c;

    // Binary Splitting（AtomicUsize カウンタで進捗を追跡しつつ並列実行）
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_for_thread = Arc::clone(&counter);

    // 最大到達インデックス（進捗表示用）
    let max_k = Arc::new(AtomicUsize::new(0));
    let max_k_for_thread = Arc::clone(&max_k);

    let c3_for_thread = c3;
    let handle = std::thread::spawn(move || {
        bs_parallel(
            0,
            n as i64,
            &c3_for_thread,
            &counter_for_thread,
            &max_k_for_thread,
        )
    });

    // メインスレッドでカウンタをポーリングして進捗を報告
    let throttle = Duration::from_millis(100);
    loop {
        std::thread::sleep(throttle);
        let completed = counter.load(Ordering::Relaxed);
        let current = max_k.load(Ordering::Relaxed);

        on_progress(ChudnovskyProgress {
            estimated_digits: completed * 14,
            range: (0, current),
            message: None,
        });

        if handle.is_finished() {
            break;
        }
    }

    let res = handle
        .join()
        .map_err(|_| crate::error::HyperPiError::ThreadPanic)?;

    // 最終報告
    let completed = counter.load(Ordering::Relaxed);
    on_progress(ChudnovskyProgress {
        estimated_digits: completed * 14,
        range: (0, n),
        message: Some("done".to_string()),
    });

    // Context を使った最終計算フェーズ（精度変換を一括管理）
    let ctx = Context::<HalfEven>::new(precision);

    let sum_t = ctx.convert_int::<2>(res.t).value();
    let sum_q = ctx.convert_int::<2>(res.q).value();
    let c_float = ctx.convert_int::<2>(c).value();

    // √C の計算
    let c_sqrt = ctx.sqrt(&c_float.repr()).value();

    // 公式: Pi = (C * √C * Q_all) / (12 * T_all)
    let twelve = ctx.convert_int::<2>(IBig::from(12)).value();

    let c_times_csqrt = ctx.mul(&c_float.repr(), &c_sqrt.repr()).value();
    let numerator = ctx.mul(&c_times_csqrt.repr(), &sum_q.repr()).value();
    let denominator = ctx.mul(&twelve.repr(), &sum_t.repr()).value();

    let pi = ctx.div(&numerator.repr(), &denominator.repr()).value();

    // 完了通知
    on_progress(ChudnovskyProgress {
        estimated_digits: n * 14,
        range: (0, n),
        message: Some("Completed".to_string()),
    });

    Ok((pi, start.elapsed()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bs_first_term() {
        let c3 = IBig::from(640320) * IBig::from(640320) * IBig::from(640320);
        let counter = AtomicUsize::new(0);
        let max_k = AtomicUsize::new(0);
        let result = bs_parallel(0, 1, &c3, &counter, &max_k);

        assert_eq!(result.p, IBig::from(1));
        assert_eq!(result.q, IBig::from(1));
        assert_eq!(result.t, IBig::from(13591409));
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        assert_eq!(max_k.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_bs_second_term() {
        let c3 = IBig::from(640320) * IBig::from(640320) * IBig::from(640320);
        let counter = AtomicUsize::new(0);
        let max_k = AtomicUsize::new(0);
        let result = bs_parallel(1, 2, &c3, &counter, &max_k);

        assert_eq!(result.p, IBig::from(-5));
        assert!(result.q > IBig::from(0));
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        assert_eq!(max_k.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_calc_chudnovsky_basic() {
        let digits = 10;
        let (pi, _duration) = calc_chudnovsky(digits, |_| {}).unwrap();
        let three = BinFloat::from(3i32);
        let four = BinFloat::from(4i32);
        assert!(pi > three);
        assert!(pi < four);
    }

    #[test]
    fn test_calc_chudnovsky_precision() {
        let digits = 50;
        let precision = 256;
        let (pi, _duration) = calc_chudnovsky(digits, |_| {}).unwrap();
        let lower = (BinFloat::from(314i32) / BinFloat::from(100i32))
            .with_precision(precision)
            .value();
        let upper = (BinFloat::from(315i32) / BinFloat::from(100i32))
            .with_precision(precision)
            .value();
        assert!(pi > lower);
        assert!(pi < upper);
    }

    #[test]
    fn test_calc_chudnovsky_progress_callback() {
        let digits = 100;
        let mut messages_seen = vec![];
        let mut max_range = 0;

        let (_pi, _duration) = calc_chudnovsky(digits, |progress| {
            if let Some(ref msg) = progress.message {
                messages_seen.push(msg.clone());
            }
            if progress.range.1 > max_range {
                max_range = progress.range.1;
            }
        })
        .unwrap();

        assert!(messages_seen.contains(&"Initializing...".to_string()));
        assert!(messages_seen.contains(&"Completed".to_string()));
        assert!(max_range > 0);
    }
}
