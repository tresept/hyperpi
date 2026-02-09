use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;

use std::time::{Duration, Instant};

type BinFloat = FBig<HalfEven, 2>;

/// Chudnovsky法のプログレス情報
#[derive(Debug, Clone)]
pub struct ChudnovskyProgress {
    /// 計算開始からの経過時間
    pub elapsed: Duration,
    /// 推定確定桁数
    pub estimated_digits: usize,
    /// 現在処理中の k 範囲 [a, b)
    pub range: (Option<i64>, Option<i64>),
    /// 追加のカスタムメッセージ
    pub message: Option<String>,
}

// 分割統治法でChudnovskyの各項を計算するための構造体
struct Pqr {
    p: IBig,
    q: IBig,
    r: IBig,
}

// ProgressCollector を借用する版（非スレッドセーフ、シングルスレッド向け）
// main と gauss_legendre と同じ FnMut スタイルに合わせるためにこちらを使います。
struct ProgressCollector<'a, F>
where
    F: FnMut(ChudnovskyProgress),
{
    start: Instant,
    total_terms: usize,
    completed: usize,
    last_report: Instant,
    on_progress: &'a mut F,
    throttle_ms: u64,
}

impl<'a, F> ProgressCollector<'a, F>
where
    F: FnMut(ChudnovskyProgress),
{
    fn new(total_terms: usize, on_progress: &'a mut F, throttle_ms: u64) -> Self {
        let now = Instant::now();
        ProgressCollector {
            start: now,
            total_terms,
            completed: 0,
            last_report: now,
            on_progress,
            throttle_ms,
        }
    }

    // 葉が完了したら呼ぶ（bs 内から呼ぶ）
    fn leaf_done(&mut self, range: (Option<i64>, Option<i64>)) {
        self.completed += 1;
        let now = Instant::now();

        let should_report = now.duration_since(self.last_report)
            > Duration::from_millis(self.throttle_ms)
            || self.completed == self.total_terms;

        if should_report {
            self.last_report = now;
            let progress = ChudnovskyProgress {
                elapsed: now.duration_since(self.start),
                estimated_digits: self.completed * 14,
                range,
                message: None,
            };
            (self.on_progress)(progress);
        }
    }

    // 最終報告
    fn final_report(&mut self) {
        let now = Instant::now();
        let progress = ChudnovskyProgress {
            elapsed: now.duration_since(self.start),
            estimated_digits: self.completed * 14,
            range: (Some(0), Some(self.total_terms as i64)),
            message: Some("done".to_string()),
        };
        (self.on_progress)(progress);
    }
}

/// 分割統治法でChudnovskyの各項を計算（プログレス付き）
fn bs_with_progress<F>(a: i64, b: i64, c3: &IBig, collector: &mut ProgressCollector<'_, F>) -> Pqr
where
    F: FnMut(ChudnovskyProgress),
{
    if b - a == 1 {
        let k = IBig::from(a);
        let a_val = IBig::from(13591409);
        let b_val = IBig::from(545140134);

        if a == 0 {
            let pqr = Pqr {
                p: IBig::from(1),
                q: a_val,
                r: IBig::from(1),
            };
            collector.leaf_done((Some(a), Some(b)));
            return pqr;
        }

        let p: IBig =
            (IBig::from(6) * &k - 5) * (IBig::from(2) * &k - 1) * (IBig::from(6) * &k - 1) * -1;
        let r = k.pow(3) * c3 / 24;
        let q = &p * (a_val + b_val * &k);

        let pqr = Pqr { p, q, r };
        collector.leaf_done((Some(a), Some(b)));
        pqr
    } else {
        let m = (a + b) / 2;
        let left = bs_with_progress(a, m, c3, collector);
        let right = bs_with_progress(m, b, c3, collector);

        // 合体ルール！
        Pqr {
            q: &left.q * &right.r + &left.p * &right.q,
            p: &left.p * &right.p,
            r: &left.r * &right.r,
        }
    }
}

/// 分割統治法でChudnovskyの各項を計算（テスト用、プログレスなし）
fn bs(a: i64, b: i64, c3: &IBig) -> Pqr {
    if b - a == 1 {
        let k = IBig::from(a);
        let a_val = IBig::from(13591409);
        let b_val = IBig::from(545140134);

        if a == 0 {
            return Pqr {
                p: IBig::from(1),
                q: a_val,
                r: IBig::from(1),
            };
        }

        let p: IBig =
            (IBig::from(6) * &k - 5) * (IBig::from(2) * &k - 1) * (IBig::from(6) * &k - 1) * -1;
        let r = k.pow(3) * c3 / 24;
        let q = &p * (a_val + b_val * &k);

        Pqr { p, q, r }
    } else {
        let m = (a + b) / 2;
        let left = bs(a, m, c3);
        let right = bs(m, b, c3);

        Pqr {
            q: &left.q * &right.r + &left.p * &right.q,
            p: &left.p * &right.p,
            r: &left.r * &right.r,
        }
    }
}

/// Chudnovsky法で円周率を計算
/// * digits: 求めたい10進数の桁数
/// * precision: 内部計算に使う2進数の精度（ビット数）
pub fn calc_chudnovsky<F>(
    digits: usize,
    precision: usize,
    mut on_progress: F,
) -> (BinFloat, Duration)
where
    F: FnMut(ChudnovskyProgress),
{
    let start = Instant::now();

    // 初期化フェーズを送る
    on_progress(ChudnovskyProgress {
        elapsed: start.elapsed(),
        estimated_digits: 0,
        range: (None, None),
        message: Some("初期化".to_string()),
    });

    // 必要な項数
    let n = digits / 14 + 1;

    let c = IBig::from(640320);
    let c3 = &c * &c * &c;

    // Binary Splitting（プログレス付き）
    let res = {
        let mut collector = ProgressCollector::new(n, &mut on_progress, 100);
        let res = bs_with_progress(0, n as i64, &c3, &mut collector);
        collector.final_report();
        res
    };
    // ↑ ブロックを抜けると collector が drop され、on_progress の借用が解放される

    // 高精度な浮動小数点数に変換
    let sum_num_float = BinFloat::from(res.q).with_precision(precision).value();
    let sum_den_float = BinFloat::from(res.r).with_precision(precision).value();
    let c_float = BinFloat::from(c.clone()).with_precision(precision).value();

    // 高精度な √C を求める
    let c_sqrt = c_float.sqrt().with_precision(precision).value();

    // Pi = (C * √C * sum_den) / (12 * sum_num)
    let twelve = BinFloat::from(12u8).with_precision(precision).value();
    let numerator = (&c_float * &c_sqrt * &sum_den_float)
        .with_precision(precision)
        .value();
    let denominator = (&twelve * &sum_num_float).with_precision(precision).value();

    let pi = (&numerator / &denominator)
        .with_precision(precision)
        .value();

    // 完了フェーズを送る
    on_progress(ChudnovskyProgress {
        elapsed: start.elapsed(),
        estimated_digits: n * 14,
        range: (Some(0), Some(n as i64)),
        message: Some("完了".to_string()),
    });

    (pi, start.elapsed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bs_first_term() {
        let c3 = IBig::from(640320) * IBig::from(640320) * IBig::from(640320);
        let result = bs(0, 1, &c3);

        assert_eq!(result.p, IBig::from(1));
        assert_eq!(result.q, IBig::from(13591409));
        assert_eq!(result.r, IBig::from(1));
    }

    #[test]
    fn test_bs_second_term() {
        let c3 = IBig::from(640320) * IBig::from(640320) * IBig::from(640320);
        let result = bs(1, 2, &c3);

        assert_eq!(result.p, IBig::from(-5));
        assert!(result.r > IBig::from(0));
    }

    #[test]
    fn test_calc_chudnovsky_basic() {
        let digits = 10;
        let precision = 128;

        let (pi, _duration) = calc_chudnovsky(digits, precision, |_| {});

        let three = BinFloat::from(3i32);
        let four = BinFloat::from(4i32);
        assert!(pi > three);
        assert!(pi < four);
    }

    #[test]
    fn test_calc_chudnovsky_precision() {
        let digits = 50;
        let precision = 256;

        let (pi, _duration) = calc_chudnovsky(digits, precision, |_| {});

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
    fn test_calc_chudnovsky_different_digits() {
        let test_cases = vec![5, 10, 20, 30];

        for digits in test_cases {
            let precision = digits * 4;
            let (pi, _duration) = calc_chudnovsky(digits, precision, |_| {});

            let three = BinFloat::from(3i32);
            let three_point_two = (BinFloat::from(32i32) / BinFloat::from(10i32))
                .with_precision(precision)
                .value();
            assert!(pi > three);
            assert!(pi < three_point_two);
        }
    }

    #[test]
    fn test_calc_chudnovsky_progress_callback() {
        let digits = 100;
        let precision = 512;
        let mut messages_seen = vec![];

        let (_pi, _duration) = calc_chudnovsky(digits, precision, |progress| {
            if let Some(ref msg) = progress.message {
                messages_seen.push(msg.clone());
            }
        });

        // 初期化と完了のメッセージが含まれていることを確認
        assert!(messages_seen.contains(&"初期化".to_string()));
        assert!(messages_seen.contains(&"完了".to_string()));
    }

    #[test]
    fn test_pqr_structure() {
        let pqr = Pqr {
            p: IBig::from(1),
            q: IBig::from(2),
            r: IBig::from(3),
        };

        assert_eq!(pqr.p, IBig::from(1));
        assert_eq!(pqr.q, IBig::from(2));
        assert_eq!(pqr.r, IBig::from(3));
    }
}
