use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};

use std::time::{Duration, Instant};

type BinFloat = FBig<HalfEven, 2>;

/// Gauss-Legendre法のプログレス情報
#[derive(Debug, Clone)]
pub struct GaussLegendreProgress {
    /// 現在の反復回数（0が初期化）
    pub iteration: u32,
    /// 総反復回数
    pub total_iterations: u32,
    /// 計算開始からの経過時間
    pub elapsed: Duration,
    /// 現在のフェーズ（"初期化", "計算中", "完了"など）
    pub phase: &'static str,
    /// 追加のカスタムメッセージ
    pub message: Option<String>,
}

/// Gauss-Legendre法で円周率を計算
///
/// アルゴリズム: https://ja.wikipedia.org/wiki/ガウス＝ルジャンドルのアルゴリズム
/// 参考: https://qiita.com/matsuda_tkm/items/418588d3c59cc8d85ec7
pub fn calc_gauss_legendre<F>(precision: usize, mut on_progress: F) -> (BinFloat, Duration)
where
    F: FnMut(GaussLegendreProgress),
{
    let start = Instant::now();

    // 2次収束なので log2(precision) 回繰り返せば十分
    let iterations = ((precision as f64).log2().ceil() as u32).max(10);

    on_progress(GaussLegendreProgress {
        iteration: 0,
        total_iterations: iterations,
        elapsed: start.elapsed(),
        phase: "初期化",
        message: None,
    });

    // 初期値を設定
    let one = BinFloat::ONE.with_precision(precision).value();
    let two = BinFloat::from(2u8).with_precision(precision).value();
    let four = BinFloat::from(4u8).with_precision(precision).value();

    let mut a = one.clone();
    let mut b = (&one / &two.sqrt()).with_precision(precision).value();
    let mut t = &one / &four;
    let mut p = one.clone();

    on_progress(GaussLegendreProgress {
        iteration: 0,
        total_iterations: iterations,
        elapsed: start.elapsed(),
        phase: "計算中",
        message: Some("初期値の計算が完了しました".to_string()),
    });

    for i in 0..iterations {
        let a_next = ((&a + &b) / &two).with_precision(precision).value();
        let b_next = (&a * &b).sqrt().with_precision(precision).value();
        let a_diff = &a - &a_next;
        let t_next = &t - &(&p * &a_diff * &a_diff);

        a = a_next;
        b = b_next;
        t = t_next;
        p = (&p * &two).with_precision(precision).value();

        on_progress(GaussLegendreProgress {
            iteration: i + 1,
            total_iterations: iterations,
            elapsed: start.elapsed(),
            phase: "計算中",
            message: None,
        });
    }

    // π = (a + b)^2 / (4t)
    let sum = &a + &b;
    let numerator = &sum * &sum;
    let denominator = &four * &t;
    let pi = (&numerator / &denominator)
        .with_precision(precision)
        .value();

    on_progress(GaussLegendreProgress {
        iteration: iterations,
        total_iterations: iterations,
        elapsed: start.elapsed(),
        phase: "完了",
        message: None,
    });

    (pi, start.elapsed())
}
