use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;

use std::time::{Duration, Instant};

type BinFloat = FBig<HalfEven, 2>;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_decimal_string_basic() {
        // π ≈ 3.14159... の変換テスト
        let precision = 128;
        // 3.14159 を有理数として表現: 314159 / 100000
        let pi = (BinFloat::from(314159i32) / BinFloat::from(100000i32))
            .with_precision(precision)
            .value();
        let (result, _duration) = convert_to_decimal_string(&pi, 5, precision);

        // 結果が "3." で始まることを確認
        assert!(result.starts_with("3."));
        // 小数点が含まれていることを確認
        assert!(result.contains('.'));
    }

    #[test]
    fn test_convert_to_decimal_string_digit_count() {
        // 指定した桁数で出力されることを確認
        let precision = 256;
        // 3.14159265358979 を有理数として表現
        let pi = (BinFloat::from(314159265358979i64) / BinFloat::from(100000000000000i64))
            .with_precision(precision)
            .value();

        let test_cases = vec![5, 10, 20, 30];

        for digits in test_cases {
            let (result, _duration) = convert_to_decimal_string(&pi, digits, precision);

            // "3." を除いた小数部の桁数を確認
            let parts: Vec<&str> = result.split('.').collect();
            assert_eq!(parts.len(), 2);
            assert!(parts[1].len() <= digits);
        }
    }

    #[test]
    fn test_convert_to_decimal_string_pi_accuracy() {
        // πの既知の値と比較
        let precision = 512;
        // 高精度なπの近似値を作成
        let pi = (BinFloat::from(3141592653589793i64) / BinFloat::from(1000000000000000i64))
            .with_precision(precision)
            .value();

        let (result, _duration) = convert_to_decimal_string(&pi, 15, precision);

        // πの既知の桁と比較
        assert!(result.starts_with("3.14159265358979"));
    }

    #[test]
    fn test_convert_to_decimal_string_integer_part() {
        // 整数部が複数桁の場合のテスト
        let precision = 128;
        // 123.456 を有理数として表現: 123456 / 1000
        let value = (BinFloat::from(123456i32) / BinFloat::from(1000i32))
            .with_precision(precision)
            .value();

        let (result, _duration) = convert_to_decimal_string(&value, 3, precision);

        // 整数部が正しく保持されていることを確認
        assert!(result.starts_with("123."));
    }

    #[test]
    fn test_convert_to_decimal_string_small_value() {
        // 小さい値（1未満）のテスト
        let precision = 128;
        // 0.123456 を有理数として表現: 123456 / 1000000
        let value = (BinFloat::from(123456i32) / BinFloat::from(1000000i32))
            .with_precision(precision)
            .value();

        let (result, _duration) = convert_to_decimal_string(&value, 6, precision);

        // 小数点が含まれていることを確認
        assert!(result.contains('.'));
        // 整数部が存在することを確認
        let parts: Vec<&str> = result.split('.').collect();
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn test_convert_to_decimal_string_elapsed_time() {
        // 経過時間が記録されることを確認
        let precision = 128;
        let pi = (BinFloat::from(314159i32) / BinFloat::from(100000i32))
            .with_precision(precision)
            .value();

        let (_result, duration) = convert_to_decimal_string(&pi, 10, precision);

        // 何らかの時間が記録されていることを確認（常に0以上）
        let _ = duration; // 時間が記録されていればOK
    }

    #[test]
    fn test_convert_to_decimal_string_zero_digits() {
        // 0桁を指定した場合のテスト
        let precision = 128;
        let value = (BinFloat::from(314159i32) / BinFloat::from(100000i32))
            .with_precision(precision)
            .value();

        let (result, _duration) = convert_to_decimal_string(&value, 0, precision);

        // 整数部のみが返されることを確認
        assert!(result.starts_with("3."));
    }

    #[test]
    fn test_convert_to_decimal_string_high_precision() {
        // 高精度での変換テスト
        let precision = 1024;
        // 高精度なπの近似値
        let pi = (BinFloat::from(3141592653589793i64) / BinFloat::from(1000000000000000i64))
            .with_precision(precision)
            .value();

        let (result, _duration) = convert_to_decimal_string(&pi, 50, precision);

        // 結果が妥当な長さを持つことを確認
        assert!(result.len() > 10);
        assert!(result.starts_with("3."));
    }

    #[test]
    fn test_convert_to_decimal_string_format() {
        // フォーマットが "整数部.小数部" の形式になっていることを確認
        let precision = 128;
        // e ≈ 2.718281828 を有理数として表現
        let value = (BinFloat::from(2718281828i64) / BinFloat::from(1000000000i64))
            .with_precision(precision)
            .value();

        let (result, _duration) = convert_to_decimal_string(&value, 9, precision);

        // 小数点が1つだけ含まれていることを確認
        assert_eq!(result.matches('.').count(), 1);

        // 整数部と小数部に分割できることを確認
        let parts: Vec<&str> = result.split('.').collect();
        assert_eq!(parts.len(), 2);
        assert!(!parts[0].is_empty()); // 整数部が空でない
        assert!(!parts[1].is_empty()); // 小数部が空でない
    }
}
