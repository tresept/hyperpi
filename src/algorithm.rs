use crate::chudnovsky::{ChudnovskyProgress, calc_chudnovsky};
use crate::error::Result;
use crate::gauss_legendre::{GaussLegendreProgress, calc_gauss_legendre};
use crate::simmer::{Metric, Shimmer, ShimmerConfig};
use dashu::float::{FBig, round::mode::HalfEven};
use owo_colors::OwoColorize;
use std::time::Duration;
use strum::{Display, EnumIter};

type BinFloat = FBig<HalfEven, 2>;

#[derive(Debug, Display, EnumIter, PartialEq, Clone, Copy)]
pub enum Algorithm {
    #[strum(to_string = "Chudnovsky Algorithm (Recommended)")]
    Chudnovsky,
    #[strum(to_string = "Gauss-Legendre Algorithm")]
    GaussLegendre,
}

impl Algorithm {
    pub fn name(&self) -> &str {
        match self {
            Algorithm::Chudnovsky => "Chudnovsky Algorithm",
            Algorithm::GaussLegendre => "Gauss-Legendre Algorithm",
        }
    }

    pub async fn execute(&self, digits: usize, precision: usize) -> Result<(BinFloat, Duration)> {
        match self {
            Algorithm::Chudnovsky => self.run_chudnovsky(digits).await,
            Algorithm::GaussLegendre => self.run_gauss_legendre(precision).await,
        }
    }

    async fn run_chudnovsky(&self, digits: usize) -> Result<(BinFloat, Duration)> {
        eprintln!("Calculation method: {}", self.name().cyan());

        // digits / terms の dual メトリクス
        let confirmed = Metric::builder()
            .suffix(" digits")
            .with_arrow()
            .build()
            .expect("with_arrow is always valid");

        let terms = Metric::builder()
            .prefix("term ")
            .suffix(" processed")
            .no_arrow()
            .build()
            .expect("no_arrow is always valid");

        let config = ShimmerConfig {
            time_display_threshold: Some(Duration::ZERO),
            metrics: vec![confirmed.clone(), terms.clone()],
            ..ShimmerConfig::default()
        };
        let shimmer = Shimmer::with_config("Calculating...", config);
        shimmer.set_status("Chudnovsky");

        let result = tokio::task::block_in_place(|| {
            calc_chudnovsky(digits, |info: ChudnovskyProgress| {
                if info.message.is_none() {
                    confirmed.set(info.estimated_digits as u64);
                    terms.set(info.range.1 as u64);
                }
            })
        });

        let (pi, duration) = match result {
            Ok(res) => res,
            Err(e) => {
                drop(shimmer);
                return Err(e);
            }
        };

        shimmer
            .stop(&format!("Calculated: {:.3}s", duration.as_secs_f64()))
            .await;
        Ok((pi, duration))
    }

    async fn run_gauss_legendre(&self, precision: usize) -> Result<(BinFloat, Duration)> {
        eprintln!("Calculation method: {}", self.name().cyan());

        // 現在の反復 / 総反復 の dual メトリクス
        let iters = Metric::builder()
            .suffix(" iterations")
            .with_arrow()
            .dual()
            .build()
            .expect("with_arrow is always valid");
        let total = (((precision as f64).log2().ceil()) as u32).max(10) as u64;
        iters.set_secondary(total);

        let config = ShimmerConfig {
            time_display_threshold: Some(Duration::ZERO),
            metrics: vec![iters.clone()],
            ..ShimmerConfig::default()
        };
        let shimmer = Shimmer::with_config("Calculating...", config);
        shimmer.set_status("Gauss-Legendre");

        let result = tokio::task::block_in_place(|| {
            calc_gauss_legendre(precision, |info: GaussLegendreProgress| {
                iters.set(info.iteration as u64);
            })
        });

        let (pi, duration) = match result {
            Ok(res) => res,
            Err(e) => {
                drop(shimmer);
                return Err(e);
            }
        };

        shimmer
            .stop(&format!("Calculated: {:.3}s", duration.as_secs_f64()))
            .await;
        Ok((pi, duration))
    }
}
