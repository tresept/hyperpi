use crate::error::{HyperPiError, Result};
use colorgrad::Gradient;
use inquire::{Confirm, CustomType, InquireError, Select};
use owo_colors::OwoColorize;
use std::time::Duration;
use strum::IntoEnumIterator;

#[macro_export]
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

/// 1行のテキストにグラデーションを適用する
pub fn gradient_line(text: &str) -> String {
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

/// 複数行のテキストに行ごとにグラデーション適用する
pub fn gradient_text(text: String) -> String {
    text.lines()
        .map(gradient_line)
        .collect::<Vec<String>>()
        .join("\n")
}

/// アプリケーションのロゴを返す
pub fn logo() -> &'static str {
    r#"
░█░█░█░█░█▀█░█▀▀░█▀▄░█▀█░▀█▀░
░█▀█░░█░░█▀▀░█▀▀░█▀▄░█▀▀░░█░░
░▀░▀░░▀░░▀░░░▀▀▀░▀░▀░▀░░░▀▀▀░
"#
}

/// ウェルカムメッセージを表示する
pub fn print_welcome(version: &str) {
    eprintln!("{}", gradient_text(logo().to_string()).bold());
    eprintln!(
        "{}",
        format!("Welcome to HyperPi v{}\n", version)
            .color(crate::hex_color!("#326095"))
            .bold()
    );
}

/// 桁数の入力を促す
pub fn prompt_digits() -> Result<Option<usize>> {
    let result = CustomType::<usize>::new(
        &"Enter the number of decimal places to calculate"
            .truecolor(255, 246, 129)
            .to_string(),
    )
    .with_default(1_048_576)
    .prompt();

    match result {
        Ok(val) => Ok(Some(val)),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            eprintln!("\n{}", "Cancelled".bright_black().italic());
            Ok(None)
        }
        Err(err) => Err(HyperPiError::InputError(err.to_string())),
    }
}

/// アルゴリズムの選択を促す
pub fn prompt_algorithm<T: IntoEnumIterator + std::fmt::Display + PartialEq>() -> Result<Option<T>> {
    let prompt_msg = "Select the algorithm to calculate Pi"
        .truecolor(135, 206, 250)
        .to_string();

    let options = T::iter().collect::<Vec<_>>();

    match Select::new(&prompt_msg, options)
        .with_help_message("Pick one of the algorithms to calculate Pi...")
        .prompt()
    {
        Ok(v) => Ok(Some(v)),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            eprintln!("\n{}", "Cancelled".bright_black().italic());
            Ok(None)
        }
        Err(err) => Err(HyperPiError::InputError(err.to_string())),
    }
}

/// 最終確認を促す
pub fn confirm_calculation(digits: usize) -> Result<bool> {
    let result = Confirm::new(
        &format!("Calculate {} digits of Pi. Are you serious???", digits)
            .truecolor(220, 79, 109)
            .to_string(),
    )
    .with_default(true)
    .prompt();

    match result {
        Ok(true) => Ok(true),
        Ok(false) => {
            println!("{}", "The system was aborted".bright_black());
            Ok(false)
        }
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!("\n{}", "Operation Interrupted".bright_black().italic());
            Ok(false)
        }
        Err(err) => Err(HyperPiError::InputError(err.to_string())),
    }
}


/// 計算結果の統計情報を表示する
pub struct Stats {
    pub algorithm: String,
    pub total_time: Duration,
    pub calc_time: Duration,
    pub conversion_time: Duration,
    pub io_time: Duration,
    pub hash: String,
}

pub fn print_stats(stats: Stats) {
    let label_width = 22;

    eprintln!(
        "{:<width$} {}",
        "Algorithm:".bright_black(),
        stats.algorithm.cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {:.3} seconds",
        "Total time:".bright_black(),
        stats.total_time.as_secs_f64().cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {:.3} seconds",
        "Calculation time:".bright_black(),
        stats.calc_time.as_secs_f64().cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {:.3} seconds",
        "Conversion time:".bright_black(),
        stats.conversion_time.as_secs_f64().cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {:.3} seconds",
        "IO time:".bright_black(),
        stats.io_time.as_secs_f64().cyan(),
        width = label_width
    );
    eprintln!(
        "{:<width$} {}{}",
        "Result SHA-256 hash:".bright_black(),
        stats.hash.chars().take(16).collect::<String>().cyan(),
        "...".cyan(),
        width = label_width
    );
}

