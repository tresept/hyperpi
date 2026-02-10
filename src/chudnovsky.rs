use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;

use std::time::{Duration, Instant};

/// 内部計算に使用する2進数浮動小数点数型
type BinFloat = FBig<HalfEven, 2>;

/// Chudnovsky法の進捗状況を保持する構造体
#[derive(Debug, Clone)]
pub struct ChudnovskyProgress {
    /// 推定確定桁数
    pub estimated_digits: usize,
    /// 現在処理中の k 範囲 [a, b)
    pub range: (Option<i64>, Option<i64>),
    /// 追加のカスタムメッセージ
    pub message: Option<String>,
}

/// Binary Splitting法で使用する中間項を保持する構造体
/// Chudnovskyの公式の各項を P, Q, R に分解して計算する
struct Pqr {
    p: IBig,
    q: IBig,
    r: IBig,
}

/// 進捗情報を収集し、スロットリング（頻度制限）を行いながら報告する
struct ProgressCollector<'a, F>
where
    F: FnMut(ChudnovskyProgress),
{
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
            total_terms,
            completed: 0,
            last_report: now,
            on_progress,
            throttle_ms,
        }
    }

    /// 最小単位の計算（葉ノード）が完了したときに呼ばれる
    fn leaf_done(&mut self, range: (Option<i64>, Option<i64>)) {
        self.completed += 1;
        let now = Instant::now();

        // 指定されたミリ秒以上経過しているか、全項完了した場合のみ報告する
        let should_report = now.duration_since(self.last_report)
            > Duration::from_millis(self.throttle_ms)
            || self.completed == self.total_terms;

        if should_report {
            self.last_report = now;
            let progress = ChudnovskyProgress {
                estimated_digits: self.completed * 14, // Chudnovsky法は1項あたり約14桁進む
                range,
                message: None,
            };
            (self.on_progress)(progress);
        }
    }

    /// 最終的な完了報告を行う
    fn final_report(&mut self) {
        let progress = ChudnovskyProgress {
            estimated_digits: self.completed * 14,
            range: (Some(0), Some(self.total_terms as i64)),
            message: Some("done".to_string()),
        };
        (self.on_progress)(progress);
    }
}

/// Binary Splitting法を並列で実行し、Chudnovskyの項を計算する
/// * a, b: 計算範囲 [a, b)
/// * c3: 定数 C^3
/// * sender: 進捗状況を送信するためのチャネル
fn bs_parallel(
    a: i64,
    b: i64,
    c3: &IBig,
    sender: Option<&std::sync::mpsc::Sender<(Option<i64>, Option<i64>)>>,
) -> Pqr {
    // 基底ケース: 範囲が1の場合は直接計算する
    if b - a == 1 {
        let k = IBig::from(a);
        let a_val = IBig::from(13591409);
        let b_val = IBig::from(545140134);

        if a == 0 {
            if let Some(s) = sender {
                let _ = s.send((Some(a), Some(b)));
            }
            return Pqr {
                p: IBig::from(1),
                q: a_val,
                r: IBig::from(1),
            };
        }

        // P(k) = -(6k-5)(2k-1)(6k-1)
        let p: IBig =
            (IBig::from(6) * &k - 5) * (IBig::from(2) * &k - 1) * (IBig::from(6) * &k - 1) * -1;
        // R(k) = k^3 * C^3 / 24
        let r = k.pow(3) * c3 / 24;
        // Q(k) = P(k) * (A + Bk)
        let q = &p * (a_val + b_val * &k);

        if let Some(s) = sender {
            let _ = s.send((Some(a), Some(b)));
        }

        Pqr { p, q, r }
    } else {
        // 並列化のしきい値。これより小さい範囲は逐次処理する
        if b - a < 500 {
            let m = (a + b) / 2;
            let left = bs_parallel(a, m, c3, sender);
            let right = bs_parallel(m, b, c3, sender);

            // 項の結合
            return Pqr {
                q: &left.q * &right.r + &left.p * &right.q,
                p: &left.p * &right.p,
                r: &left.r * &right.r,
            };
        }

        let m = (a + b) / 2;
        // 左右の範囲を rayon を用いて並列に計算する
        let (left, right) = rayon::join(
            || bs_parallel(a, m, c3, sender),
            || bs_parallel(m, b, c3, sender),
        );

        Pqr {
            q: &left.q * &right.r + &left.p * &right.q,
            p: &left.p * &right.p,
            r: &left.r * &right.r,
        }
    }
}

/// Chudnovsky法を用いて円周率を高精度に計算する
/// * digits: 求めたい10進数の桁数
/// * precision: 内部計算に使用するビット精度
/// * on_progress: 進捗状況を受け取るコールバック関数
pub fn calc_chudnovsky<F>(
    digits: usize,
    precision: usize,
    mut on_progress: F,
) -> crate::error::Result<(BinFloat, Duration)>
where
    F: FnMut(ChudnovskyProgress),
{
    let start = Instant::now();

    // 初期化フェーズの通知
    on_progress(ChudnovskyProgress {
        estimated_digits: 0,
        range: (None, None),
        message: Some("Initializing...".to_string()),
    });

    // 必要な項数を算出 (1項あたり約14.18桁)
    let n = digits / 14 + 1;

    let c = IBig::from(640320);
    let c3 = &c * &c * &c;

    // Binary Splitting（チャネル経由でプログレスを受け取りつつ並列実行）
    let res = {
        use std::sync::mpsc;
        let (sender, receiver) = mpsc::channel::<(Option<i64>, Option<i64>)>();

        let c3_for_thread = c3.clone();
        let handle = std::thread::spawn(move || {
            let result = bs_parallel(0, n as i64, &c3_for_thread, Some(&sender));
            drop(sender); // 送信側を明示的にドロップして受信ループを終わらせる
            result
        });

        // メインスレッドで進捗を受信してコールバックを実行
        let mut collector = ProgressCollector::new(n, &mut on_progress, 100);
        for range in receiver {
            collector.leaf_done(range);
        }
        collector.final_report();

        handle.join().map_err(|_| crate::error::HyperPiError::ThreadPanic)?
    };

    // 計算結果の結合と定数を用いた最終計算
    let sum_num_float = BinFloat::from(res.q).with_precision(precision).value();
    let sum_den_float = BinFloat::from(res.r).with_precision(precision).value();
    let c_float = BinFloat::from(c.clone()).with_precision(precision).value();

    // √C の計算
    let c_sqrt = c_float.sqrt().with_precision(precision).value();

    // 公式: Pi = (C * √C * sum_den) / (12 * sum_num)
    let twelve = BinFloat::from(12u8).with_precision(precision).value();
    let numerator = (&c_float * &c_sqrt * &sum_den_float)
        .with_precision(precision)
        .value();
    let denominator = (&twelve * &sum_num_float).with_precision(precision).value();

    let pi = (&numerator / &denominator)
        .with_precision(precision)
        .value();

    // 完了通知
    on_progress(ChudnovskyProgress {
        estimated_digits: n * 14,
        range: (Some(0), Some(n as i64)),
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
        let result = bs_parallel(0, 1, &c3, None);

        assert_eq!(result.p, IBig::from(1));
        assert_eq!(result.q, IBig::from(13591409));
        assert_eq!(result.r, IBig::from(1));
    }

    #[test]
    fn test_bs_second_term() {
        let c3 = IBig::from(640320) * IBig::from(640320) * IBig::from(640320);
        let result = bs_parallel(1, 2, &c3, None);

        assert_eq!(result.p, IBig::from(-5));
        assert!(result.r > IBig::from(0));
    }

    #[test]
    fn test_calc_chudnovsky_basic() {
        let digits = 10;
        let precision = 128;

        let (pi, _duration) = calc_chudnovsky(digits, precision, |_| {}).unwrap();

        let three = BinFloat::from(3i32);
        let four = BinFloat::from(4i32);
        assert!(pi > three);
        assert!(pi < four);
    }

    #[test]
    fn test_calc_chudnovsky_precision() {
        let digits = 50;
        let precision = 256;

        let (pi, _duration) = calc_chudnovsky(digits, precision, |_| {}).unwrap();

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
            let (pi, _duration) = calc_chudnovsky(digits, precision, |_| {}).unwrap();

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
        }).unwrap();

        // 初期化と完了のメッセージが含まれていることを確認
        assert!(messages_seen.contains(&"Initializing...".to_string()));
        assert!(messages_seen.contains(&"Completed".to_string()));
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
