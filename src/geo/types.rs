use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoMatchResult {
    pub matched: bool,
    pub confidence: f64,
    pub strategy: MatchStrategy,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchStrategy {
    PostalCode,
    BoundingBox,
    Locality,
    Region,
    None,
}

impl std::fmt::Display for MatchStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchStrategy::PostalCode => write!(f, "postal_code"),
            MatchStrategy::BoundingBox => write!(f, "bounding_box"),
            MatchStrategy::Locality => write!(f, "locality"),
            MatchStrategy::Region => write!(f, "region"),
            MatchStrategy::None => write!(f, "none"),
        }
    }
}

impl GeoMatchResult {
    pub fn no_match() -> Self {
        Self {
            matched: false,
            confidence: 0.0,
            strategy: MatchStrategy::None,
            details: String::new(),
        }
    }
}
