use dashu::base::Abs;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;

use std::time::{Duration, Instant};

/// 内部計算に使用する2進数浮動小数点数型
type BinFloat = FBig<HalfEven, 2>;

/// 2進数表現のFBigを10進数文字列に変換する
/// * value: 変換対象の浮動小数点数
/// * digits: 抽出する小数部の桁数
/// * precision: 内部計算に使用するビット精度
pub fn convert_to_decimal_string(
    value: &BinFloat,
    digits: usize,
    precision: usize,
) -> (String, Duration) {
    let start = Instant::now();

    // 整数部を抽出
    let integer_part_val = value.clone().trunc();
    let mut integer_part_int = integer_part_val.clone().to_int().value(); // IBig

    if digits == 0 {
        return (integer_part_int.to_string(), start.elapsed());
    }

    // 小数部のみを抽出
    let fractional_part = value - &integer_part_val;

    // 小数部を 10^digits 倍して整数化 (roundを使用)
    let multiplier = IBig::from(10u8).pow(digits);

    let scaled_fractional = (fractional_part
        * FBig::from(multiplier).with_precision(precision).value())
    .round()
    .to_int()
    .value()
    .abs(); // 絶対値を使用

    let s = scaled_fractional.to_string();

    // 桁上がり処理とゼロ埋め
    // scaled_fractional が 10^digits に達した場合 (例: 0.999... -> 1.000)
    // 文字列長が digits より大きくなる (例: digits=2, s="100")
    let decimal_str = if s.len() > digits {
        // 桁代わりが発生したため、整数部をインクリメント
        integer_part_int = integer_part_int + 1;
        // 小数部は先頭の '1' を除いた部分 (つまり "00...0")
        // s が "100" なら "00"
        if digits > 0 {
            s[1..].to_string()
        } else {
            String::new()
        }
    } else {
        // 桁代わりなし。必要なゼロ埋めを行う
        // format!マクロは巨大なwidthでパニックする可能性があるため、手動で構築
        if s.len() < digits {
            let pad_len = digits - s.len();
            let mut padded = String::with_capacity(digits);
            // push_str でまとめて埋めると効率的だが、ここではループでシンプルに
            for _ in 0..pad_len {
                padded.push('0');
            }
            padded.push_str(&s);
            padded
        } else {
            s
        }
    };

    let result = format!("{}.{}", integer_part_int, decimal_str);

    (result, start.elapsed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_decimal_string_basic() {
        let precision = 128;
        let pi = (BinFloat::from(314159i32) / BinFloat::from(100000i32))
            .with_precision(precision)
            .value(); // 3.14159
        let (result, _duration) = convert_to_decimal_string(&pi, 5, precision);
        assert_eq!(result, "3.14159");
    }

    #[test]
    fn test_convert_to_decimal_string_carry() {
        // 0.9999 -> 2桁 -> 1.00 の繰り上がりテスト
        let precision = 128;
        let val = (BinFloat::from(9999i32) / BinFloat::from(10000i32))
            .with_precision(precision)
            .value(); // 0.9999
        let (result, _duration) = convert_to_decimal_string(&val, 2, precision);
        assert_eq!(result, "1.00");
    }

    #[test]
    fn test_convert_to_decimal_string_digit_count() {
        let precision = 256;
        let pi = (BinFloat::from(314159265358979i64) / BinFloat::from(100000000000000i64))
            .with_precision(precision)
            .value();
        let test_cases = vec![5, 10];
        for digits in test_cases {
            let (result, _duration) = convert_to_decimal_string(&pi, digits, precision);
            let parts: Vec<&str> = result.split('.').collect();
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[1].len(), digits);
        }
    }

    #[test]
    fn test_convert_to_decimal_string_small_value() {
        let precision = 128;
        let value = (BinFloat::from(123456i32) / BinFloat::from(1000000i32))
            .with_precision(precision)
            .value();
        let (result, _duration) = convert_to_decimal_string(&value, 6, precision);
        assert_eq!(result, "0.123456");
    }

    #[test]
    fn test_convert_to_decimal_string_leading_zeros() {
        let precision = 128;
        let value = (BinFloat::from(1234i32) / BinFloat::from(1000000i32))
            .with_precision(precision)
            .value();
        let (result, _duration) = convert_to_decimal_string(&value, 6, precision);
        assert_eq!(result, "0.001234");
    }

    #[test]
    fn test_convert_to_decimal_string_integer_part_multidigit() {
        let precision = 128;
        let value = (BinFloat::from(123456i32) / BinFloat::from(1000i32))
            .with_precision(precision)
            .value();
        let (result, _duration) = convert_to_decimal_string(&value, 3, precision);
        assert_eq!(result, "123.456");
    }

    #[test]
    fn test_convert_to_decimal_string_zero_digits() {
        let precision = 128;
        let value = (BinFloat::from(314159i32) / BinFloat::from(100000i32))
            .with_precision(precision)
            .value();
        let (result, _duration) = convert_to_decimal_string(&value, 0, precision);
        assert_eq!(result, "3");
    }
}
