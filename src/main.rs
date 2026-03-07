mod algorithm;
mod chudnovsky;
mod cli;
mod convert;
mod error;
mod gauss_legendre;
mod utils;

use algorithm::Algorithm;
use cli::{Stats, confirm_calculation, print_stats, prompt_algorithm, prompt_digits};
use colorgrad::Gradient;
use convert::convert_to_decimal_string;
use crust::{Metric, Shimmer, ShimmerConfig};
use error::{HyperPiError, Result};
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;
use sysinfo::System;
use utils::calculate_sha256;

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

const FILENAME: &str = "pi.txt";
const PRECISION_OFFSET: f64 = 128.0;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> miette::Result<()> {
    run().await.map_err(miette::Report::new)?;
    Ok(())
}

async fn run() -> Result<()> {
    // システム情報の初期化（将来的な利用のため）
    let mut sys = System::new_all();
    sys.refresh_memory();

    let version = env!("CARGO_PKG_VERSION");
    print_welcome(version);

    // ユーザー設定の取得
    let Some(digits) = prompt_digits()? else {
        return Ok(());
    };
    let Some(algorithm) = prompt_algorithm::<Algorithm>()? else {
        return Ok(());
    };
    if !confirm_calculation(digits)? {
        return Ok(());
    }

    // 精度の計算
    let precision = calculate_precision(digits);

    eprintln!("Now, calculate {} digits of Pi\n", digits);

    // 円周率の計算 (バイナリ)
    let (pi_bin, calc_time) = algorithm.execute(digits, precision).await?;

    // 10進数文字列への変換
    let digits_metric = Metric::builder()
        .suffix(" digits")
        .no_arrow()
        .build()
        .expect("no_arrow is always valid");
    digits_metric.set(digits as u64);
    let config = ShimmerConfig {
        time_display_threshold: Some(std::time::Duration::ZERO),
        metrics: vec![digits_metric],
        ..ShimmerConfig::default()
    };
    let shimmer = Shimmer::with_config("Converting to decimal...", config);
    shimmer.complete_status(&format!("Calculated {:.1}s", calc_time.as_secs_f64()));
    let (pi_str, conversion_time) =
        tokio::task::block_in_place(|| convert_to_decimal_string(&pi_bin, digits, precision));
    shimmer
        .stop(&format!(
            "Decimal conversion complete: {:.3}s",
            conversion_time.as_secs_f64()
        ))
        .await;

    // ファイルへの保存
    let start_io = Instant::now();
    save_to_file(FILENAME, &pi_str)?;
    let io_time = start_io.elapsed();
    eprintln!("✓ File writing completed: {:.3}s", io_time.as_secs_f64());

    // ハッシュ計算と検証
    let start_hash = Instant::now();
    let pi_hash = calculate_sha256(Path::new(FILENAME))?;
    let hash_time = start_hash.elapsed();

    eprintln!("\n✨ Completed Calculation! ✨\n");

    // 統計情報の表示
    print_stats(Stats {
        algorithm: algorithm.name().to_string(),
        total_time: calc_time + conversion_time + io_time + hash_time,
        calc_time,
        conversion_time,
        io_time,
        hash: pi_hash,
    });

    Ok(())
}

/// ウェルカムメッセージを表示する
fn print_welcome(version: &str) {
    eprintln!("{}", gradient_text(logo().to_string()).bold());
    eprintln!(
        "{}",
        format!("Welcome to HyperPi v{}\n", version)
            .color(hex_color!("#326095"))
            .bold()
    );
}

/// アプリケーションのロゴを返す
fn logo() -> &'static str {
    r#"
░█░█░█░█░█▀█░█▀▀░█▀▄░█▀█░▀█▀░
░█▀█░░█░░█▀▀░█▀▀░█▀▄░█▀▀░░█░░
░▀░▀░░▀░░▀░░░▀▀▀░▀░▀░▀░░░▀▀▀░
"#
}

/// 複数行のテキストに行ごとにグラデーション適用する
fn gradient_text(text: String) -> String {
    text.lines()
        .map(gradient_line)
        .collect::<Vec<String>>()
        .join("\n")
}

/// 1行のテキストにグラデーションを適用する
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

/// 必要なビット精度を計算する: 桁数 * log2(10) + 誤差補正
fn calculate_precision(digits: usize) -> usize {
    (digits as f64 * 10.0_f64.log2() + PRECISION_OFFSET) as usize
}

/// 結果をファイルに保存する
fn save_to_file(filename: &str, content: &str) -> Result<()> {
    let file = File::create(filename).map_err(|e| HyperPiError::FileWriteError {
        path: Path::new(filename).to_path_buf(),
        source: e,
    })?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", content).map_err(|e| HyperPiError::FileWriteError {
        path: Path::new(filename).to_path_buf(),
        source: e,
    })?;
    writer.flush().map_err(|e| HyperPiError::FileWriteError {
        path: Path::new(filename).to_path_buf(),
        source: e,
    })?;
    Ok(())
}
