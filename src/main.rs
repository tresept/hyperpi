use colorgrad::Gradient;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Confirm, CustomType, InquireError};
use miette::{IntoDiagnostic, Result};
use owo_colors::OwoColorize;
use sha2::Digest;
use std::fs::File;
use std::io::{BufWriter, Write, copy};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use sysinfo::System;

#[allow(unused_imports)]
mod gauss_legendre;
#[allow(unused_imports)]
use gauss_legendre::{GaussLegendreProgress, calc_gauss_legendre};

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
            colorgrad::Color::from_rgba8(219, 79, 109, 255),
            colorgrad::Color::from_rgba8(255, 246, 129, 255),
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

fn check_hash(path: PathBuf) -> Result<String, miette::Error> {
    let file = File::open(&path)
        .into_diagnostic()
        .map_err(|e| miette::miette!(format!("Failed to open file {}: {}", path.display(), e)))?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = sha2::Sha256::new();
    let _ = copy(&mut reader, &mut hasher)
        .into_diagnostic()
        .map_err(|e| miette::miette!(format!("Error: {}", e)));
    let result_hash = hasher.finalize();
    Ok(format!("{:x}", result_hash))
}

fn main() -> miette::Result<()> {
    let mut sys = System::new_all();

    sys.refresh_memory();

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    eprintln!("{}", gradient_text(logo().to_string()).bold());
    eprintln!(
        "{}",
        format!("Welcome to HyperPi v{}\n", VERSION)
            .color(hex_color!("#326095"))
            .bold()
    );

    // 桁数入力ダイアログ
    let digits_result = CustomType::<usize>::new(
        &"Enter the number of decimal places to calculate"
            .truecolor(255, 246, 129)
            .to_string(),
    )
    .with_default(1_048_576)
    .prompt();

    let digits = match digits_result {
        Ok(val) => val,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            // Ctrl+C や Esc でのキャンセル時
            println!("\n{}", "Cancelled".bright_black().italic());
            return Ok(()); // 正常終了として扱うのがプロの嗜みらしい
        }
        Err(err) => return Err(miette::miette!(err)), // それ以外のの異常はmietteに任せる
    };

    // 実行確認ダイアログ
    let confirm_result = Confirm::new(
        &format!("Calculate {} digits of Pi. Are you serious???", digits)
            .truecolor(220, 79, 109)
            .to_string(),
    )
    .with_default(true)
    .prompt();

    match confirm_result {
        Ok(true) => {
            // はいを選んだので続行
        }
        Ok(false) => {
            // いいえを選んだ場合は中断
            println!("{}", "The system was aborted".bright_black());
            return Ok(());
        }
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => {
            // Ctrl+C や Esc で中断された場合
            println!("\n{}", "Operation Interrupted".bright_black().italic());
            return Ok(());
        }
        Err(err) => {
            // それ以外の予期せぬエラーは miette で
            return Err(miette::miette!(err));
        }
    }

    let filename = "pi.txt";

    // 必要なビット精度: 桁数 * log2(10) + 誤差補正
    let precision = (digits as f64 * 10.0_f64.log2() + 128.0) as usize;

    eprintln!("Now, calculate {} digits of Pi\n", digits);

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
                spinner.set_message("Initializing...");
            }
            Some("完了") => {
                spinner.set_style(
                    ProgressStyle::default_spinner()
                        .template("✓ [{elapsed_precise}] {msg}")
                        .unwrap(),
                );
                spinner
                    .finish_with_message(format!("Calculated: {:.3}s", info.elapsed.as_secs_f64()));
            }
            _ => {
                // Binary Splitting の進捗（leaf_done からのコールバック）
                spinner.set_message(format!(
                    "Chudnovsky Algorithm: Estimated {} digits confirmed -> Processing range [{}, {})",
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
    spinner.set_message("Converting to decimal string...");

    let (pi_str, conversion_time) = convert_to_decimal_string(&pi_bin, digits, precision);

    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("✓ [{elapsed_precise}] {msg}")
            .unwrap(),
    );

    spinner.finish_with_message(format!(
        "Decimal conversion complete: {:.3}s",
        conversion_time.as_secs_f64()
    ));

    // ファイルに保存
    let start_io = Instant::now();
    let file = File::create(filename).into_diagnostic()?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", pi_str).into_diagnostic()?;
    let io_time = start_io.elapsed();
    eprintln!("✓ File writing completed: {:.3}s", io_time.as_secs_f64());

    eprintln!("\n✨ Completed Calculation! ✨");
    eprintln!();

    // ハッシュ計算
    let pi_hash = check_hash(PathBuf::from(filename))?;

    let total_time =
        calc_time.as_secs_f64() + conversion_time.as_secs_f64() + io_time.as_secs_f64();

    let label_width = 22; // 一番長いラベルに合わせます

    // {:<width$} で左寄せの幅を動的に指定できます
    eprintln!(
        "{:<width$} {}",
        "Algorithm:".bright_black(),
        "Chudnovsky Algorithm".cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {:.3} seconds",
        "Total time:".bright_black(),
        total_time.cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {:.3} seconds",
        "Calculation time:".bright_black(),
        calc_time.as_secs_f64().cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {:.3} seconds",
        "Conversion time:".bright_black(),
        conversion_time.as_secs_f64().cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {:.3} seconds",
        "IO time:".bright_black(),
        io_time.as_secs_f64().cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {}{}",
        "Result SHA-256 hash:".bright_black(),
        pi_hash.chars().take(16).collect::<String>().cyan(),
        "...".cyan(),
        width = label_width
    );

    Ok(())
}
