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

    /// T-54.2: location-authoritative fallback for legacy done/ files.
    pub fn done_override(path: PathBuf) -> Self {
        Self::new(path, TaskLocation::Done, TaskStatus::Done, 0, vec![])
    }

    pub fn parse_quietly(
        path: &Path,
        content: &str,
    ) -> Result<Self, crate::parser::ParseTaskError> {
        quiet_parse_task_content(path, content)
    }

    /// T-54.2: preserve parsed attempts/stages while enforcing done/ as terminal.
    pub fn with_done_override(mut self) -> Self {
        self.location = TaskLocation::Done;
        self.status = TaskStatus::Done;
        self
    }

    /// T-54.2: recover partial stage data for legacy done/ files when only the
    /// location/status reconciliation failed.
    pub fn done_override_from_content(path: &Path, content: &str) -> Self {
        let lines: Vec<_> = content.lines().collect();
        let mut sections: Vec<FallbackSection> = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if let Some(stage) = TaskStage::ALL
                .into_iter()
                .find(|stage| line.trim().starts_with(stage.canonical_heading()))
            {
                if let Some(previous) = sections.last_mut() {
                    previous.end = index;
                }
                sections.push(FallbackSection {
                    stage,
                    heading_line: index + 1,
                    start: index + 1,
                    end: lines.len(),
                });
            }
        }
        let mut history = Vec::with_capacity(sections.len());
        let mut attempts = 0;
        for (index, section) in sections.iter().enumerate() {
            let has_later_same_stage = sections[index + 1..]
                .iter()
                .any(|other| other.stage == section.stage);
            let verdict = fallback_verdict(&lines, section, has_later_same_stage);
            if verdict.is_some_and(StageVerdict::counts_as_attempt) {
                attempts += 1;
            }
            history.push(StageState {
                stage: section.stage,
                heading_line: section.heading_line,
                verdict,
            });
        }
        let (status, stages) = fallback_status(history);
        Self::new(
            path.to_path_buf(),
            TaskLocation::Done,
            status,
            attempts,
            stages,
        )
        .with_done_override()
    }

    /// T-54.2: effective status considering location-authoritative fallback.
    /// Files in done/ always resolve to Done; files in todo/ use parsed status.
    pub fn effective_status(&self) -> TaskStatus {
        match self.location {
            TaskLocation::Done => TaskStatus::Done,
            _ => self.status,
        }
    }
}

#[derive(Clone, Copy)]
struct FallbackSection {
    stage: TaskStage,
    heading_line: usize,
    start: usize,
    end: usize,
}

fn fallback_verdict(
    lines: &[&str],
    section: &FallbackSection,
    has_later_same_stage: bool,
) -> Option<StageVerdict> {
    match section.stage {
        TaskStage::Swe => None,
        TaskStage::Red | TaskStage::Blue => lines[section.start..section.end]
            .iter()
            .find_map(|line| {
                line.trim()
                    .strip_prefix("**Verdict**:")
                    .and_then(parse_fallback_verdict)
            })
            .or(has_later_same_stage.then_some(StageVerdict::Reject)),
    }
}

fn parse_fallback_verdict(raw: &str) -> Option<StageVerdict> {
    match raw
        .trim()
        .split(|ch: char| !ch.is_alphanumeric())
        .find(|token| matches!(*token, "PASS" | "REJECT" | "FAIL"))
    {
        Some("PASS") => Some(StageVerdict::Pass),
        Some("REJECT") => Some(StageVerdict::Reject),
        Some("FAIL") => Some(StageVerdict::Fail),
        _ => None,
    }
}

fn fallback_status(history: Vec<StageState>) -> (TaskStatus, Vec<StageState>) {
    if history.is_empty() {
        return (TaskStatus::Pending, vec![]);
    }
    let swe = history
        .iter()
        .rfind(|stage| stage.stage == TaskStage::Swe)
        .cloned();
    let Some(swe) = swe else {
        return (TaskStatus::Pending, vec![]);
    };
    let red = history
        .iter()
        .rfind(|stage| stage.stage == TaskStage::Red && stage.heading_line > swe.heading_line)
        .cloned();
    let blue = red
        .as_ref()
        .filter(|stage| stage.verdict == Some(StageVerdict::Pass))
        .and_then(|red| {
            history
                .iter()
                .rfind(|stage| {
                    stage.stage == TaskStage::Blue && stage.heading_line > red.heading_line
                })
                .cloned()
        });
    let stages = [Some(swe.clone()), red.clone(), blue.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let status = match (
        red.as_ref().and_then(|stage| stage.verdict),
        blue.as_ref().and_then(|stage| stage.verdict),
    ) {
        (None, None) => TaskStatus::Red,
        (Some(StageVerdict::Pass), None) => TaskStatus::Blue,
        (Some(StageVerdict::Pass), Some(StageVerdict::Pass)) => TaskStatus::Done,
        _ => TaskStatus::Pending,
    };
    (status, stages)
}

fn quiet_parse_task_content(
    path: &Path,
    content: &str,
) -> Result<TaskState, crate::parser::ParseTaskError> {
    #[cfg(unix)]
    unsafe {
        use std::{fs::File, os::fd::AsRawFd};
        unsafe extern "C" {
            fn close(fd: i32) -> i32;
            fn dup(fd: i32) -> i32;
            fn dup2(src: i32, dst: i32) -> i32;
        }
        let stderr_fd = std::io::stderr().as_raw_fd();
        let saved = dup(stderr_fd);
        if saved >= 0 {
            if let Ok(devnull) = File::options().write(true).open("/dev/null") {
                if dup2(devnull.as_raw_fd(), stderr_fd) >= 0 {
                    let parsed = crate::parser::parse_task_content(path, content);
                    let _ = dup2(saved, stderr_fd);
                    let _ = close(saved);
                    return parsed;
                }
            }
            let _ = close(saved);
        }
    }
    crate::parser::parse_task_content(path, content)
}
