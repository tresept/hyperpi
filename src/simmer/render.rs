use super::color::{auto_highlight, lerp_color};
use super::config::ShimmerConfig;
use super::metric::Metric;
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{Write, stdout};
use std::time::Duration;

const SPINNER_FRAMES: &[&str] = &["▖", "▘", "▝", "▗", "▙", "▛", "▜", "▟", "▚", "▞"];

pub(crate) struct StatusDisplay<'a> {
    pub(crate) text: &'a str,
    pub(crate) done: bool,
}

pub(crate) fn render_frame(
    text: &str,
    status: Option<StatusDisplay<'_>>,
    metrics: &[Metric],
    shimmer_pos: usize,
    spinner_frame: usize,
    elapsed: Duration,
    animation_paused: bool,
    user_pause_progress: f32,
    pause_color: Color,
    spinner_color: Color,
    config: &ShimmerConfig,
) -> std::io::Result<()> {
    let mut stdout = stdout();
    let chars: Vec<char> = text.chars().collect();

    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::CurrentLine)
    )?;

    render_spinner(
        &mut stdout,
        spinner_frame,
        spinner_color,
        pause_color,
        user_pause_progress,
    )?;
    render_text(
        &mut stdout,
        &chars,
        shimmer_pos,
        animation_paused,
        user_pause_progress,
        pause_color,
        config,
    )?;
    render_suffix(&mut stdout, elapsed, metrics, status, config)?;

    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    Ok(())
}

fn render_spinner(
    stdout: &mut std::io::Stdout,
    spinner_frame: usize,
    spinner_color: Color,
    pause_color: Color,
    user_pause_progress: f32,
) -> std::io::Result<()> {
    let icon = SPINNER_FRAMES[spinner_frame % SPINNER_FRAMES.len()];
    let color = lerp_color(spinner_color, pause_color, user_pause_progress);
    execute!(stdout, SetForegroundColor(color), Print(format!("{icon} ")))
}

fn render_text(
    stdout: &mut std::io::Stdout,
    chars: &[char],
    shimmer_pos: usize,
    animation_paused: bool,
    user_pause_progress: f32,
    pause_color: Color,
    config: &ShimmerConfig,
) -> std::io::Result<()> {
    if chars.is_empty() {
        return Ok(());
    }

    let highlight = config
        .highlight_color
        .unwrap_or_else(|| auto_highlight(config.base_color));

    for (index, ch) in chars.iter().copied().enumerate() {
        let shimmer_color = if user_pause_progress > 0.0 || animation_paused {
            config.base_color
        } else {
            let distance = (index as isize - shimmer_pos as isize).unsigned_abs();
            if distance < config.highlight_width {
                let ratio = 1.0 - (distance as f32 / config.highlight_width as f32);
                lerp_color(config.base_color, highlight, ratio)
            } else {
                config.base_color
            }
        };
        let color = lerp_color(shimmer_color, pause_color, user_pause_progress);
        execute!(stdout, SetForegroundColor(color), Print(ch))?;
    }

    Ok(())
}

fn render_suffix(
    stdout: &mut std::io::Stdout,
    elapsed: Duration,
    metrics: &[Metric],
    status: Option<StatusDisplay<'_>>,
    config: &ShimmerConfig,
) -> std::io::Result<()> {
    let static_gray = Color::Rgb {
        r: 120,
        g: 120,
        b: 120,
    };
    let pulse_t = (elapsed.as_secs_f32() * std::f32::consts::PI).sin() * 0.5 + 0.5;
    let pulse_color = lerp_color(
        Color::Rgb {
            r: 100,
            g: 100,
            b: 100,
        },
        Color::Rgb {
            r: 140,
            g: 140,
            b: 140,
        },
        pulse_t,
    );

    let mut parts: Vec<(String, bool)> = Vec::new();

    if should_show_time(elapsed, config) {
        parts.push((format_elapsed(elapsed, config), false));
    }

    for metric in metrics {
        if metric.has_value() {
            parts.push((metric.format(), false));
        }
    }

    if let Some(status) = status {
        parts.push((status.text.to_string(), !status.done));
    }

    if parts.is_empty() {
        return Ok(());
    }

    execute!(stdout, SetForegroundColor(static_gray), Print(" ("))?;
    for (index, (text, pulsing)) in parts.iter().enumerate() {
        if index > 0 {
            execute!(stdout, SetForegroundColor(static_gray), Print(" ・ "))?;
        }
        let color = if *pulsing { pulse_color } else { static_gray };
        execute!(stdout, SetForegroundColor(color), Print(text))?;
    }
    execute!(stdout, SetForegroundColor(static_gray), Print(")"))?;

    Ok(())
}

fn should_show_time(elapsed: Duration, config: &ShimmerConfig) -> bool {
    match config.time_display_threshold {
        None => true,
        Some(threshold) => elapsed >= threshold,
    }
}

fn format_elapsed(elapsed: Duration, config: &ShimmerConfig) -> String {
    let total_secs = elapsed.as_secs();
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let show_minutes = match config.show_minutes_threshold {
        None => true,
        Some(threshold) => elapsed >= threshold,
    };

    match (show_minutes, config.show_centiseconds) {
        (true, true) => {
            let centiseconds = elapsed.subsec_millis() / 10;
            format!("{minutes}m {seconds}.{centiseconds:02}s")
        }
        (true, false) => format!("{minutes}m {seconds}s"),
        (false, true) => {
            let centiseconds = elapsed.subsec_millis() / 10;
            format!("{seconds}.{centiseconds:02}s")
        }
        (false, false) => format!("{seconds}s"),
    }
}
