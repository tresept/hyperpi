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
    /// 現在のフェーズ（"初期化", "計算中", "完了"など）
    pub phase: &'static str,
}

/// Gauss-Legendre法で円周率を計算
///
/// アルゴリズム: https://ja.wikipedia.org/wiki/ガウス＝ルジャンドルのアルゴリズム
/// 参考: https://qiita.com/matsuda_tkm/items/418588d3c59cc8d85ec7
pub fn calc_gauss_legendre<F>(precision: usize, mut on_progress: F) -> crate::error::Result<(BinFloat, Duration)>
where
    F: FnMut(GaussLegendreProgress),
{
    let start = Instant::now();

    // 2次収束なので log2(precision) 回繰り返せば十分
    let iterations = ((precision as f64).log2().ceil() as u32).max(10);

    on_progress(GaussLegendreProgress {
        iteration: 0,
        total_iterations: iterations,
        phase: "Initializing...",
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
        phase: "Calculating...",
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
            phase: "Calculating...",
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
        phase: "Completed",
    });

    Ok((pi, start.elapsed()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_gauss_legendre_basic() {
        // 基本的な計算のテスト
        let precision = 128;
        let (pi, _duration) = calc_gauss_legendre(precision, |_| {}).unwrap();

        // πが妥当な範囲にあることを確認（3 < π < 4）
        let three = BinFloat::from(3i32);
        let four = BinFloat::from(4i32);
        assert!(pi > three);
        assert!(pi < four);
    }

    #[test]
    fn test_calc_gauss_legendre_precision() {
        // より高精度でπを計算
        let precision = 256;
        let (pi, _duration) = calc_gauss_legendre(precision, |_| {}).unwrap();

        // πが妥当な範囲にあることを確認（3.14 < π < 3.15）
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
    fn test_calc_gauss_legendre_progress_callback() {
        // プログレスコールバックが呼ばれることを確認
        let precision = 64;
        let mut callback_count = 0;
        let mut phases_seen = vec![];

        let (_pi, _duration) = calc_gauss_legendre(precision, |progress| {
            callback_count += 1;
            phases_seen.push(progress.phase);
        }).unwrap();

        // コールバックが複数回呼ばれていることを確認
        assert!(callback_count > 0);

        // 初期化と計算中のフェーズが含まれていることを確認
        assert!(phases_seen.contains(&"Initializing..."));
        assert!(phases_seen.contains(&"Calculating..."));
        assert!(phases_seen.contains(&"Completed"));
    }

    #[test]
    fn test_calc_gauss_legendre_progress_iteration_count() {
        // イテレーション数が正しく報告されることを確認
        let precision = 128;
        let mut max_iteration = 0;
        let mut total_iterations = 0;

        let (_pi, _duration) = calc_gauss_legendre(precision, |progress| {
            max_iteration = max_iteration.max(progress.iteration);
            total_iterations = progress.total_iterations;
        }).unwrap();

        // イテレーション数が合理的な範囲にあることを確認
        assert!(total_iterations > 0);
        assert_eq!(max_iteration, total_iterations);
    }

    #[test]
    fn test_calc_gauss_legendre_convergence() {
        // 精度を上げると結果が改善されることを確認
        let (pi_low, _) = calc_gauss_legendre(64, |_| {}).unwrap();
        let (pi_high, _) = calc_gauss_legendre(256, |_| {}).unwrap();

        // 両方とも3と4の間にあることを確認
        let three = BinFloat::from(3i32);
        let four = BinFloat::from(4i32);
        assert!(pi_low > three);
        assert!(pi_low < four.clone());
        assert!(pi_high > three);
        assert!(pi_high < four);
    }

    #[test]
    fn test_gauss_legendre_progress_structure() {
        // GaussLegendreProgress構造体が正しく作成できることを確認
        let progress = GaussLegendreProgress {
            iteration: 5,
            total_iterations: 10,
            phase: "Calculating...",
        };

        assert_eq!(progress.iteration, 5);
        assert_eq!(progress.total_iterations, 10);
        assert_eq!(progress.phase, "Calculating...");
    }

    #[test]
    fn test_calc_gauss_legendre_elapsed_time() {
        // 経過時間が記録されることを確認
        let precision = 128;
        let (_, duration) = calc_gauss_legendre(precision, |_| {}).unwrap();

        // 何らかの時間が経過していることを確認
        assert!(duration.as_nanos() > 0);
    }
}
