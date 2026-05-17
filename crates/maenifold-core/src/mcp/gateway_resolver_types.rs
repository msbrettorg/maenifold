#![allow(dead_code)]

use crate::mcp::domain_mapping::SafetyClass;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

pub(crate) type ResolverResult<T> = std::result::Result<T, ResolverError>;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ResolverOutput {
    pub intent_summary: String,
    pub candidates: Vec<ResolverCandidate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ResolverCandidate {
    pub domain: String,
    pub command: String,
    pub confidence: f64,
    pub reason: String,
    pub safety_class: SafetyClass,
    pub requires_exact_execute: bool,
    pub parameters_draft: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolverError {
    Disabled { fallback: String },
    DirectInferenceSetup { fallback: String },
    Timeout { seconds: u64, fallback: String },
    Malformed { message: String, fallback: String },
    DirectInferenceFailed { message: String, fallback: String },
}

impl ResolverError {
    pub(crate) fn fallback(&self) -> &str {
        match self {
            Self::Disabled { fallback }
            | Self::DirectInferenceSetup { fallback }
            | Self::Timeout { fallback, .. }
            | Self::Malformed { fallback, .. }
            | Self::DirectInferenceFailed { fallback, .. } => fallback,
        }
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled { .. } => f.write_str("gateway intent resolver is disabled"),
            Self::DirectInferenceSetup { .. } => {
                f.write_str("gateway intent direct inference is not configured")
            }
            Self::Timeout { seconds, .. } => {
                write!(f, "gateway intent direct inference timed out after {seconds}s")
            }
            Self::Malformed { message, .. } => {
                write!(
                    f,
                    "gateway intent direct inference returned invalid output: {message}"
                )
            }
            Self::DirectInferenceFailed { message, .. } => {
                write!(f, "gateway intent direct inference failed: {message}")
            }
        }
    }
}

impl std::error::Error for ResolverError {}
