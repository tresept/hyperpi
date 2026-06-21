//! Terminal shimmer spinner used by HyperPi.

mod color;
mod config;
mod error;
mod metric;
mod render;
mod shimmer;

#[allow(unused_imports)]
pub use config::{PauseColor, ShimmerConfig, ShimmerMode};
#[allow(unused_imports)]
pub use error::{Error, Result};
#[allow(unused_imports)]
pub use metric::{ArrowMode, Metric, MetricBuilder};
pub use shimmer::Shimmer;
