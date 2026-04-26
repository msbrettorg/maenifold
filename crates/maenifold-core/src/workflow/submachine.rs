use crate::error::{MaenifoldError, Result};
use crate::markdown::Frontmatter;
use crate::sessions::workflow_session_uri;
use crate::thinking::sequential_session_uri;
use serde_yml::Value as YamlValue;

// T-12.0: RTM NFR-1 keeps submachine registration and waiting/resume transitions isolated from queue helpers.

pub(crate) enum ResumeSubmachineResult {
    Ready(Frontmatter),
    Waiting(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostSessionType {
    Workflow,
    Sequential,
}

impl HostSessionType {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Sequential => "sequential",
        }
    }

    pub(super) fn display_name(self) -> &'static str {
        match self {
            Self::Workflow => "Workflow",
            Self::Sequential => "SequentialThinking session",
        }
    }

    pub(super) fn link(self, session_id: &str) -> String {
        format!("[[{}{}{}]]", self.as_str(), "/", session_id)
    }

    pub(super) fn session_uri(self, session_id: &str) -> String {
        match self {
            Self::Workflow => workflow_session_uri(session_id),
            Self::Sequential => sequential_session_uri(session_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmachineType {
    Sequential,
    Workflow,
    Gate,
}

impl SubmachineType {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Workflow => "workflow",
            Self::Gate => "gate",
        }
    }

    pub(super) fn display_name(self) -> &'static str {
        match self {
            Self::Sequential => "SequentialThinking session",
            Self::Workflow => "Workflow session",
            Self::Gate => "gate task",
        }
    }

    pub(super) fn as_session_kind(self) -> Option<HostSessionType> {
        match self {
            Self::Sequential => Some(HostSessionType::Sequential),
            Self::Workflow => Some(HostSessionType::Workflow),
            Self::Gate => None,
        }
    }
}

pub(crate) fn parse_submachine_type(value: Option<&str>) -> Result<SubmachineType> {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("sequential")
        .to_ascii_lowercase();

    match normalized.as_str() {
        "sequential" => Ok(SubmachineType::Sequential),
        "workflow" => Ok(SubmachineType::Workflow),
        "gate" => Ok(SubmachineType::Gate),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "submachineType must be one of sequential, workflow, or gate. Got: {normalized}"
        ))),
    }
}

pub(crate) fn clear_submachine_frontmatter(frontmatter: &mut Frontmatter) {
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineTarget".to_string(),
        YamlValue::String(String::new()),
    );
}
