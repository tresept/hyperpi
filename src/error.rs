use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum HyperPiError {
    #[error("Failed to open file: {path}")]
    #[diagnostic(code(hyperpi::io::open_failed), help("Make sure the file exists and you have permission to access it."))]
    FileOpenError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write to file: {path}")]
    #[diagnostic(code(hyperpi::io::write_failed))]
    FileWriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Error occurred while calculating hash for: {path}")]
    #[diagnostic(code(hyperpi::io::hash_failed))]
    HashCalculationError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Operation was cancelled by the user")]
    #[diagnostic(code(hyperpi::cli::cancelled))]
    OperationCancelled,

    #[error("Input error: {0}")]
    #[diagnostic(code(hyperpi::cli::input_error))]
    InputError(String),

    #[error("Worker thread panicked")]
    #[diagnostic(code(hyperpi::runtime::panic))]
    ThreadPanic,
}

pub type Result<T> = std::result::Result<T, HyperPiError>;
