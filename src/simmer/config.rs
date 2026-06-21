use super::metric::Metric;
use crossterm::style::Color;
use tokio::time::Duration;

/// [`crate::Shimmer::pause`] を呼んだときに遷移する目標色。
#[derive(Clone, Copy, Debug)]
pub enum PauseColor {
    /// [`ShimmerConfig::base_color`] を黒方向に 55% 暗くした色を自動計算する。
    Auto,
    /// 手動で指定した色を使用する。
    Custom(Color),
}

/// シマーハイライトの移動パターン。
#[derive(Clone, Copy, Debug)]
pub enum ShimmerMode {
    /// 左から右への一方向。右端に達すると左端に戻り、2 秒間停止する。
    OneWay {
        /// ハイライトが 1 文字進む間隔。
        duration: Duration,
    },
    /// 往復。端に達するたびに 2 秒間停止する。
    Bounce {
        /// 右方向に進む間隔。
        forward_duration: Duration,
        /// 左方向に戻る間隔。
        backward_duration: Duration,
    },
}

/// [`crate::Shimmer`] の外観・動作を設定する構造体。
///
/// フィールドを個別に上書きして使うことを想定しています。
///
/// # 例
///
/// ```no_run
/// use crossterm::style::Color;
/// use crate::simmer::{ShimmerConfig, ShimmerMode};
/// use tokio::time::Duration;
///
/// let config = ShimmerConfig {
///     base_color: Color::Rgb { r: 80, g: 200, b: 100 },
///     spinner_color: Some(Color::Rgb { r: 80, g: 200, b: 100 }),
///     shimmer_mode: ShimmerMode::OneWay {
///         duration: Duration::from_millis(60),
///     },
///     ..ShimmerConfig::default()
/// };
/// ```
#[derive(Clone)]
pub struct ShimmerConfig {
    /// テキストのベースカラー。デフォルト: 明るい緑 `Rgb(80, 200, 100)`。
    pub base_color: Color,
    /// ハイライトカラー。`None` の場合は `base_color` から白方向へ自動計算する。
    pub highlight_color: Option<Color>,
    /// スピナーアイコンのカラー。`None` のとき `base_color` と同じ色を使う。
    pub spinner_color: Option<Color>,
    /// ハイライトが広がる幅（文字数）。デフォルト: `2`。
    pub highlight_width: usize,
    /// ハイライトの移動パターン。デフォルト: [`ShimmerMode::Bounce`]。
    pub shimmer_mode: ShimmerMode,
    /// スピナーアイコンが 1 コマ進む間隔。デフォルト: `250ms`。
    pub spinner_duration: Duration,
    /// 経過時間の表示を開始するしきい値。`None` なら常に表示。デフォルト: `Some(30s)`。
    pub time_display_threshold: Option<Duration>,
    /// `true` のとき、経過時間にセンチ秒を含める。デフォルト: `false`。
    pub show_centiseconds: bool,
    /// 分表示を開始するしきい値。`None` なら常に分を表示。デフォルト: `Some(60s)`。
    pub show_minutes_threshold: Option<Duration>,
    /// [`crate::Shimmer::pause`] 時の目標色。デフォルト: [`PauseColor::Auto`]。
    pub pause_color: PauseColor,
    /// サフィックス部に表示するメトリクス一覧。値が 0 の項目はスキップされる。
    pub metrics: Vec<Metric>,
}

impl Default for ShimmerConfig {
    fn default() -> Self {
        Self {
            base_color: Color::Rgb {
                r: 80,
                g: 200,
                b: 100,
            },
            highlight_color: None,
            spinner_color: None,
            highlight_width: 2,
            shimmer_mode: ShimmerMode::Bounce {
                forward_duration: Duration::from_millis(40),
                backward_duration: Duration::from_millis(80),
            },
            spinner_duration: Duration::from_millis(250),
            time_display_threshold: Some(Duration::from_secs(30)),
            show_centiseconds: false,
            show_minutes_threshold: Some(Duration::from_secs(60)),
            pause_color: PauseColor::Auto,
            metrics: Vec::new(),
        }
    }
}
