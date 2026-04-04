use chrono::{DateTime, Utc};
use serde::Serialize;

/// The four cognitive signal types in the Endophasia visualization.
/// Each maps to a specific tap point and RGBY color.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalType {
    /// extract_wikilinks tap — Blue #0078D4
    Writing,
    /// build_context tap — Green #107C10
    Thinking,
    /// find_similar_concepts tap — Yellow #FFB900
    Exploring,
    /// read_memory tap — Red #D83B01
    Remembering,
}

/// A single concept event emitted from a tap point.
#[derive(Debug, Clone, Serialize)]
pub struct ConceptEvent {
    pub signal: SignalType,
    pub concept: String,
    pub timestamp: DateTime<Utc>,
}

impl ConceptEvent {
    pub fn new(signal: SignalType, concept: String) -> Self {
        Self {
            signal,
            concept,
            timestamp: Utc::now(),
        }
    }
}
