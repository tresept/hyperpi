use crate::chudnovsky::{calc_chudnovsky, ChudnovskyProgress};
use crate::gauss_legendre::{calc_gauss_legendre, GaussLegendreProgress};
use crate::simmer::{finish_shimmer, start_shimmer};
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

    pub fn execute(&self, digits: usize, precision: usize) -> (BinFloat, Duration) {
        match self {
            Algorithm::Chudnovsky => self.run_chudnovsky(digits, precision),
            Algorithm::GaussLegendre => self.run_gauss_legendre(precision),
        }
    }

    fn run_chudnovsky(&self, digits: usize, precision: usize) -> (BinFloat, Duration) {
        eprintln!("Calculation method: {}", self.name().cyan());
        let shimmer = start_shimmer("Initializing...".to_string());

        let result = calc_chudnovsky(digits, precision, |info: ChudnovskyProgress| {
            match info.message.as_deref() {
                Some("Initializing...") | Some("初期化") => {
                    shimmer.send("Initializing...".to_string()).ok();
                }
                Some("Completed") | Some("完了") => {}
                _ => {
                    let msg = format!(
                        "Chudnovsky: {} digits → range [{}, {})",
                        info.estimated_digits,
                        info.range.0.unwrap_or(0),
                        info.range.1.unwrap_or(0),
                    );
                    shimmer.send(msg).ok();
                }
            }
        });

        finish_shimmer(
            shimmer,
            format!("Calculated: {:.3}s", result.1.as_secs_f64()),
        );

        result
    }

    fn run_gauss_legendre(&self, precision: usize) -> (BinFloat, Duration) {
        eprintln!("Calculation method: {}", self.name().cyan());
        let shimmer = start_shimmer("Calculating Pi using Gauss-Legendre...".to_string());

        let result = calc_gauss_legendre(precision, |info: GaussLegendreProgress| {
            if info.iteration != info.total_iterations {
                let msg = format!(
                    "Gauss-Legendre: Iteration {}/{} - {}",
                    info.iteration, info.total_iterations, info.phase,
                );
                shimmer.send(msg).ok();
            }
        });

        finish_shimmer(
            shimmer,
            format!("Calculated: {:.3}s", result.1.as_secs_f64()),
        );

        result
    }
}
