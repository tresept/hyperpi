use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};

type BinFloat = FBig<HalfEven, 2>;

fn main() -> std::io::Result<()> {
    // let digits = 1_048_576;
    let digits = 4;
    let filename = "pi.txt";

    // 必要なビット精度: 桁数 * log2(10) + 誤差補正
    let precision = (digits as f64 * 10.0_f64.log2() + 128.0) as usize;

    eprintln!("ただいまより {} 桁の円周率を計算します\n", digits);

    // 円周率を計算（2進数）
    let (pi_bin, calc_time) = calculate_pi(precision);
    eprintln!("計算を {:?} で完了しました", calc_time);

    // 10進数文字列に変換
    let (pi_str, conv_time) = convert_to_decimal_string(&pi_bin, digits, precision);
    eprintln!("変換を {:?} で完了しました", conv_time);
    eprintln!("合計時間: {:?}\n", calc_time + conv_time);

    // ファイルに保存
    let start_io = Instant::now();
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", pi_str)?;
    eprintln!("ファイル書き込みを {:?} で完了しました", start_io.elapsed());

    Ok(())
}

/// 2進数表現のFBigを10進数文字列に変換する
fn convert_to_decimal_string(
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

    eprintln!("初期値の計算が完了しました: {:?}", start.elapsed());

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
            "第 {} 回のループが完了しました: {:?}",
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
