use std::error::Error as StdError;
use std::fmt;

/// `simmer` が公開 API で返すエラー型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// [`MetricBuilder`](crate::MetricBuilder) の矢印モードが未指定です。
    ArrowModeNotSpecified,
    /// 矢印が無効な [`Metric`](crate::Metric) で方向変更が呼ばれました。
    ArrowDisabled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArrowModeNotSpecified => f.write_str(
                "arrow mode must be explicitly specified with .with_arrow(), \
                 .with_arrow_symbols(), or .no_arrow()",
            ),
            Self::ArrowDisabled => {
                f.write_str("cannot set arrow direction: this metric was built with .no_arrow()")
            }
        }
    }
}

impl StdError for Error {}

/// `simmer` の標準 Result 型。
pub type Result<T> = std::result::Result<T, Error>;
