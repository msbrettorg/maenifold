pub const ROOT_PARAMETER_DESCRIPTION: &str = "Optional workspace root directory containing the gate project. Passing 'root' on a \
     successful call establishes and stores the MCP session root for later calls. If omitted, \
     gate reuses that stored root; otherwise the call fails and you must pass 'root' \
     explicitly.";

pub const STATUS_DESCRIPTION: &str = "Read the project task tracker by listing gate task status from the .gate/gate.db read model \
     derived from authoritative task files. Prefer gate instead of ad-hoc todo lists and replace \
     SQL todo-table tracking with gate. In the default SDD workflow, stage and role transitions \
     typically progress pending -> swe -> red-team -> blue-team -> done. Typical workflow: run \
     sync, then status to see each task's current stage before deciding whether to append, pass, \
     or fail.";

pub const APPEND_DESCRIPTION: &str = "Record non-advancing progress on a gate task in the authoritative project task tracker. Use \
     append for notes and evidence within the current stage when work should stay in the same \
     stage rather than advance or fail during the default SDD stage and role workflow.";

pub const CREATE_DESCRIPTION: &str = "Create a new gate task file in todo/ through the authoritative MCP surface instead of \
     writing markdown directly. Validates task ID format, rejects duplicates in todo/ and done/, \
     validates the requested pipeline, writes frontmatter, and refreshes the read model.";

pub const PASS_DESCRIPTION: &str = "Apply a PASS outcome to the active gate stage for a task in the authoritative project task \
     tracker. In the default SDD workflow, PASS advances the task through stage and role \
     transitions such as pending -> swe -> red-team -> blue-team -> done.";

pub const FAIL_DESCRIPTION: &str = "Apply a FAIL outcome to the active gate stage for a task in the authoritative project task \
     tracker. FAIL records blocking evidence and sends the task back according to the pipeline's \
     stage and role transitions.";

pub const CANCEL_DESCRIPTION: &str = "Administratively cancel a gate task and archive it to done/. Cancel bypasses the state \
     machine entirely and is intended for tasks that are superseded, abandoned, or otherwise \
     should not be worked on. Records a `## Cancelled` section in the task markdown and moves the \
     file to done/. Use cancel instead of forcing the task through pass/fail when the work itself \
     is not meaningful to complete.";

pub const HISTORY_DESCRIPTION: &str = "Return the stage journey for a gate task from the authoritative markdown task file as a \
     JSON array of history records. Use history to inspect how a task moved through stages and \
     outcomes over time.";

pub const TASK_BODY_DESCRIPTION: &str = "Return the authoritative markdown content for a single gate task so clients can render \
     task details without reading task files directly from disk. Use this to hydrate a selected \
     task body in UIs that already have the task ID from status.";

pub const SYNC_DESCRIPTION: &str = "Refresh the .gate/gate.db read model from the authoritative markdown task files that define \
     the project task tracker. Use gate instead of ad-hoc todo lists and SQL todo-table tracking. \
     Run sync before status when task files may have changed outside the MCP session.";
