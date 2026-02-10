use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{Write, stdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

/// スピナーのフレーム（Unicode Braille パターン）
const SPINNER_FRAMES: &[&str] = &["▖", "▘", "▝", "▗", "▙", "▛", "▜", "▟", "▚", "▞"];

/// シマーアニメーションの設定
struct ShimmerConfig {
    /// ベースカラー（暗め）
    base_color: Color,
    /// ハイライトカラー（明るめ）
    highlight_color: Color,
    /// スピナーカラー
    spinner_color: Color,
    /// ハイライト幅（文字数）
    highlight_width: usize,
    /// フレーム間隔
    frame_duration: Duration,
}

impl Default for ShimmerConfig {
    fn default() -> Self {
        Self {
            base_color: Color::Rgb {
                r: 100,
                g: 100,
                b: 180,
            },
            highlight_color: Color::Rgb {
                r: 220,
                g: 220,
                b: 255,
            },
            spinner_color: Color::Rgb {
                r: 150,
                g: 200,
                b: 255,
            },
            highlight_width: 4,
            frame_duration: Duration::from_millis(80),
        }
    }
}

/// 1フレーム描画する（スピナー + シマーテキスト + 経過時間）
fn render_frame(
    text: &str,
    shimmer_offset: usize,
    spinner_frame: usize,
    elapsed: Duration,
    config: &ShimmerConfig,
) -> std::io::Result<()> {
    let mut stdout = stdout();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    // 行をクリアしてカーソルを行頭へ
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::CurrentLine)
    )?;

    // スピナーアイコンを表示
    let spinner_icon = SPINNER_FRAMES[spinner_frame % SPINNER_FRAMES.len()];
    execute!(
        stdout,
        SetForegroundColor(config.spinner_color),
        Print(format!("{} ", spinner_icon))
    )?;

    // 各文字をハイライト範囲かどうかで色分けして出力
    if len == 0 {
        // 空文字なら何もしない（スピナーは表示済み）
    } else {
        for (i, &ch) in chars.iter().enumerate() {
            // shimmer_offset を中心に highlight_width の範囲をハイライト
            let dist = distance_on_ring(i, shimmer_offset, len, config.highlight_width);
            let color = if dist < config.highlight_width {
                // ハイライト中心に近いほど明るく（グラデーション）
                let t = 1.0 - (dist as f32 / config.highlight_width as f32);
                lerp_color(config.base_color, config.highlight_color, t)
            } else {
                config.base_color
            };
            execute!(stdout, SetForegroundColor(color), Print(ch))?;
        }
    }

    // 経過時間を表示（右側に追加）
    let total_secs = elapsed.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    let time_display = if hours > 0 {
        format!(" ({}h {}m {}s)", hours, minutes, secs)
    } else if minutes > 0 {
        format!(" ({}m {}s)", minutes, secs)
    } else {
        format!(" ({}s)", secs)
    };

    execute!(
        stdout,
        SetForegroundColor(Color::Rgb {
            r: 120,
            g: 120,
            b: 120
        }),
        Print(&time_display)
    )?;

    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    Ok(())
}

/// 環状距離（テキストをリングとして扱い、折り返しても最短距離を返す）
/// オーバーフロー対策済み
fn distance_on_ring(a: usize, b: usize, len: usize, highlight_width: usize) -> usize {
    if len == 0 {
        return 0;
    }

    // b を len + highlight_width でループさせる（範囲外アクセス防止）
    let effective_len = len + highlight_width;
    let b_wrapped = b % effective_len;

    // a と b_wrapped の距離を計算（符号なし整数のオーバーフロー対策）
    let forward = if b_wrapped >= a {
        b_wrapped - a
    } else {
        effective_len + b_wrapped - a
    };

    let backward = effective_len - forward;
    forward.min(backward)
}

/// 2色間を t (0.0〜1.0) で線形補間
fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    if let (
        Color::Rgb {
            r: r1,
            g: g1,
            b: b1,
        },
        Color::Rgb {
            r: r2,
            g: g2,
            b: b2,
        },
    ) = (from, to)
    {
        Color::Rgb {
            r: lerp_u8(r1, r2, t),
            g: lerp_u8(g1, g2, t),
            b: lerp_u8(b1, b2, t),
        }
    } else {
        to
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

/// start_shimmer:
/// - 受け取った初期テキストを表示しつつ、別スレッドでスピナー＋シマーを回す。
/// - 戻り値は `Sender<String>`。この Sender に新しい表示文字列を送ると表示が更新される。
/// - Sender を drop すると（あるいは全ての Sender が drop されると）スレッドは終了し、
///   カーソル表示の復帰と行消去を行います。
pub fn start_shimmer(initial_text: String) -> Sender<String> {
    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    // 最初のテキストを enqueue（受信側は最初の recv_timeout で取得）
    let initial = initial_text.clone();
    tx.send(initial_text).ok();

    // ここでスレッドを起動
    thread::spawn(move || {
        let config = ShimmerConfig::default();

        // カーソルを非表示に
        let _ = execute!(stdout(), cursor::Hide);

        let mut shimmer_offset: usize = 0;
        let mut spinner_frame: usize = 0;
        let mut last_text = initial;
        let start_time = Instant::now();

        loop {
            // 1フレーム描画
            let elapsed = start_time.elapsed();
            let _ = render_frame(&last_text, shimmer_offset, spinner_frame, elapsed, &config);
            shimmer_offset = shimmer_offset.wrapping_add(1);
            spinner_frame = spinner_frame.wrapping_add(1);

            // 次のメッセージが来るまで frame_duration を待つ
            match rx.recv_timeout(config.frame_duration) {
                Ok(msg) => {
                    // 受信したら表示テキストを更新し即座に次フレームへ
                    last_text = msg;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // タイムアウト: 次フレーム描画へループ
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // 送信側が全て drop された -> 終了
                    break;
                }
            }
        }

        // 後片付け: 行クリア・カーソル表示を戻す
        let mut stdout = stdout();
        let _ = execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            cursor::Show
        );
    });

    tx
}

/// finish_shimmer: シマーを停止して完了メッセージを表示
/// （Sender を drop してスレッドを終了させ、完了メッセージを出力）
pub fn finish_shimmer(shimmer: Sender<String>, message: String) {
    // Sender を drop してスレッドを停止
    drop(shimmer);

    // スレッドが終了するまで少し待つ（行クリアが完了するまで）
    thread::sleep(Duration::from_millis(50));

    // 完了メッセージを表示
    eprintln!("✓ {}", message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_on_ring() {
        // len=10, highlight_width=4 のリングで距離をテスト
        // a=0, b=3 の距離は 3
        assert_eq!(distance_on_ring(0, 3, 10, 4), 3);
        // a=0, b=12 の距離（折り返し考慮）
        let dist = distance_on_ring(0, 12, 10, 4);
        assert!(dist <= 7); // 最大でも effective_len/2 以下
    }

    #[test]
    fn test_distance_on_ring_no_overflow() {
        // 大きな値でもオーバーフローしないことを確認
        let dist = distance_on_ring(0, 1000, 10, 4);
        assert!(dist <= 7);
    }

    #[test]
    fn test_lerp_u8() {
        assert_eq!(lerp_u8(0, 100, 0.5), 50);
        assert_eq!(lerp_u8(0, 200, 1.0), 200);
        assert_eq!(lerp_u8(100, 200, 0.0), 100);
    }

    #[test]
    fn test_lerp_color() {
        let from = Color::Rgb { r: 0, g: 0, b: 0 };
        let to = Color::Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        let mid = lerp_color(from, to, 0.5);
        assert_eq!(
            mid,
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
    }
}
