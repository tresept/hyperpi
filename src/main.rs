use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};

type BinFloat = FBig<HalfEven, 2>;

fn main() -> std::io::Result<()> {
    let digits = 1_048_576;
    let filename = "pi.txt";

    // 必要なビット精度を計算: 桁数 * log2(10) + 誤差補正
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
    write_to_file(filename, &pi_str)?;
    eprintln!("結果を {} に保存しました", filename);

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
    // ほしい桁数分の整数部 + 小数点部に分ける
    let multiplier = IBig::from(10u8).pow(digits);

    // value * 10^digits を計算して整数に変換
    let value_int = (value * FBig::from(multiplier).with_precision(precision).value())
        .to_int()
        .value();

    // 整数を文字列に変換して小数点を挿入
    // 分割統治基数変換
    let value_str = value_int.to_string();

    // ここ気持ち悪いなぁ
    // TODO: 先頭が一桁で確定してるわけないから，計算結果の桁数に応じて小数点の位置を調整するようにしたい
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
