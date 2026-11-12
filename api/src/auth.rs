use anyhow::{ensure, Result};
use sha2::{Digest, Sha256};
use std::path::Path;
use subtle::ConstantTimeEq;
pub fn digest(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
pub fn verify(path: &Path, token: &str) -> Result<Option<String>> {
    ensure!(
        token.len() >= 32 && token.len() <= 256,
        "invalid token length"
    );
    let bytes = std::fs::read(path)?;
    ensure!(bytes.len() <= 65536, "token file too large");
    let hashes: std::collections::BTreeMap<String, String> = serde_json::from_slice(&bytes)?;
    let hash = Sha256::digest(token.as_bytes());
    let mut found = None;
    for (id, h) in hashes {
        let expected = hex::decode(h)?;
        ensure!(expected.len() == 32, "invalid token verifier");
        if expected.as_slice().ct_eq(hash.as_slice()).into() {
            found = Some(id);
        }
    }
    Ok(found)
}
