use anyhow::{Result, anyhow};


pub struct DeltaEngine;

impl DeltaEngine {
    /// Computes a binary delta (patch) between base and target data.
    pub fn compute_delta(base: &[u8], target: &[u8]) -> Result<Vec<u8>> {
        let mut patch = Vec::new();
        qbsdiff::Bsdiff::new(base, target).compare(&mut patch).map_err(|e| anyhow!("Delta oluşturma hatası: {}", e))?;
        Ok(patch)
    }

    /// Applies a binary delta (patch) to base data to reconstruct the target data.
    pub fn apply_delta(base: &[u8], patch: &[u8]) -> Result<Vec<u8>> {
        let mut target = Vec::new();
        qbsdiff::Bspatch::new(patch).map_err(|e| anyhow!("Patch okuma hatası: {}", e))?
            .apply(base, &mut target).map_err(|e| anyhow!("Patch uygulama hatası: {}", e))?;
        Ok(target)
    }
}
