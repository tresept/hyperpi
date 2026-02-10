use colorgrad::Gradient;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::time::Duration;

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
        .join("
")
}

/// アプリケーションのロゴを返す
pub fn logo() -> &'static str {
    r#"
░█░█░█░█░█▀█░█▀▀░█▀▄░█▀█░▀█▀░
░█▀█░░█░░█▀▀░█▀▀░█▀▄░█▀▀░░█░░
░▀░▀░░▀░░▀░░░▀▀▀░▀░▀░▀░░░▀▀▀░
"#
}

/// 計算用の標準的なスピナーを作成する
pub fn create_spinner(message: &str, color_cyan: bool) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(Duration::from_millis(200));
    
    let template = if color_cyan {
        "{spinner:.cyan} [{elapsed_precise}] {msg}"
    } else {
        "{spinner:.green} [{elapsed_precise}] {msg}"
    };

    spinner.set_style(
        ProgressStyle::default_spinner()
            .template(template)
            .unwrap()
            .tick_chars("▖▗▘▙▚▛▜▝▞▟"),
    );
    spinner.set_message(message.to_string());
    spinner
}

/// スピナーを完了状態にする
pub fn finish_spinner(spinner: &ProgressBar, message: String) {
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("✓ [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    spinner.finish_with_message(message);
}
