use miette::{IntoDiagnostic, Result};
use sha2::Digest;
use std::fs::File;
use std::io::{BufReader, copy};
use std::path::Path;

/// ファイルのSHA-256ハッシュを計算する
pub fn calculate_sha256(path: &Path) -> Result<String> {
    let file = File::open(path)
        .into_diagnostic()
        .map_err(|e| miette::miette!(format!("Failed to open file {}: {}", path.display(), e)))?;
    let mut reader = BufReader::new(file);
    let mut hasher = sha2::Sha256::new();
    
    copy(&mut reader, &mut hasher)
        .into_diagnostic()
        .map_err(|e| miette::miette!(format!("Error while hashing {}: {}", path.display(), e)))?;
    
    let result_hash = hasher.finalize();
    Ok(format!("{:x}", result_hash))
}
