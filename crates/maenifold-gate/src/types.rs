use std::path::{Path, PathBuf};

/// T-53.2: RTM FR-41 — canonical pipeline location for an authoritative task file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskLocation {
    Todo,
    Done,
}

impl TaskLocation {
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.parent()?.file_name()?.to_str()? {
            "todo" => Some(Self::Todo),
            "done" => Some(Self::Done),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Done => "done",
        }
    }
}

/// T-53.2: RTM FR-41 — canonical pipeline stage headings for [[gate]].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaskStage {
    Swe,
    Red,
    Blue,
}

impl TaskStage {
    pub const ALL: [Self; 3] = [Self::Swe, Self::Red, Self::Blue];

    pub fn canonical_heading(self) -> &'static str {
        match self {
            Self::Swe => "## SWE Remediation",
            Self::Red => "## Red-team Audit",
            Self::Blue => "## Blue-team Verification",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Swe => "swe",
            Self::Red => "red",
            Self::Blue => "blue",
        }
    }
}

/// T-53.2: RTM FR-41 — effective file-derived task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    Pending,
    Red,
    Blue,
    Done,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Done => "done",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done)
    }
}

/// T-53.2: RTM FR-41 — canonical review verdicts recorded in task files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// T-53.2: RTM FR-41 — parsed canonical stage metadata from an authoritative task file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageState {
    pub stage: TaskStage,
    pub heading_line: usize,
    pub verdict: Option<StageVerdict>,
}

/// T-53.2: RTM FR-41 — file-authoritative state derived from markdown + location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskState {
    pub path: PathBuf,
    pub location: TaskLocation,
    pub status: TaskStatus,
    pub attempts: u32,
    pub stages: Vec<StageState>,
}

impl TaskState {
    pub fn new(
        path: PathBuf,
        location: TaskLocation,
        status: TaskStatus,
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
}
