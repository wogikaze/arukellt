//! Optimization level controlling which MIR passes are enabled.

/// Optimization level for the MIR pass pipeline.
///
/// Each variant enables a progressively larger set of optimization passes.
/// The gating table lives in `passes/mod.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum OptLevel {
    /// No MIR optimizations — debug builds and targets under migration.
    None,
    /// Safe, low-cost passes only (default).
    #[default]
    O1,
    /// All O1 passes plus heavier structural passes.
    O2,
    /// All O2 passes plus speculative / profile-guided passes.
    O3,
}

impl OptLevel {
    /// Convert a raw `u8` opt-level to `OptLevel`.
    ///
    /// `0` → `None`, `1` → `O1`, `2` → `O2`, `3` → `O3`.
    /// Returns an error string for any other value.
    pub fn from_u8(level: u8) -> Result<Self, String> {
        match level {
            0 => Ok(Self::None),
            1 => Ok(Self::O1),
            2 => Ok(Self::O2),
            3 => Ok(Self::O3),
            _ => Err(format!("invalid opt-level: {} (expected 0–3)", level)),
        }
    }

    /// Return `true` if this level is at least `other`.
    pub fn at_least(self, other: OptLevel) -> bool {
        self >= other
    }
}
