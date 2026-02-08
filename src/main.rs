use colorgrad::Gradient;
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

mod calculation;
use calculation::{calculate_pi, convert_to_decimal_string};

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
    let (pi_str, _) = convert_to_decimal_string(&pi_bin, digits, precision);

    // ファイルに保存
    let start_io = Instant::now();
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "{}", pi_str)?;
    let io_time = start_io.elapsed();
    eprintln!(
        "ファイル書き込みを {:.3}s で完了しました",
        io_time.as_secs_f64()
    );

    eprintln!(
        "合計 {:.3}s で完了しました",
        (calc_time.as_secs_f64() + io_time.as_secs_f64())
    );

    Ok(())
}
