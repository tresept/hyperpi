use colorgrad::Gradient;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};

mod calculation;
use calculation::{calc_gauss_legendre, convert_to_decimal_string, GaussLegendreProgress};

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
fn gradient_text(text: String) -> String {
    text.lines()
        .map(gradient_line)
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
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    eprintln!("{}", gradient_text(logo()).bold());
    eprintln!(
        "{}",
        format!("  Welcome to HyperPi v{}\n", VERSION)
            .color(hex_color!("#87cefa"))
            .bold()
    );

    let digits = 1048576;

    let filename = "pi.txt";

    // 必要なビット精度: 桁数 * log2(10) + 誤差補正
    let precision = (digits as f64 * 10.0_f64.log2() + 128.0) as usize;

    eprintln!("ただいまより {} 桁の円周率を計算します\n", digits);

    // プログレスバー/スピナーの設定
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(Duration::from_millis(200)); // 0.2秒おきに自動更新
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} [{elapsed_precise}] {msg}")
            .unwrap()
            .tick_chars("▖▗▘▙▚▛▜▝▞▟"),
    );

    // 円周率を計算（2進数）
    let (pi_bin, calc_time) =
        calc_gauss_legendre(precision, |info: GaussLegendreProgress| match info.phase {
            "初期化" => {
                if let Some(msg) = info.message {
                    eprintln!("{}", msg);
                }
                spinner.set_message("初期値を計算中...");
            }
            "計算中" => {
                if let Some(msg) = &info.message {
                    spinner.set_message(msg.clone());
                } else {
                    let progress_pct =
                        (info.iteration as f64 / info.total_iterations as f64) * 100.0;
                    spinner.set_message(format!(
                        "Gauss-Legendre法: {}/{} 回 ({:.1}%)",
                        info.iteration, info.total_iterations, progress_pct
                    ));
                }
            }
            "完了" => {
                spinner.set_style(
                    ProgressStyle::default_spinner()
                        .template("✓ [{elapsed_precise}] {msg}")
                        .unwrap(),
                );
                spinner
                    .finish_with_message(format!("計算完了: {:.3}s", info.elapsed.as_secs_f64()));
            }
            _ => {}
        });
    eprintln!();

    // 10進数文字列に変換（スピナーなし、時間だけ表示）
    let (pi_str, conversion_time) = convert_to_decimal_string(&pi_bin, digits, precision);
    eprintln!("✓ 10進数変換完了: {:.3}s", conversion_time.as_secs_f64());

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
