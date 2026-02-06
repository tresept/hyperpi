use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};

type BinFloat = FBig<HalfEven, 2>;
type DecFloat = FBig<HalfEven, 10>;

fn main() -> std::io::Result<()> {
    let digits = 1_040_000;

    println!("=== 円周率計算ベンチマーク ({} 桁) ===\n", digits);

    // 2進数版の計算
    println!("【2進数版】");
    let precision_bin = (digits as f64 * 10.0_f64.log2() + 128.0) as usize;
    let (pi_bin, bin_calc_time) = calculate_pi_binary(precision_bin);
    let (pi_bin_str, bin_conv_time) = convert_bin_to_decimal_string(&pi_bin, digits, precision_bin);

    println!("  計算時間: {:?}", bin_calc_time);
    println!("  変換時間: {:?}", bin_conv_time);
    println!("  合計時間: {:?}\n", bin_calc_time + bin_conv_time);

    // 10進数版の計算
    println!("【10進数版】");
    let precision_dec = digits + 100;
    let (pi_dec, dec_calc_time) = calculate_pi_decimal(precision_dec);
    let pi_dec_str = pi_dec.to_string();

    println!("  計算時間: {:?}", dec_calc_time);
    println!("  合計時間: {:?}\n", dec_calc_time);

    // ファイルに保存
    write_to_file("pi_bin.txt", &pi_bin_str)?;
    write_to_file("pi_dec.txt", &pi_dec_str)?;

    println!("結果をファイルに保存しました:");
    println!("  - pi_bin.txt (2進数版)");
    println!("  - pi_dec.txt (10進数版)");

    Ok(())
}

/// ファイルに文字列を書き込む
fn write_to_file(filename: &str, content: &str) -> std::io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", content)?;
    Ok(())
}

/// 2進数表現のFBigを10進数文字列に変換する
fn convert_bin_to_decimal_string(
    value: &BinFloat,
    digits: usize,
    precision: usize,
) -> (String, Duration) {
    let start = Instant::now();

    // 10^digits を掛けて整数化する
    let multiplier = IBig::from(10u8).pow(digits);
    let value_int = (value * FBig::from(multiplier).with_precision(precision).value())
        .to_int()
        .value();

    // 整数を文字列に変換して小数点を挿入
    let value_str = value_int.to_string();
    let (first, rest) = value_str.split_at(1);
    let result = format!("{}.{}", first, rest);

    (result, start.elapsed())
}

/// Gauss-Legendre法で円周率を計算（2進数版）
///
/// 2進数で計算するため、内部演算は高速だが、
/// 最後に10進数に変換するコストがかかる
///
/// アルゴリズム: https://ja.wikipedia.org/wiki/ガウス＝ルジャンドルのアルゴリズム
fn calculate_pi_binary(precision: usize) -> (BinFloat, Duration) {
    let start = Instant::now();

    // 初期値を設定
    let one = BinFloat::ONE.with_precision(precision).value();
    let two = BinFloat::from(2u8).with_precision(precision).value();
    let four = BinFloat::from(4u8).with_precision(precision).value();

    let mut a = one.clone();
    let mut b = (&one / &two.sqrt()).with_precision(precision).value();
    let mut t = &one / &four;
    let mut p = one.clone();

    // 2次収束なので log2(precision) 回繰り返せば十分
    let iterations = ((precision as f64).log2().ceil() as u32).max(10);

    for i in 0..iterations {
        let a_next = ((&a + &b) / &two).with_precision(precision).value();
        let b_next = (&a * &b).sqrt().with_precision(precision).value();
        let a_diff = &a - &a_next;
        let t_next = &t - &(&p * &a_diff * &a_diff);

        a = a_next;
        b = b_next;
        t = t_next;
        p = (&p * &two).with_precision(precision).value();

        eprintln!(
            "  [BIN] 第 {} 回反復 - 経過時間: {:?}",
            i + 1,
            start.elapsed()
        );
    }

    // π = (a + b)² / (4t)
    let sum = &a + &b;
    let numerator = &sum * &sum;
    let denominator = &four * &t;
    let pi = (&numerator / &denominator)
        .with_precision(precision)
        .value();

    (pi, start.elapsed())
}

/// Gauss-Legendre法で円周率を計算（10進数版）
///
/// 10進数で直接計算するため、変換コストは不要だが、
/// 内部演算が2進数版より遅い
///
/// アルゴリズム: https://ja.wikipedia.org/wiki/ガウス＝ルジャンドルのアルゴリズム
fn calculate_pi_decimal(precision: usize) -> (DecFloat, Duration) {
    let start = Instant::now();

    // 初期値を設定
    let one = DecFloat::ONE.with_precision(precision).value();
    let two = DecFloat::from(2u8).with_precision(precision).value();
    let four = DecFloat::from(4u8).with_precision(precision).value();

    let mut a = one.clone();
    let mut b = (&one / &two.sqrt()).with_precision(precision).value();
    let mut t = &one / &four;
    let mut p = one.clone();

    // 2次収束なので log2(precision) 回繰り返せば十分
    let iterations = ((precision as f64).log2().ceil() as u32).max(10);

    for i in 0..iterations {
        let a_next = ((&a + &b) / &two).with_precision(precision).value();
        let b_next = (&a * &b).sqrt().with_precision(precision).value();
        let a_diff = &a - &a_next;
        let t_next = &t - &(&p * &a_diff * &a_diff);

        a = a_next;
        b = b_next;
        t = t_next;
        p = (&p * &two).with_precision(precision).value();

        eprintln!(
            "  [DEC] 第 {} 回反復 - 経過時間: {:?}",
            i + 1,
            start.elapsed()
        );
    }

    // π = (a + b)² / (4t)
    let sum = &a + &b;
    let numerator = &sum * &sum;
    let denominator = &four * &t;
    let pi = (&numerator / &denominator)
        .with_precision(precision)
        .value();

    (pi, start.elapsed())
}
