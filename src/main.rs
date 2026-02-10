use miette::{IntoDiagnostic, Result};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;
use sysinfo::System;

mod algorithm;
mod chudnovsky;
mod cli;
mod convert;
mod gauss_legendre;
mod simmer;
mod utils;

use algorithm::Algorithm;
use cli::{confirm_calculation, print_stats, print_welcome, prompt_algorithm, prompt_digits, Stats};
use convert::convert_to_decimal_string;
use simmer::{finish_shimmer, start_shimmer};
use utils::calculate_sha256;

const FILENAME: &str = "pi.txt";
const PRECISION_OFFSET: f64 = 128.0;

fn main() -> Result<()> {
    // システム情報の初期化（将来的な利用のため）
    let mut sys = System::new_all();
    sys.refresh_memory();

    let version = env!("CARGO_PKG_VERSION");
    print_welcome(version);

    // ユーザー設定の取得
    let Some(digits) = prompt_digits()? else { return Ok(()); };
    let Some(algorithm) = prompt_algorithm::<Algorithm>()? else { return Ok(()); };
    if !confirm_calculation(digits)? { return Ok(()); }

    // 精度の計算
    let precision = calculate_precision(digits);

    eprintln!("Now, calculate {} digits of Pi\n", digits);

    // 1. 円周率の計算 (バイナリ)
    let (pi_bin, calc_time) = algorithm.execute(digits, precision);
    eprintln!();

    // 2. 10進数文字列への変換
    let shimmer = start_shimmer("Converting to decimal string...".to_string());
    let (pi_str, conversion_time) = convert_to_decimal_string(&pi_bin, digits, precision);
    finish_shimmer(
        shimmer,
        format!("Decimal conversion complete: {:.3}s", conversion_time.as_secs_f64()),
    );

    // 3. ファイルへの保存
    let start_io = Instant::now();
    save_to_file(FILENAME, &pi_str)?;
    let io_time = start_io.elapsed();
    eprintln!("✓ File writing completed: {:.3}s", io_time.as_secs_f64());

    // 4. ハッシュ計算と検証
    let start_hash = Instant::now();
    let pi_hash = calculate_sha256(Path::new(FILENAME))?;
    let hash_time = start_hash.elapsed();

    eprintln!("\n✨ Completed Calculation! ✨\n");

    // 5. 統計情報の表示
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

/// 必要なビット精度を計算する: 桁数 * log2(10) + 誤差補正
fn calculate_precision(digits: usize) -> usize {
    (digits as f64 * 10.0_f64.log2() + PRECISION_OFFSET) as usize
}

/// 結果をファイルに保存する
fn save_to_file(filename: &str, content: &str) -> Result<()> {
    let file = File::create(filename).into_diagnostic()?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", content).into_diagnostic()?;
    writer.flush().into_diagnostic()?;
    Ok(())
}