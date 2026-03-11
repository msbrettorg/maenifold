use crate::error::{MaenifoldError, Result};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetKind {
    Workflow,
    Role,
    Color,
    Perspective,
}

impl AssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Role => "role",
            Self::Color => "color",
            Self::Perspective => "perspective",
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Workflow => "workflows",
            Self::Role => "roles",
            Self::Color => "colors",
            Self::Perspective => "perspectives",
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Workflow, Self::Role, Self::Color, Self::Perspective]
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "workflow" | "workflows" => Ok(Self::Workflow),
            "role" | "roles" => Ok(Self::Role),
            "color" | "colors" => Ok(Self::Color),
            "perspective" | "perspectives" => Ok(Self::Perspective),
            _ => Err(MaenifoldError::InvalidArgument(format!(
                "Unknown asset type '{value}'. Valid types: workflow, role, color, perspective"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetCatalog {
    #[serde(rename = "singleAgentWorkflows")]
    pub single_agent_workflows: Vec<Value>,
    #[serde(rename = "multiAgentWorkflows")]
    pub multi_agent_workflows: Vec<Value>,
    pub roles: Vec<Value>,
    pub colors: Vec<Value>,
    pub perspectives: Vec<Value>,
}
