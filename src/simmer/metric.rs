use super::error::{Error, Result};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

/// メトリクスに表示する矢印の動作モード。
///
/// [`MetricBuilder::with_arrow`]、[`MetricBuilder::with_arrow_symbols`]、
/// [`MetricBuilder::no_arrow`] のいずれかで設定します。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArrowMode {
    /// 矢印を表示しない。
    Disabled,
    /// 矢印を表示する。`up` / `down` に表示するシンボルを保持する。
    Enabled {
        /// 上向き（送信）を示すシンボル（例: `"↑"`）。
        up: String,
        /// 下向き（受信）を示すシンボル（例: `"↓"`）。
        down: String,
    },
}

/// リアルタイムに更新できる数値メトリクス。
///
/// [`Metric::builder`] で [`MetricBuilder`] を取得し、設定後に [`MetricBuilder::build`]
/// で生成します。`clone()` すると同じ値を共有するハンドルが得られるため、
/// [`crate::simmer::ShimmerConfig::metrics`] に渡しつつ手元でも更新できます。
///
/// # 表示フォーマット
///
/// | 設定 | 例 |
/// |------|-----|
/// | 矢印なし、単一値 | `6.8k tokens` |
/// | 矢印あり、単一値 | `↓ 6.8k tokens` |
/// | 矢印あり、dual | `↓ 86k / 772 tokens` |
///
/// 数値は自動的に `k` / `M` / `G` 単位に変換されます（例: 1200 → `1.2k`）。
///
/// # 例
///
/// ```no_run
/// use crate::simmer::Metric;
///
/// # fn main() -> crate::simmer::Result<()> {
/// let tokens = Metric::builder()
///     .suffix(" tokens")
///     .with_arrow()
///     .build()?;
///
/// let handle = tokens.clone();
///
/// tokens.set_direction_down()?;
/// tokens.add(1_500);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Metric {
    prefix: String,
    suffix: String,
    arrow: ArrowMode,
    dual: bool,
    primary: Arc<AtomicU64>,
    secondary: Arc<AtomicU64>,
    direction_up: Arc<AtomicBool>,
}

impl Metric {
    /// [`MetricBuilder`] を返します。
    #[must_use]
    pub fn builder() -> MetricBuilder {
        MetricBuilder::new()
    }

    /// primary 値を指定した値に設定します。
    pub fn set(&self, value: u64) {
        self.primary.store(value, Ordering::Relaxed);
    }

    /// primary 値に `delta` を加算します。
    pub fn add(&self, delta: u64) {
        self.primary.fetch_add(delta, Ordering::Relaxed);
    }

    /// secondary 値を指定した値に設定します（[dual モード][MetricBuilder::dual]用）。
    pub fn set_secondary(&self, value: u64) {
        self.secondary.store(value, Ordering::Relaxed);
    }

    /// secondary 値に `delta` を加算します（[dual モード][MetricBuilder::dual]用）。
    pub fn add_secondary(&self, delta: u64) {
        self.secondary.fetch_add(delta, Ordering::Relaxed);
    }

    /// 矢印を上向き（送信方向）に切り替えます。
    ///
    /// # Errors
    ///
    /// [`MetricBuilder::no_arrow`] で構築したメトリクスに対して呼び出すとエラーを返します。
    pub fn set_direction_up(&self) -> Result<()> {
        match self.arrow {
            ArrowMode::Disabled => Err(Error::ArrowDisabled),
            ArrowMode::Enabled { .. } => {
                self.direction_up.store(true, Ordering::Relaxed);
                Ok(())
            }
        }
    }

    /// 矢印を下向き（受信方向）に切り替えます。
    ///
    /// # Errors
    ///
    /// [`MetricBuilder::no_arrow`] で構築したメトリクスに対して呼び出すとエラーを返します。
    pub fn set_direction_down(&self) -> Result<()> {
        match self.arrow {
            ArrowMode::Disabled => Err(Error::ArrowDisabled),
            ArrowMode::Enabled { .. } => {
                self.direction_up.store(false, Ordering::Relaxed);
                Ok(())
            }
        }
    }

    pub(crate) fn has_value(&self) -> bool {
        self.primary.load(Ordering::Relaxed) > 0
            || (self.dual && self.secondary.load(Ordering::Relaxed) > 0)
    }

    pub(crate) fn format(&self) -> String {
        let primary = format_number(self.primary.load(Ordering::Relaxed));
        let value = if self.dual {
            let secondary = format_number(self.secondary.load(Ordering::Relaxed));
            format!("{primary} / {secondary}")
        } else {
            primary
        };

        match &self.arrow {
            ArrowMode::Disabled => format!("{}{}{}", self.prefix, value, self.suffix),
            ArrowMode::Enabled { up, down } => {
                let arrow = if self.direction_up.load(Ordering::Relaxed) {
                    up
                } else {
                    down
                };
                format!("{}{} {}{}", self.prefix, arrow, value, self.suffix)
            }
        }
    }
}

/// [`Metric`] を構築するビルダー。
///
/// [`Metric::builder`] または [`MetricBuilder::new`] で取得します。
///
/// # 必須設定
///
/// 矢印モードは必ず明示的に指定してください。未指定のまま [`MetricBuilder::build`]
/// を呼ぶとエラーになります。
///
/// | メソッド | 効果 |
/// |----------|------|
/// | [`MetricBuilder::with_arrow`] | デフォルト矢印（`↑` / `↓`）を有効化 |
/// | [`MetricBuilder::with_arrow_symbols`] | カスタム矢印シンボルを有効化 |
/// | [`MetricBuilder::no_arrow`] | 矢印を無効化 |
///
/// # 例
///
/// ```no_run
/// use crate::simmer::Metric;
///
/// # fn main() -> crate::simmer::Result<()> {
/// let io = Metric::builder()
///     .suffix(" bytes")
///     .with_arrow()
///     .dual()
///     .build()?;
///
/// let count = Metric::builder()
///     .prefix("ops: ")
///     .no_arrow()
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct MetricBuilder {
    prefix: String,
    suffix: String,
    arrow: ArrowConfig,
    dual: bool,
}

enum ArrowConfig {
    NotSet,
    Disabled,
    Enabled { up: String, down: String },
}

impl MetricBuilder {
    /// 新しい `MetricBuilder` を返します。
    #[must_use]
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
            suffix: String::new(),
            arrow: ArrowConfig::NotSet,
            dual: false,
        }
    }

    /// 数値の前に表示する静的テキストを設定します（デフォルト: `""`）。
    #[must_use]
    pub fn prefix(mut self, value: impl Into<String>) -> Self {
        self.prefix = value.into();
        self
    }

    /// 数値の後に表示する静的テキストを設定します（例: `" tokens"`）。
    #[must_use]
    pub fn suffix(mut self, value: impl Into<String>) -> Self {
        self.suffix = value.into();
        self
    }

    /// デフォルト矢印（`↑` / `↓`）を有効化します。
    ///
    /// 方向は [`Metric::set_direction_up`] / [`Metric::set_direction_down`] で切り替えます。
    #[must_use]
    pub fn with_arrow(mut self) -> Self {
        self.arrow = ArrowConfig::Enabled {
            up: "↑".into(),
            down: "↓".into(),
        };
        self
    }

    /// カスタムシンボルで矢印を有効化します。
    ///
    /// # 例
    ///
    /// ```no_run
    /// # use crate::simmer::Metric;
    /// # fn main() -> crate::simmer::Result<()> {
    /// let metric = Metric::builder()
    ///     .suffix(" B")
    ///     .with_arrow_symbols("⬆", "⬇")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn with_arrow_symbols(mut self, up: impl Into<String>, down: impl Into<String>) -> Self {
        self.arrow = ArrowConfig::Enabled {
            up: up.into(),
            down: down.into(),
        };
        self
    }

    /// 矢印を無効化します。
    ///
    /// このモードで構築したメトリクスに [`Metric::set_direction_up`] /
    /// [`Metric::set_direction_down`] を呼ぶとエラーになります。
    #[must_use]
    pub fn no_arrow(mut self) -> Self {
        self.arrow = ArrowConfig::Disabled;
        self
    }

    /// dual モードを有効化します。
    ///
    /// primary と secondary の両方の値を `primary / secondary` 形式で表示します。
    #[must_use]
    pub fn dual(mut self) -> Self {
        self.dual = true;
        self
    }

    /// [`Metric`] を構築して返します。
    ///
    /// # Errors
    ///
    /// 矢印モードが未指定の場合にエラーを返します。
    pub fn build(self) -> Result<Metric> {
        let arrow = match self.arrow {
            ArrowConfig::NotSet => return Err(Error::ArrowModeNotSpecified),
            ArrowConfig::Disabled => ArrowMode::Disabled,
            ArrowConfig::Enabled { up, down } => ArrowMode::Enabled { up, down },
        };

        Ok(Metric {
            prefix: self.prefix,
            suffix: self.suffix,
            arrow,
            dual: self.dual,
            primary: Arc::new(AtomicU64::new(0)),
            secondary: Arc::new(AtomicU64::new(0)),
            direction_up: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl Default for MetricBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn format_number(value: u64) -> String {
    if value < 1_000 {
        value.to_string()
    } else if value < 10_000 {
        format!("{:.1}k", value as f64 / 1_000.0)
    } else if value < 1_000_000 {
        format!("{}k", value / 1_000)
    } else if value < 10_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value < 1_000_000_000 {
        format!("{}M", value / 1_000_000)
    } else if value < 10_000_000_000 {
        format!("{:.1}G", value as f64 / 1_000_000_000.0)
    } else {
        format!("{}G", value / 1_000_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::format_number;

    #[test]
    fn formats_small_numbers_without_suffix() {
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn formats_thousands_with_decimal_when_needed() {
        assert_eq!(format_number(1_200), "1.2k");
    }

    #[test]
    fn formats_millions_without_decimal_when_large_enough() {
        assert_eq!(format_number(12_500_000), "12M");
    }
}
