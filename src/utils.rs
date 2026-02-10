use crate::error::{HyperPiError, Result};
use sha2::Digest;
use std::fs::File;
use std::io::{BufReader, copy};
use std::path::Path;

/// ファイルのSHA-256ハッシュを計算する
pub fn calculate_sha256(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(|e| HyperPiError::FileOpenError {
        path: path.to_path_buf(),
        source: e,
    })?;
    
    let mut reader = BufReader::new(file);
    let mut hasher = sha2::Sha256::new();
    
    copy(&mut reader, &mut hasher).map_err(|e| HyperPiError::HashCalculationError {
        path: path.to_path_buf(),
        source: e,
    })?;
    
    let result_hash = hasher.finalize();
    Ok(format!("{:x}", result_hash))
}

