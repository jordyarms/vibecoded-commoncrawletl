use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::atomic_write;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Checkpoint {
    /// Phase 1: domain analysis complete
    pub phase1_complete: bool,
    /// Phase 2: part prioritization complete
    pub phase2_complete: bool,
    /// Phase 3: per-part extraction tracking
    pub phase3_parts_completed: BTreeSet<u32>,
    /// Phase 3: fully complete
    pub phase3_complete: bool,
    /// Phase 4: geo-filtering complete
    pub phase4_complete: bool,
    /// Phase 5: domain scoring complete
    pub phase5_complete: bool,
    /// Phase 6: output generation complete
    pub phase6_complete: bool,
}

impl Checkpoint {
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let data = std::fs::read_to_string(path)?;
            let cp: Checkpoint = serde_json::from_str(&data)?;
            Ok(cp)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        atomic_write(path, json.as_bytes())?;
        Ok(())
    }

    pub fn mark_part_complete(&mut self, part: u32, path: &Path) -> Result<()> {
        self.phase3_parts_completed.insert(part);
        self.save(path)
    }

    pub fn is_part_complete(&self, part: u32) -> bool {
        self.phase3_parts_completed.contains(&part)
    }
}
