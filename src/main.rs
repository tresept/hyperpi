use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};

type BinFloat = FBig<HalfEven, 2>;

fn main() -> std::io::Result<()> {
    let digits = 1_000_000;

    println!("=== 円周率計算 ({} 桁) ===\n", digits);

    // 必要なビット精度を計算: 桁数 * log2(10) + 誤差補正
    let precision = (digits as f64 * 10.0_f64.log2() + 128.0) as usize;

    // 円周率を計算（2進数）
    let (pi_bin, calc_time) = calculate_pi(precision);
    println!("計算時間: {:?}", calc_time);

    // 10進数文字列に変換
    let (pi_str, conv_time) = convert_to_decimal_string(&pi_bin, digits, precision);
    println!("変換時間: {:?}", conv_time);
    println!("合計時間: {:?}\n", calc_time + conv_time);

    // ファイルに保存
    write_to_file("pi.txt", &pi_str)?;
    println!("結果を pi.txt に保存しました");

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
fn convert_to_decimal_string(
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

/// Gauss-Legendre法で円周率を計算
///
/// アルゴリズム: https://ja.wikipedia.org/wiki/ガウス＝ルジャンドルのアルゴリズム
/// 参考: https://qiita.com/matsuda_tkm/items/418588d3c59cc8d85ec7
fn calculate_pi(precision: usize) -> (BinFloat, Duration) {
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

        eprintln!("  第 {} 回反復 - 経過時間: {:?}", i + 1, start.elapsed());
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
