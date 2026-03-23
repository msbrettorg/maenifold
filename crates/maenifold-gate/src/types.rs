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

/// T-53.2: RTM FR-41 — parsed stage metadata from an authoritative task file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageState {
    pub stage: String,
    pub heading_line: usize,
    pub verdict: Option<StageVerdict>,
}

/// T-53.2: RTM FR-41 — file-authoritative state derived from markdown + location.
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

    /// T-54.2: location-authoritative fallback for legacy done/ files.
    pub fn done_override(path: PathBuf) -> Self {
        Self::new(path, TaskLocation::Done, "done".to_string(), 0, vec![])
    }

    #[deprecated(note = "use parse_quietly_with_pipeline; this path uses SDD-hardcoded stage rules")]
    pub fn parse_quietly(
        path: &Path,
        content: &str,
    ) -> Result<Self, crate::parser::ParseTaskError> {
        crate::parser::parse_task_content(path, content)
    }

    pub fn parse_quietly_with_pipeline(
        path: &Path,
        content: &str,
        pipeline: &crate::pipeline::PipelineDef,
    ) -> Result<Self, crate::parser::ParseTaskError> {
        crate::parser::parse_task_content_with_pipeline(path, content, pipeline)
    }

    /// T-54.2: preserve parsed attempts/stages while enforcing done/ as terminal.
    pub fn with_done_override(mut self) -> Self {
        self.location = TaskLocation::Done;
        self.status = "done".to_string();
        self
    }

    /// T-54.2: recover partial stage data for legacy done/ files when only the
    /// location/status reconciliation failed. Delegates to the main parser then
    /// forces done-override on the result.
    pub fn done_override_from_content(path: &Path, content: &str) -> Self {
        match crate::parser::parse_task_content(path, content) {
            Ok(state) => state.with_done_override(),
            Err(crate::parser::ParseTaskError::LocationStatusMismatch { .. }) => {
                let todo_path = todo_shadow_path(path);
                match crate::parser::parse_task_content(&todo_path, content) {
                    Ok(state) => state.with_done_override(),
                    Err(_) => Self::done_override(path.to_path_buf()),
                }
            }
            Err(_) => Self::done_override(path.to_path_buf()),
        }
    }

    pub fn done_override_from_content_with_pipeline(
        path: &Path,
        content: &str,
        pipeline: &crate::pipeline::PipelineDef,
    ) -> Self {
        match crate::parser::parse_task_content_with_pipeline(path, content, pipeline) {
            Ok(state) => state.with_done_override(),
            Err(crate::parser::ParseTaskError::LocationStatusMismatch { .. }) => {
                let todo_path = todo_shadow_path(path);
                match crate::parser::parse_task_content_with_pipeline(&todo_path, content, pipeline)
                {
                    Ok(state) => state.with_done_override(),
                    Err(_) => Self::done_override(path.to_path_buf()),
                }
            }
            Err(_) => Self::done_override(path.to_path_buf()),
        }
    }

    /// T-54.2: effective status considering location-authoritative fallback.
    /// Files in done/ always resolve to "done"; files in todo/ use parsed status.
    pub fn effective_status(&self) -> &str {
        match self.location {
            TaskLocation::Done => "done",
            _ => &self.status,
        }
    }

    /// Returns true when the task status is terminal ("done").
    pub fn is_terminal(&self) -> bool {
        self.status == "done"
    }
}

fn todo_shadow_path(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(location_dir), Some(file_name)) => location_dir
            .parent()
            .unwrap_or(location_dir)
            .join("todo")
            .join(file_name),
        _ => path.to_path_buf(),
    }
}
