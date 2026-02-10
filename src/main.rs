use colorgrad::Gradient;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Confirm, CustomType};
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{BufWriter, Error, ErrorKind, Write};
use std::time::{Duration, Instant};
use sysinfo::System;

#[allow(unused_imports)]
mod gauss_legendre;

mod chudnovsky;
use chudnovsky::{ChudnovskyProgress, calc_chudnovsky};

mod convert;
use convert::convert_to_decimal_string;

macro_rules! hex_color {
    ($hex:expr) => {{
        let h = $hex.trim_start_matches('#');
        owo_colors::Rgb(
            u8::from_str_radix(&h[0..2], 16).unwrap(),
            u8::from_str_radix(&h[2..4], 16).unwrap(),
            u8::from_str_radix(&h[4..6], 16).unwrap(),
        )
    }};
}

/// 1行のテキストにグラデーションを適用する関数
/// 虹色グラデーション（シアン → グリーン → イエロー → マゼンタ）
fn gradient_line(text: &str) -> String {
    let gradient = colorgrad::GradientBuilder::new()
        .colors(&[
            // colorgrad::Color::from_rgba8(0, 255, 255, 255), // シアン
            // colorgrad::Color::from_rgba8(143, 250, 171, 255), // グリーン
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
fn gradient_text(text: String) -> String {
    text.lines()
        .map(gradient_line)
        .collect::<Vec<String>>()
        .join("\n")
}

fn logo() -> &'static str {
    r#"
░█░█░█░█░█▀█░█▀▀░█▀▄░█▀█░▀█▀░
░█▀█░░█░░█▀▀░█▀▀░█▀▄░█▀▀░░█░░
░▀░▀░░▀░░▀░░░▀▀▀░▀░▀░▀░░░▀▀▀░
"#
}

fn main() -> std::io::Result<()> {
    let mut sys = System::new_all();

    sys.refresh_memory();

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    eprintln!("{}", gradient_text(logo().to_string()).bold());
    eprintln!(
        "{}",
        format!("  Welcome to HyperPi v{}\n", VERSION)
            .color(hex_color!("#87cefa"))
            .bold()
    );

    let total_memory = sys.total_memory();
    let available_memory = sys.available_memory();

    let digits: usize = CustomType::<usize>::new("計算する円周率の桁数を入力")
        .with_default(1_048_576)
        .with_help_message("小数点以下の桁数を符号なし整数で指定してください")
        .with_error_message("無効な値です")
        .prompt()
        .map_err(|_| Error::new(ErrorKind::Interrupted, "Cancelled"))?;

    let message = format!("{} 桁の円周率を計算します．よろしいですか？", digits)
        .cyan()
        .to_string();
    if !Confirm::new(&message)
        .with_default(true)
        .prompt()
        .map_err(|_| Error::new(ErrorKind::Interrupted, "Aborted"))?
    {
        println!("{}", "要求は中断されました".bright_black());
        return Ok(());
    }

    let filename = "pi.txt";

    // 必要なビット精度: 桁数 * log2(10) + 誤差補正
    let precision = (digits as f64 * 10.0_f64.log2() + 128.0) as usize;

    eprintln!("ただいまより {} 桁の円周率を計算します\n", digits);

    // 計算用スピナー
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(Duration::from_millis(200)); // 0.2秒おきに自動更新
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} [{elapsed_precise}] {msg}")
            .unwrap()
            .tick_chars("▖▗▘▙▚▛▜▝▞▟"),
    );

    // 円周率を計算（2進数）— Chudnovsky法
    let (pi_bin, calc_time) = calc_chudnovsky(digits, precision, |info: ChudnovskyProgress| {
        match info.message.as_deref() {
            Some("初期化") => {
                spinner.set_message("初期化中...");
            }
            Some("完了") => {
                spinner.set_style(
                    ProgressStyle::default_spinner()
                        .template("✓ [{elapsed_precise}] {msg}")
                        .unwrap(),
                );
                spinner
                    .finish_with_message(format!("計算完了: {:.3}s", info.elapsed.as_secs_f64()));
            }
            _ => {
                // Binary Splitting の進捗（leaf_done からのコールバック）
                spinner.set_message(format!(
                    "Chudnovsky法: 推定 {} 桁確定 -> 範囲 [{}, {}) を処理中",
                    info.estimated_digits,
                    info.range.0.map_or(0, |v| v),
                    info.range.1.map_or(0, |v| v),
                ));
            }
        }
    });
    eprintln!();

    // 10進数変換用スピナー
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(Duration::from_millis(200)); // 0.2秒おきに自動更新
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
            .tick_chars("▖▗▘▙▚▛▜▝▞▟"),
    );
    spinner.set_message("10進数への変換中...");

    let (pi_str, conversion_time) = convert_to_decimal_string(&pi_bin, digits, precision);

    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("✓ [{elapsed_precise}] {msg}")
            .unwrap(),
    );

    spinner.finish_with_message(format!(
        "10進数変換完了: {:.3}s",
        conversion_time.as_secs_f64()
    ));

    // ファイルに保存
    let start_io = Instant::now();
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", pi_str)?;
    let io_time = start_io.elapsed();
    eprintln!("✓ ファイル書き込み完了: {:.3}s", io_time.as_secs_f64());

    eprintln!(
        "\n✨ 合計 {:.3}s で完了しました ✨",
        (calc_time.as_secs_f64() + conversion_time.as_secs_f64() + io_time.as_secs_f64())
    );

    Ok(())
}
