use inquire::{Confirm, CustomType, InquireError, Select};
use miette::{IntoDiagnostic, Result};
use owo_colors::OwoColorize;
use sha2::Digest;
use std::fs::File;
use std::io::{BufWriter, Write, copy};
use std::path::PathBuf;
use std::time::Instant;
use strum::{Display, EnumIter, IntoEnumIterator};
use sysinfo::System;

#[allow(unused_imports)]
mod gauss_legendre;
#[allow(unused_imports)]
use gauss_legendre::{GaussLegendreProgress, calc_gauss_legendre};

mod chudnovsky;
use chudnovsky::{ChudnovskyProgress, calc_chudnovsky};

mod convert;
use convert::convert_to_decimal_string;

mod cli;
use cli::{gradient_text, logo};

mod simmer;
use simmer::{finish_shimmer, start_shimmer};

/// ファイルのSHA-256ハッシュを計算する
fn check_hash(path: PathBuf) -> Result<String, miette::Error> {
    let file = File::open(&path)
        .into_diagnostic()
        .map_err(|e| miette::miette!(format!("Failed to open file {}: {}", path.display(), e)))?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = sha2::Sha256::new();
    // copy の結果は不要なので `_` にするが、エラーは伝播させる
    copy(&mut reader, &mut hasher)
        .into_diagnostic()
        .map_err(|e| miette::miette!(format!("Error while hashing {}: {}", path.display(), e)))?;
    let result_hash = hasher.finalize();
    Ok(format!("{:x}", result_hash))
}

#[derive(Debug, Display, EnumIter, PartialEq)]
enum Algorithm {
    #[strum(to_string = "Chudnovsky Algorithm (Recommended)")]
    Chudnovsky,
    #[strum(to_string = "Gauss-Legendre Algorithm")]
    GaussLegendre,
    // #[strum(to_string = "ボルウェインのアルゴリズム (高次収束)")]
    // Borwein,
}

fn main() -> miette::Result<()> {
    let mut sys = System::new_all();
    sys.refresh_memory();

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    // ロゴとウェルカムメッセージの表示
    eprintln!("{}", gradient_text(logo().to_string()).bold());
    eprintln!(
        "{}",
        format!("Welcome to HyperPi v{}\n", VERSION)
            .color(crate::hex_color!("#326095"))
            .bold()
    );

    // ユーザーに計算したい桁数を入力してもらう
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
            eprintln!("\n{}", "Cancelled".bright_black().italic());
            return Ok(());
        }
        Err(err) => return Err(miette::miette!(err)),
    };

    // アルゴリズムの選択
    let prompt_msg = "Select the algorithm to calculate Pi"
        .truecolor(135, 206, 250)
        .to_string();

    let options = Algorithm::iter().collect::<Vec<_>>();

    let target = match Select::new(&prompt_msg, options)
        .with_help_message("Pick one of the algorithms to calculate Pi...")
        .prompt()
    {
        Ok(v) => v,
        Err(_) => {
            eprintln!("\n{}", "Cancelled".bright_black().italic());
            return Ok(());
        }
    };

    // 実行の最終確認
    let confirm_result = Confirm::new(
        &format!("Calculate {} digits of Pi. Are you serious???", digits)
            .truecolor(220, 79, 109)
            .to_string(),
    )
    .with_default(true)
    .prompt();

    match confirm_result {
        Ok(true) => {}
        Ok(false) => {
            println!("{}", "The system was aborted".bright_black());
            return Ok(());
        }
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => {
            println!("\n{}", "Operation Interrupted".bright_black().italic());
            return Ok(());
        }
        Err(err) => return Err(miette::miette!(err)),
    }

    let filename = "pi.txt";

    // 必要なビット精度を計算: 桁数 * log2(10) + 誤差補正(128bit)
    let precision = (digits as f64 * 10.0_f64.log2() + 128.0) as usize;

    eprintln!("Now, calculate {} digits of Pi\n", digits);

    let algorithm_string = match target {
        Algorithm::Chudnovsky => "Chudnovsky Algorithm",
        Algorithm::GaussLegendre => "Gauss-Legendre Algorithm",
    };

    let (pi_bin, calc_time) = match target {
        Algorithm::Chudnovsky => {
            eprintln!("Calculation method: {}", "Chudnovsky Algorithm".cyan());
            // シマー統合スピナー開始
            let shimmer = start_shimmer("Initializing...".to_string());

            // pi_bin, calc_timeを返す
            let result = calc_chudnovsky(digits, precision, |info: ChudnovskyProgress| {
                match info.message.as_deref() {
                    Some("初期化") => {
                        shimmer.send("Initializing...".to_string()).ok();
                    }
                    Some("完了") => {
                        // 完了メッセージは後で finish_shimmer で表示
                    }
                    _ => {
                        let msg = format!(
                            "Chudnovsky: {} digits → range [{}, {})",
                            info.estimated_digits,
                            info.range.0.map_or(0, |v| v),
                            info.range.1.map_or(0, |v| v),
                        );
                        shimmer.send(msg).ok();
                    }
                }
            });

            // シマー停止＋完了メッセージ表示
            finish_shimmer(
                shimmer,
                format!("Calculated: {:.3}s", result.1.as_secs_f64()),
            );

            result
        }
        Algorithm::GaussLegendre => {
            eprintln!("Calculation method: {}", "Gauss-legendre Algorithm".cyan());
            let shimmer = start_shimmer("Calculating Pi using Gauss-Legendre...".to_string());

            // pi_bin, calc_timeを返す
            let result = calc_gauss_legendre(precision, |info: GaussLegendreProgress| {
                if info.iteration == info.total_iterations {
                    // 完了メッセージは後で finish_shimmer で表示
                } else {
                    let msg = format!(
                        "Gauss-Legendre: Iteration {}/{} - {}",
                        info.iteration, info.total_iterations, info.phase,
                    );
                    shimmer.send(msg).ok();
                }
            });

            // シマー停止＋完了メッセージ表示
            finish_shimmer(
                shimmer,
                format!("Calculated: {:.3}s", result.1.as_secs_f64()),
            );

            result
        }
    };
    eprintln!();

    // 10進数文字列への変換
    let shimmer = start_shimmer("Converting to decimal string...".to_string());

    let (pi_str, conversion_time) = convert_to_decimal_string(&pi_bin, digits, precision);

    finish_shimmer(
        shimmer,
        format!(
            "Decimal conversion complete: {:.3}s",
            conversion_time.as_secs_f64()
        ),
    );

    // 結果をファイルに保存
    let start_io = Instant::now();
    let file = File::create(filename).into_diagnostic()?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", pi_str).into_diagnostic()?;
    let io_time = start_io.elapsed();
    eprintln!("✓ File writing completed: {:.3}s", io_time.as_secs_f64());

    // ファイルのハッシュ値を計算して検証用に出力
    // ハッシュ計測は個別で表示しないが、処理時間は全体時間に加算する
    let start_hash = Instant::now();
    let pi_hash = check_hash(PathBuf::from(filename))?;
    let hash_time = start_hash.elapsed();

    // ここで完了メッセージをハッシュ計算後に表示（順序を正す）
    eprintln!("\n✨ Completed Calculation! ✨");
    eprintln!();

    let total_time = calc_time.as_secs_f64()
        + conversion_time.as_secs_f64()
        + io_time.as_secs_f64()
        + hash_time.as_secs_f64();

    let label_width = 22;

    // 結果の統計情報を表示
    eprintln!(
        "{:<width$} {}",
        "Algorithm:".bright_black(),
        algorithm_string.cyan(),
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
