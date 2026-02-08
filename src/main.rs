use colorgrad::Gradient;
use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};

type BinFloat = FBig<HalfEven, 2>;

/// 1行のテキストにグラデーションを適用する関数
/// 虹色グラデーション（シアン → グリーン → イエロー → マゼンタ）
fn gradient_line(text: &str) -> String {
    let gradient = colorgrad::GradientBuilder::new()
        .colors(&[
            colorgrad::Color::from_rgba8(0, 255, 255, 255), // シアン
            colorgrad::Color::from_rgba8(143, 250, 171, 255), // グリーン
            colorgrad::Color::from_rgba8(250, 214, 77, 255), // イエロー
            colorgrad::Color::from_rgba8(250, 122, 205, 255), // マゼンタ
        ])
        .build::<colorgrad::LinearGradient>()
        .unwrap();

    let len = text.chars().count();
    if len == 0 {
        return String::new();
    }

    text.chars()
        .enumerate()
        .map(|(i, c)| {
            // 各文字の位置に応じてグラデーションの色を取得（0.0〜1.0）
            let t = if len == 1 {
                0.5
            } else {
                i as f32 / (len - 1) as f32
            };
            let color = gradient.at(t).to_rgba8();
            format!("{}", c.to_string().truecolor(color[0], color[1], color[2]))
        })
        .collect()
}

/// 複数行のテキストを行ごとにグラデーション適用する関数
fn gradient_text(text: &str) -> String {
    text.lines()
        .map(|line| gradient_line(line))
        .collect::<Vec<String>>()
        .join("\n")
}

fn logo() -> String {
    r#"
░█░█░█░█░█▀█░█▀▀░█▀▄░█▀█░▀█▀
░█▀█░░█░░█▀▀░█▀▀░█▀▄░█▀▀░░█░
░▀░▀░░▀░░▀░░░▀▀▀░▀░▀░▀░░░▀▀▀
"#
    .to_string()
}

fn main() -> std::io::Result<()> {
    eprintln!("{}", gradient_text(&logo()).bold());

    let digits = 1_048_576;

    let filename = "pi.txt";

    // 必要なビット精度: 桁数 * log2(10) + 誤差補正
    let precision = (digits as f64 * 10.0_f64.log2() + 128.0) as usize;

    eprintln!("ただいまより {} 桁の円周率を計算します\n", digits);

    // 円周率を計算（2進数）
    let (pi_bin, calc_time) = calculate_pi(precision);
    eprintln!("計算を {:.3}s で完了しました", calc_time.as_secs_f64());

    // 10進数文字列に変換
    let (pi_str, conv_time) = convert_to_decimal_string(&pi_bin, digits, precision);
    eprintln!("変換を {:.3}s で完了しました", conv_time.as_secs_f64());
    eprintln!("合計時間: {:.3}s\n", (calc_time + conv_time).as_secs_f64());

    // ファイルに保存
    let start_io = Instant::now();
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", pi_str)?;
    eprintln!(
        "ファイル書き込みを {:.3}s で完了しました",
        start_io.elapsed().as_secs_f64()
    );

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

    eprintln!(
        "初期値の計算が完了しました: {:.3}s",
        start.elapsed().as_secs_f64()
    );

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
            "第 {} 回のループが完了しました: {:.3}s",
            i + 1,
            start.elapsed().as_secs_f64()
        );
    }

    // π = (a + b)^2 / (4t)
    let sum = &a + &b;
    let numerator = &sum * &sum;
    let denominator = &four * &t;
    let pi = (&numerator / &denominator)
        .with_precision(precision)
        .value();

    (pi, start.elapsed())
}
