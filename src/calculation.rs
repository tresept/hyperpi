use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;

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

/// 2進数表現のFBigを10進数文字列に変換する
pub fn convert_to_decimal_string(
    value: &BinFloat,
    digits: usize,
    precision: usize,
) -> (String, Duration) {
    let start = Instant::now();

    // 整数部が何桁あるかを計算する
    // 整数部だけIBigにすればいいらしい
    let integer_part = value.clone().trunc().to_int().value();
    let integer_str = integer_part.to_string();
    let int_len = integer_str.len();

    // 10^digits を掛けて整数化する
    // ほしい桁数分の整数部 + 小数点部に分ける
    let multiplier = IBig::from(10u8).pow(digits);
    let scaled_value = (value * FBig::from(multiplier).with_precision(precision).value())
        .trunc()
        .to_int()
        .value();

    let full_str = scaled_value.to_string();

    // 整数部の桁数で分割してから、小数部から必要な桁数だけ取り出す
    let (integer_part_str, decimal_part_full) = full_str.split_at(int_len);
    let decimal_part = if decimal_part_full.len() > digits {
        &decimal_part_full[..digits]
    } else {
        decimal_part_full
    };

    let result = format!("{}.{}", integer_part_str, decimal_part);

    (result, start.elapsed())
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

fn calc_chudnovsky() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gauss_legendre_10_digits() {
        // 10桁精度の円周率を計算
        let digits = 10;
        // 2進数精度は10進数の桁数 * log2(10) ≈ 桁数 * 3.32 + マージン
        let precision = (digits as f64 * 3.32 * 1.5) as usize;

        let (pi, _duration) = calc_gauss_legendre(precision, |_progress| {
            // テスト中はプログレス情報を無視
        });

        // 2進数表現を10進数文字列に変換
        let (pi_str, _convert_duration) = convert_to_decimal_string(&pi, digits, precision);

        // 期待される円周率の値（10桁）
        let expected = "3.1415926535";

        // 小数点以下10桁まで一致することを確認
        assert_eq!(
            &pi_str[..expected.len()],
            expected,
            "計算された円周率が期待値と一致しません\n計算値: {}\n期待値: {}",
            pi_str,
            expected
        );

        // 念のため、各桁を個別にチェック
        let pi_chars: Vec<char> = pi_str.chars().collect();
        let expected_chars: Vec<char> = expected.chars().collect();

        for (i, (calc_char, exp_char)) in pi_chars.iter().zip(expected_chars.iter()).enumerate() {
            assert_eq!(
                calc_char, exp_char,
                "{}桁目が一致しません: 計算値='{}', 期待値='{}'",
                i, calc_char, exp_char
            );
        }
    }

    #[test]
    fn test_gauss_legendre_progress_callback() {
        // プログレスコールバックが正しく呼ばれることを確認
        let precision = 100;
        let mut progress_count = 0;
        let mut last_iteration = 0;

        let (_pi, _duration) = calc_gauss_legendre(precision, |progress| {
            progress_count += 1;
            last_iteration = progress.iteration;

            // プログレス情報の妥当性チェック
            assert!(progress.iteration <= progress.total_iterations);
            assert!(
                progress.phase == "初期化"
                    || progress.phase == "計算中"
                    || progress.phase == "完了"
            );
        });

        // プログレスコールバックが少なくとも複数回呼ばれていることを確認
        assert!(
            progress_count > 1,
            "プログレスコールバックが呼ばれていません"
        );
        assert!(last_iteration > 0, "反復計算が実行されていません");
    }
}
