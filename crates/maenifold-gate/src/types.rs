use std::path::{Path, PathBuf};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum TaskLocation {
    Todo,
    Done,
}

impl TaskLocation {
    pub fn from_path(path: &Path) -> Option<Self> {
        path.parent()?.file_name()?.to_str()?.parse().ok()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString)]
#[strum(serialize_all = "UPPERCASE", ascii_case_insensitive)]
pub enum StageVerdict {
    Pass,
    Reject,
    Fail,
}

impl StageVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Reject => "REJECT",
            Self::Fail => "FAIL",
        }
    }

    pub fn counts_as_attempt(self) -> bool {
        matches!(self, Self::Reject | Self::Fail)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageState {
    pub stage: String,
    pub heading_line: usize,
    pub verdict: Option<StageVerdict>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskState {
    pub path: PathBuf,
    pub location: TaskLocation,
    pub status: String,
    pub attempts: u32,
    pub stages: Vec<StageState>,
}

impl TaskState {
    pub fn new(
        path: PathBuf,
        location: TaskLocation,
        status: String,
        attempts: u32,
        stages: Vec<StageState>,
    ) -> Self {
        Self {
            path,
            location,
            status,
            attempts,
            stages,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.status == "done"
    }
}
